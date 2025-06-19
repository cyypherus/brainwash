use crate::Signal;

pub struct Chord {
    notes: Vec<i32>,
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

pub struct Sequence {
    chords: Vec<Chord>,
    bar_duration_samples: usize,
    bpm: f32,
    bars: usize,
}

pub fn chord(notes: &[i32]) -> Chord {
    Chord {
        notes: notes.to_vec(),
    }
}

pub fn note(note: i32) -> Chord {
    chord(&[note])
}

pub fn sequence<T: Into<Vec<Chord>>>(_id: usize, chords: T) -> Sequence {
    Sequence {
        chords: chords.into(),
        bar_duration_samples: 44100,
        bpm: 120.0,
        bars: 1,
    }
}

impl Sequence {
    pub fn tempo(mut self, bpm: f32) -> Self {
        self.bpm = bpm;
        self.bar_duration_samples = self.calculate_bar_duration_samples(44100);
        self
    }

    pub fn bars(mut self, bars: usize) -> Self {
        self.bars = bars;
        self
    }

    fn calculate_bar_duration_samples(&self, sample_rate: usize) -> usize {
        let beats_per_second = self.bpm / 60.0;
        let seconds_per_beat = 1.0 / beats_per_second;
        let seconds_per_bar = seconds_per_beat * 4.0;
        (seconds_per_bar * sample_rate as f32) as usize
    }

    pub fn output(&mut self, signal: &mut Signal) -> Vec<Key> {
        if self.chords.is_empty() {
            return Vec::new();
        }

        let all_notes = self.get_all_notes();
        let bar_duration = self.calculate_bar_duration_samples(signal.sample_rate);
        let sequence_duration = bar_duration * self.bars;
        let chord_duration = sequence_duration / self.chords.len();
        let position_in_sequence = signal.position % sequence_duration;
        let chord_index = position_in_sequence / chord_duration;

        let active_notes: std::collections::HashSet<i32> =
            if let Some(chord) = self.chords.get(chord_index) {
                chord.notes.iter().cloned().collect()
            } else {
                std::collections::HashSet::new()
            };

        all_notes
            .iter()
            .map(|&note| Key {
                on: active_notes.contains(&note),
                note,
                pitch: note as f32,
            })
            .collect()
    }

    fn get_all_notes(&self) -> Vec<i32> {
        let mut all_notes = std::collections::HashSet::new();
        for chord in &self.chords {
            for &note in &chord.notes {
                all_notes.insert(note);
            }
        }
        let mut notes: Vec<i32> = all_notes.into_iter().collect();
        notes.sort();
        notes
    }

    pub fn set_bar_duration(&mut self, samples: usize) {
        self.bar_duration_samples = samples;
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
