pub struct Scale {
    notes: [i32; 7],
    shift: i32,
}

impl Scale {
    pub fn shift(&mut self, i: i32) -> &mut Self {
        self.shift = i;
        self
    }

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

pub fn cmaj() -> Scale {
    Scale {
        notes: [0, 2, 4, 5, 7, 9, 11],
        shift: 0,
    }
}

pub fn cmin() -> Scale {
    Scale {
        notes: [0, 2, 3, 5, 7, 8, 10],
        shift: 0,
    }
}
