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
        let note = self.notes[wrapped_index as usize] + octave_offset * 12;
        note
    }
}

pub(crate) struct SequenceState {
    pub(crate) all_notes: Vec<i32>,
    pub(crate) last_chord_index: usize,
    pub(crate) active_notes: std::collections::HashSet<i32>,
    pub(crate) previous_notes: std::collections::HashSet<i32>,
    pub(crate) params_hash: u64,
    pub(crate) current_bar: usize,
    pub(crate) last_clock_position: f32,
}

pub struct Sequence {
    id: usize,
    elements: Vec<SequenceElement>,
    bars: usize,
}

pub fn chord(notes: &[i32]) -> SequenceElement {
    SequenceElement::Chord(Chord {
        notes: notes.to_vec(),
    })
}

pub fn note(note: i32) -> SequenceElement {
    chord(&[note])
}

pub fn rest() -> SequenceElement {
    chord(&[])
}

pub fn sequence<T: Into<Vec<SequenceElement>>>(id: usize, elements: T) -> Sequence {
    Sequence {
        id,
        elements: elements.into(),
        bars: 1,
    }
}

pub fn sub<T: Into<Vec<SequenceElement>>>(elements: T) -> SequenceElement {
    SequenceElement::Subdivision(elements.into())
}

impl Sequence {
    pub fn bars(mut self, bars: usize) -> Self {
        self.bars = bars;
        self
    }

    fn hash_params(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.elements.len().hash(&mut hasher);
        self.hash_elements(&self.elements, &mut hasher);
        self.bars.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_elements(
        &self,
        elements: &[SequenceElement],
        hasher: &mut std::collections::hash_map::DefaultHasher,
    ) {
        use std::hash::Hash;

        for element in elements {
            match element {
                SequenceElement::Chord(chord) => {
                    0u8.hash(hasher);
                    chord.notes.len().hash(hasher);
                    for &note in &chord.notes {
                        note.hash(hasher);
                    }
                }
                SequenceElement::Subdivision(sub_elements) => {
                    1u8.hash(hasher);
                    sub_elements.len().hash(hasher);
                    self.hash_elements(sub_elements, hasher);
                }
            }
        }
    }

    fn ensure_state(&self, state: &mut SequenceState) {
        let current_hash = self.hash_params();

        if state.params_hash != current_hash {
            state.all_notes = self.get_all_notes();
            state.last_chord_index = usize::MAX;
            state.active_notes.clear();
            state.previous_notes.clear();
            state.params_hash = current_hash;
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
            state.current_bar = (state.current_bar + 1) % self.bars;
        }
        state.last_clock_position = clock_position;

        let sequence_position = (state.current_bar as f32 + clock_position) / self.bars as f32;

        let (chord_index, active_chord) = self.find_active_chord(&self.elements, sequence_position);

        let chord_changed = chord_index != state.last_chord_index;

        if chord_changed {
            state.previous_notes = state.active_notes.clone();
            state.active_notes.clear();
            if let Some(chord) = active_chord {
                state.active_notes.extend(chord.notes.iter().cloned());
            }
            state.last_chord_index = chord_index;
        }

        state
            .all_notes
            .iter()
            .map(|&note| {
                let was_on = state.previous_notes.contains(&note);
                let is_on = state.active_notes.contains(&note);

                Key {
                    on: if chord_changed && was_on && is_on {
                        false
                    } else {
                        is_on
                    },
                    note,
                    pitch: note as f32,
                }
            })
            .collect()
    }

    fn find_active_chord<'a>(
        &'a self,
        elements: &'a [SequenceElement],
        position: f32,
    ) -> (usize, Option<&'a Chord>) {
        if elements.is_empty() {
            return (0, None);
        }

        let element_index = ((position * elements.len() as f32) as usize).min(elements.len() - 1);
        let element_position = (position * elements.len() as f32) % 1.0;

        match &elements[element_index] {
            SequenceElement::Chord(chord) => (element_index, Some(chord)),
            SequenceElement::Subdivision(sub_elements) => {
                let (sub_index, chord) = self.find_active_chord(sub_elements, element_position);
                (element_index * 1000 + sub_index, chord)
            }
        }
    }

    fn get_all_notes(&self) -> Vec<i32> {
        let mut all_notes = std::collections::HashSet::new();
        self.collect_notes_from_elements(&self.elements, &mut all_notes);
        let mut notes: Vec<i32> = all_notes.into_iter().collect();
        notes.sort();
        notes
    }

    fn collect_notes_from_elements(
        &self,
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
                    self.collect_notes_from_elements(sub_elements, all_notes);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub on: bool,
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
