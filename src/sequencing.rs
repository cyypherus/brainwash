use crate::Signal;

pub struct Chord {
    notes: Vec<i32>,
}

pub enum SequenceElement {
    Chord(Chord),
    Subdivision(Vec<SequenceElement>),
}

pub struct Scale {
    notes: Vec<i32>,
}

impl Scale {
    pub fn shift(&self, i: i32) -> Scale {
        Scale {
            notes: self.notes.iter().map(|n| n + i).collect(),
        }
    }
}

pub fn cmaj() -> Scale {
    Scale {
        notes: vec![0, 2, 4, 5, 7, 9, 11],
    }
}

pub fn cmin() -> Scale {
    Scale {
        notes: vec![0, 2, 3, 5, 7, 8, 10],
    }
}

impl Scale {
    pub fn note(&self, index: i32) -> i32 {
        if self.notes.is_empty() {
            return 0;
        }

        let scale_len = self.notes.len() as i32;
        let octave_offset = if index < 0 {
            ((index + 1) / scale_len) - 1
        } else {
            index / scale_len
        };
        let wrapped_index = ((index % scale_len) + scale_len) % scale_len;
        self.notes[wrapped_index as usize] + octave_offset * 12
    }
}

pub(crate) struct SequenceState {
    pub(crate) all_notes: Vec<i32>,
    pub(crate) last_chord_index: usize,
    pub(crate) active_notes: std::collections::HashSet<i32>,
    pub(crate) previous_notes: std::collections::HashSet<i32>,
    pub(crate) current_bar: usize,
    pub(crate) last_clock_position: f32,
}

pub struct Sequence {
    id: usize,
    elements: Vec<SequenceElement>,
    bars: f32,
}

pub fn chord(notes: impl Into<Vec<i32>>) -> SequenceElement {
    SequenceElement::Chord(Chord {
        notes: notes.into(),
    })
}

pub fn note(note: i32) -> SequenceElement {
    chord([note])
}

pub fn rest() -> SequenceElement {
    chord([])
}

pub fn seq(id: usize, elements: impl Into<Vec<SequenceElement>>) -> Sequence {
    Sequence {
        id,
        elements: elements.into(),
        bars: 1.,
    }
}

pub fn sub(elements: impl Into<Vec<SequenceElement>>) -> SequenceElement {
    SequenceElement::Subdivision(elements.into())
}

impl Sequence {
    pub fn bars(mut self, bars: f32) -> Self {
        self.bars = bars;
        self
    }

    fn ensure_state(&self, state: &mut SequenceState) {
        if state.all_notes.is_empty() {
            state.all_notes = self.get_all_notes();
            state.last_chord_index = usize::MAX;
            state.current_bar = 0;
            state.last_clock_position = 0.0;
        }
    }

    pub fn output(&mut self, clock_position: f32, signal: &mut Signal) -> Vec<Key> {
        if self.elements.is_empty() {
            return Vec::new();
        }

        let state = signal.get_sequence_state(self.id as i32, 0);

        self.ensure_state(state);

        if clock_position < state.last_clock_position {
            state.current_bar = ((state.current_bar + 1) as f32 % self.bars) as usize;
        }
        state.last_clock_position = clock_position;

        let sequence_position = (state.current_bar as f32 + clock_position) / self.bars;

        let (chord_index, active_chord) =
            Self::find_active_chord(&self.elements, sequence_position);

        let chord_changed = chord_index != state.last_chord_index;

        if chord_changed {
            state.previous_notes = state.active_notes.clone();
            state.active_notes.clear();
            if let Some(chord) = active_chord {
                state.active_notes.extend(chord.notes.iter().cloned());
            }
            state.last_chord_index = chord_index;
        }

        let mut keys = Vec::with_capacity(state.all_notes.len());
        let mut on_index = 0;
        for (index, &note) in state.all_notes.iter().enumerate() {
            let was_on = state.previous_notes.contains(&note);
            let is_on = state.active_notes.contains(&note);

            if is_on {
                on_index += 1;
            }

            keys.push(Key {
                on: if chord_changed && was_on && is_on {
                    false
                } else {
                    is_on
                },
                note,
                index,
                on_index,
                pitch: note as f32,
            });
        }
        keys
    }

    fn find_active_chord(elements: &[SequenceElement], position: f32) -> (usize, Option<&Chord>) {
        if elements.is_empty() {
            return (0, None);
        }

        let element_index = ((position * elements.len() as f32) as usize).min(elements.len() - 1);
        let element_position = (position * elements.len() as f32) % 1.0;

        match &elements[element_index] {
            SequenceElement::Chord(chord) => (element_index, Some(chord)),
            SequenceElement::Subdivision(sub_elements) => {
                let (sub_index, chord) = Self::find_active_chord(sub_elements, element_position);
                (element_index * 1000 + sub_index, chord)
            }
        }
    }

    fn get_all_notes(&self) -> Vec<i32> {
        let mut all_notes = std::collections::HashSet::new();
        Self::collect_notes_from_elements(&self.elements, &mut all_notes);
        let mut notes: Vec<i32> = all_notes.into_iter().collect();
        notes.sort();
        notes
    }

    fn collect_notes_from_elements(
        elements: &[SequenceElement],
        all_notes: &mut std::collections::HashSet<i32>,
    ) {
        for element in elements {
            match element {
                SequenceElement::Chord(chord) => {
                    for &note in &chord.notes {
                        all_notes.insert(note);
                    }
                }
                SequenceElement::Subdivision(sub_elements) => {
                    Self::collect_notes_from_elements(sub_elements, all_notes);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub on: bool,
    pub index: usize,
    pub on_index: usize,
    pub note: i32,
    pub pitch: f32,
}

impl Chord {
    pub fn output(&self) -> Vec<f32> {
        self.notes.iter().map(|&n| n as f32).collect()
    }
}

impl From<Chord> for SequenceElement {
    fn from(chord: Chord) -> Self {
        SequenceElement::Chord(chord)
    }
}

impl From<Vec<SequenceElement>> for SequenceElement {
    fn from(elements: Vec<SequenceElement>) -> Self {
        SequenceElement::Subdivision(elements)
    }
}
