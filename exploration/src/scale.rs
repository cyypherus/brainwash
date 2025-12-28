pub struct Scale {
    notes: [i32; 7],
    shift: i32,
}

impl Scale {
    pub fn new(notes: [i32; 7]) -> Self {
        Scale { notes, shift: 0 }
    }

    pub fn shift(mut self, semitones: i32) -> Self {
        self.shift = semitones;
        self
    }

    pub fn degree(&self, index: i32) -> i32 {
        let scale_len = self.notes.len() as i32;
        let octave_offset = if index < 0 {
            ((index + 1) / scale_len) - 1
        } else {
            index / scale_len
        };
        let wrapped_index = ((index % scale_len) + scale_len) % scale_len;
        (self.notes[wrapped_index as usize] + self.shift) + octave_offset * 12
    }

    pub fn hz(&self, pitch: u8) -> f32 {
        let semitones = self.degree(pitch as i32) as f32;
        440.0 * 2.0_f32.powf(semitones / 12.0)
    }
}

pub fn cmaj() -> Scale {
    Scale::new([0, 2, 4, 5, 7, 9, 11]).shift(52)
}

pub fn cmin() -> Scale {
    Scale::new([0, 2, 3, 5, 7, 8, 10]).shift(52)
}

pub fn chromatic() -> Scale {
    Scale::new([0, 1, 2, 3, 4, 5, 6])
}
