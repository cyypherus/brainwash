use crate::Signal;

pub struct Chord {
    notes: Vec<i32>,
}

pub struct Sequence {
    chords: Vec<Chord>,
    bar_duration_samples: usize,
}

pub fn chord(notes: &[i32]) -> Chord {
    Chord {
        notes: notes.to_vec(),
    }
}

pub fn sequence<T: Into<Vec<Chord>>>(chords: T) -> Sequence {
    Sequence {
        chords: chords.into(),
        bar_duration_samples: 44100,
    }
}

impl Sequence {
    pub fn output(&self, signal: &Signal) -> Vec<f32> {
        if self.chords.is_empty() {
            return Vec::new();
        }

        let chord_duration = self.bar_duration_samples / self.chords.len();
        let position_in_bar = signal.position % self.bar_duration_samples;
        let chord_index = position_in_bar / chord_duration;

        if let Some(chord) = self.chords.get(chord_index) {
            chord.notes.iter().map(|&n| n as f32).collect()
        } else {
            Vec::new()
        }
    }

    pub fn set_bar_duration(&mut self, samples: usize) {
        self.bar_duration_samples = samples;
    }
}

impl Chord {
    pub fn output(&self) -> Vec<f32> {
        self.notes.iter().map(|&n| n as f32).collect()
    }
}
