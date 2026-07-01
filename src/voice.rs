use crate::env::Gate;
use crate::sample::Unit;
use crate::time::Hertz;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Note(u8);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteEvent {
    note: Note,
    gate: Gate,
    velocity: Unit,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    events: Vec<NoteEvent>,
}

impl Note {
    pub fn new(value: u8) -> Option<Self> {
        (value <= 127).then_some(Self(value))
    }

    pub fn value(self) -> u8 {
        self.0
    }

    pub fn frequency(self) -> Hertz {
        let hz = 440.0 * 2.0_f32.powf((self.0 as f32 - 69.0) / 12.0);
        Hertz::new(hz).unwrap()
    }
}

impl NoteEvent {
    pub fn new(note: Note, gate: Gate, velocity: Unit) -> Self {
        Self {
            note,
            gate,
            velocity,
        }
    }

    pub fn note(self) -> Note {
        self.note
    }

    pub fn gate(self) -> Gate {
        self.gate
    }

    pub fn velocity(self) -> Unit {
        self.velocity
    }
}

impl Track {
    pub fn new(events: Vec<NoteEvent>) -> Self {
        Self { events }
    }

    pub fn events(&self) -> &[NoteEvent] {
        &self.events
    }
}
