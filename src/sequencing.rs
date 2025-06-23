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

pub struct Sequence {
    elements: Vec<SequenceElement>,
    pub(crate) all_notes: Vec<i32>,
    pub(crate) last_chord_index: Option<usize>,
    pub(crate) active_notes: std::collections::HashSet<i32>,
    pub(crate) previous_notes: std::collections::HashSet<i32>,
    pub(crate) current_bar: usize,
    pub(crate) last_clock_position: f32,
}

impl Default for Sequence {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            all_notes: Vec::new(),
            last_chord_index: None,
            active_notes: std::collections::HashSet::new(),
            previous_notes: std::collections::HashSet::new(),
            current_bar: 0,
            last_clock_position: 0.0,
        }
    }
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

pub fn sub(elements: impl Into<Vec<SequenceElement>>) -> SequenceElement {
    SequenceElement::Subdivision(elements.into())
}

impl Sequence {
    pub fn elements(&mut self, elements: impl Into<Vec<SequenceElement>>) -> &mut Self {
        self.elements = elements.into();
        self
    }
    pub fn output(&mut self, clock_position: f32) -> Vec<Key> {
        if self.elements.is_empty() {
            return Vec::new();
        }
        self.all_notes = self.get_all_notes();
        if clock_position < self.last_clock_position {
            self.current_bar = ((self.current_bar + 1) as f32 % 1.) as usize;
        }
        self.last_clock_position = clock_position;

        let raw_position = self.current_bar as f32 + clock_position;
        let sequence_position = raw_position;

        let (chord_index, active_chord) =
            Self::find_active_chord(&self.elements, sequence_position);

        let chord_changed = self
            .last_chord_index
            .is_some_and(|last| last != chord_index);

        if chord_changed || self.last_chord_index.is_none() {
            self.previous_notes = self.active_notes.clone();
            self.active_notes.clear();
            if let Some(chord) = active_chord {
                self.active_notes.extend(chord.notes.iter().cloned());
            }
            self.last_chord_index = Some(chord_index);
        }

        let mut keys = Vec::with_capacity(self.active_notes.len());
        let mut on_index = 0;
        for (index, &note) in self.all_notes.iter().enumerate() {
            let was_on = self.previous_notes.contains(&note);
            let is_on = self.active_notes.contains(&note);

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
    /// The index of the key returned from the sequence, not including keys that are off
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
