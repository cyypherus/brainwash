pub struct Chord {
    notes: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordValue {
    Rest,
    Note(i32),
    Dyad([i32; 2]),
    Triad([i32; 3]),
    Tetrad([i32; 4]),
    Pentad([i32; 5]),
    Hexad([i32; 6]),
    Heptad([i32; 7]),
    Octad([i32; 8]),
}

pub fn rest() -> ChordValue {
    ChordValue::Rest
}

pub fn note(note: i32) -> ChordValue {
    ChordValue::Note(note)
}

pub fn dy(notes: [i32; 2]) -> ChordValue {
    ChordValue::Dyad(notes)
}

pub fn tri(notes: [i32; 3]) -> ChordValue {
    ChordValue::Triad(notes)
}

pub fn tet(notes: [i32; 4]) -> ChordValue {
    ChordValue::Tetrad(notes)
}

pub fn pent(notes: [i32; 5]) -> ChordValue {
    ChordValue::Pentad(notes)
}

pub fn hex(notes: [i32; 6]) -> ChordValue {
    ChordValue::Hexad(notes)
}

pub fn hept(notes: [i32; 7]) -> ChordValue {
    ChordValue::Heptad(notes)
}

pub fn oct(notes: [i32; 8]) -> ChordValue {
    ChordValue::Octad(notes)
}

impl ChordValue {
    fn for_each_note<F>(&self, mut f: F)
    where
        F: FnMut(i32),
    {
        match self {
            ChordValue::Rest => (),
            ChordValue::Note(note) => f(*note),
            ChordValue::Dyad(notes) => notes.iter().for_each(|&n| f(n)),
            ChordValue::Triad(notes) => notes.iter().for_each(|&n| f(n)),
            ChordValue::Tetrad(notes) => notes.iter().for_each(|&n| f(n)),
            ChordValue::Pentad(notes) => notes.iter().for_each(|&n| f(n)),
            ChordValue::Hexad(notes) => notes.iter().for_each(|&n| f(n)),
            ChordValue::Heptad(notes) => notes.iter().for_each(|&n| f(n)),
            ChordValue::Octad(notes) => notes.iter().for_each(|&n| f(n)),
        }
    }
}

pub enum SequenceValue {
    Bar(Option<ChordValue>),
    Half([Option<ChordValue>; 2]),
    Third([Option<ChordValue>; 3]),
    Quarter([Option<ChordValue>; 4]),
    Fifth([Option<ChordValue>; 5]),
    Sixth([Option<ChordValue>; 6]),
    Seventh([Option<ChordValue>; 7]),
    Eighth([Option<ChordValue>; 8]),
    Ninth([Option<ChordValue>; 9]),
    Tenth([Option<ChordValue>; 10]),
    Twelfth([Option<ChordValue>; 12]),
    Sixteenth([Option<ChordValue>; 16]),
}

pub fn bar(v: ChordValue) -> SequenceValue {
    SequenceValue::Bar(Some(v))
}

fn slice_to_seq_value<const N: usize>(elements: [ChordValue; N]) -> SequenceValue {
    match N {
        2 => SequenceValue::Half([Some(elements[0]), Some(elements[1])]),
        3 => SequenceValue::Third([Some(elements[0]), Some(elements[1]), Some(elements[2])]),
        4 => SequenceValue::Quarter([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
        ]),
        5 => SequenceValue::Fifth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
        ]),
        6 => SequenceValue::Sixth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
        ]),
        7 => SequenceValue::Seventh([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
            Some(elements[6]),
        ]),
        8 => SequenceValue::Eighth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
            Some(elements[6]),
            Some(elements[7]),
        ]),
        9 => SequenceValue::Ninth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
            Some(elements[6]),
            Some(elements[7]),
            Some(elements[8]),
        ]),
        10 => SequenceValue::Tenth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
            Some(elements[6]),
            Some(elements[7]),
            Some(elements[8]),
            Some(elements[9]),
        ]),
        12 => SequenceValue::Twelfth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
            Some(elements[6]),
            Some(elements[7]),
            Some(elements[8]),
            Some(elements[9]),
            Some(elements[10]),
            Some(elements[11]),
        ]),
        16 => SequenceValue::Sixteenth([
            Some(elements[0]),
            Some(elements[1]),
            Some(elements[2]),
            Some(elements[3]),
            Some(elements[4]),
            Some(elements[5]),
            Some(elements[6]),
            Some(elements[7]),
            Some(elements[8]),
            Some(elements[9]),
            Some(elements[10]),
            Some(elements[11]),
            Some(elements[12]),
            Some(elements[13]),
            Some(elements[14]),
            Some(elements[15]),
        ]),
        _ => panic!("Unsupported sequence length"),
    }
}

