use crate::Signal;

pub struct Chord {
    notes: Vec<i32>,
}

pub struct Sequence {
    chords: Vec<Chord>,
    bar_duration_samples: usize,
    bpm: f32,
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
        bpm: 120.0,
    }
}

impl Sequence {
    pub fn tempo(mut self, bpm: f32) -> Self {
        self.bpm = bpm;
        self.bar_duration_samples = self.calculate_bar_duration_samples(44100);
        self
    }

    fn calculate_bar_duration_samples(&self, sample_rate: usize) -> usize {
        let beats_per_second = self.bpm / 60.0;
        let seconds_per_beat = 1.0 / beats_per_second;
        let seconds_per_bar = seconds_per_beat * 4.0;
        (seconds_per_bar * sample_rate as f32) as usize
    }

    pub fn output(&mut self, signal: &mut Signal) -> Vec<(f32, i32)> {
        if self.chords.is_empty() {
            return Vec::new();
        }

        let pos = signal.position as u32;
        let bar_duration = self.calculate_bar_duration_samples(signal.sample_rate);
        let chord_duration = bar_duration / self.chords.len();
        let position_in_bar = signal.position % bar_duration;
        let chord_index = position_in_bar / chord_duration;

        if let Some(chord) = self.chords.get(chord_index) {
            for (i, (trigger, time)) in signal.pitch_triggers.iter_mut().enumerate() {
                if chord.notes.contains(&(i as i32)) {
                    if !*trigger {
                        *trigger = true;
                        *time = pos;
                    }
                } else {
                    if *trigger {
                        *trigger = false;
                        *time = pos;
                    }
                }
            }
            chord.notes.iter().map(|&n| (n as f32, n)).collect()
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