pub struct Sequence {
    elements: SequenceValue,
    pub(crate) all_notes: Vec<i32>,
    pub(crate) last_chord_index: Option<usize>,
    pub(crate) active_notes: std::collections::HashSet<i32>,
    pub(crate) previous_notes: std::collections::HashSet<i32>,
    pub(crate) current_bar: usize,
    pub(crate) last_clock_position: f32,
    all_notes_set: std::collections::HashSet<i32>,
    keys: Vec<Key>,
}

impl Default for Sequence {
    fn default() -> Self {
        Self {
            elements: SequenceValue::Bar(None),
            all_notes: Vec::with_capacity(64),
            last_chord_index: None,
            active_notes: std::collections::HashSet::with_capacity(16),
            previous_notes: std::collections::HashSet::with_capacity(16),
            current_bar: 0,
            last_clock_position: 0.0,
            all_notes_set: std::collections::HashSet::with_capacity(64),
            keys: Vec::with_capacity(64),
        }
    }
}

impl Sequence {
    pub fn elements<const N: usize>(&mut self, elements: [ChordValue; N]) -> &mut Self {
        self.elements = slice_to_seq_value(elements);
        self
    }

    pub fn output(&mut self, clock_position: f32) -> &Vec<Key> {
        self.get_all_notes();

        if clock_position < self.last_clock_position {
            self.current_bar = ((self.current_bar + 1) as f32 % 1.) as usize;
        }
        self.last_clock_position = clock_position;

        let raw_position = self.current_bar as f32 + clock_position;
        let sequence_position = raw_position;

        let (step_index, active_value) = self.find_active_step(sequence_position);

        let step_changed = self.last_chord_index.is_some_and(|last| last != step_index);

        if step_changed || self.last_chord_index.is_none() {
            std::mem::swap(&mut self.previous_notes, &mut self.active_notes);
            self.active_notes.clear();

            if let Some(value) = active_value {
                value.for_each_note(|note| {
                    self.active_notes.insert(note);
                });
            }
            self.last_chord_index = Some(step_index);
        }

        self.keys.clear();
        let mut on_index = 0;
        for (index, &note) in self.all_notes.iter().enumerate() {
            let was_on = self.previous_notes.contains(&note);
            let is_on = self.active_notes.contains(&note);

            if is_on {
                on_index += 1;
            }

            self.keys.push(Key {
                on: if step_changed && was_on && is_on {
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
        &self.keys
    }

    fn find_active_step(&self, position: f32) -> (usize, Option<ChordValue>) {
        let pos = position % 1.0;

        match &self.elements {
            SequenceValue::Bar(val) => (0, *val),
            SequenceValue::Half(arr) => {
                let index = (pos * 2.0) as usize % 2;
                (index, arr[index])
            }
            SequenceValue::Third(arr) => {
                let index = (pos * 3.0) as usize % 3;
                (index, arr[index])
            }
            SequenceValue::Quarter(arr) => {
                let index = (pos * 4.0) as usize % 4;
                (index, arr[index])
            }
            SequenceValue::Fifth(arr) => {
                let index = (pos * 5.0) as usize % 5;
                (index, arr[index])
            }
            SequenceValue::Sixth(arr) => {
                let index = (pos * 6.0) as usize % 6;
                (index, arr[index])
            }
            SequenceValue::Seventh(arr) => {
                let index = (pos * 7.0) as usize % 7;
                (index, arr[index])
            }
            SequenceValue::Eighth(arr) => {
                let index = (pos * 8.0) as usize % 8;
                (index, arr[index])
            }
            SequenceValue::Ninth(arr) => {
                let index = (pos * 9.0) as usize % 9;
                (index, arr[index])
            }
            SequenceValue::Sixteenth(arr) => {
                let index = (pos * 16.0) as usize % 16;
                (index, arr[index])
            }
            SequenceValue::Tenth(arr) => {
                let index = (pos * 10.0) as usize % 10;
                (index, arr[index])
            }
            SequenceValue::Twelfth(arr) => {
                let index = (pos * 12.0) as usize % 12;
                (index, arr[index])
            }
        }
    }

    fn get_all_notes(&mut self) {
        self.all_notes_set.clear();
        self.all_notes.clear();
        self.collect_notes_from_sequence();
        for note in self.all_notes_set.iter() {
            self.all_notes.push(*note);
        }
    }

    fn collect_notes_from_sequence(&mut self) {
        match &self.elements {
            SequenceValue::Bar(val) => {
                if let Some(chord_value) = val {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Half(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Third(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Quarter(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Fifth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Sixth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Seventh(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Eighth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Ninth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Tenth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Twelfth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
                }
            }
            SequenceValue::Sixteenth(arr) => {
                for chord_value in arr.iter().flatten() {
                    chord_value.for_each_note(|note| {
                        self.all_notes_set.insert(note);
                    });
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
