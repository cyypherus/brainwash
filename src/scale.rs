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
        (self.notes[wrapped_index as usize] + self.shift) + octave_offset * 12
    }
}

const BASE_SHIFT: i32 = 48;
const MAJ: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const MIN: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];
const CHROM: [i32; 7] = [0, 1, 2, 3, 4, 5, 6];

pub fn chromatic() -> Scale {
    Scale {
        notes: CHROM,
        shift: BASE_SHIFT,
    }
}

pub fn cmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT,
    }
}
pub fn cmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT,
    }
}
pub fn csharpmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 1,
    }
}
pub fn csharpmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 1,
    }
}

pub fn dmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 2,
    }
}
pub fn dmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 2,
    }
}
pub fn dsharpmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 3,
    }
}
pub fn dsharpmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 3,
    }
}

pub fn emaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 4,
    }
}
pub fn emin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 4,
    }
}

pub fn fmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 5,
    }
}
pub fn fmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 5,
    }
}
pub fn fsharpmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 6,
    }
}
pub fn fsharpmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 6,
    }
}

pub fn gmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 7,
    }
}
pub fn gmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 7,
    }
}
pub fn gsharpmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 8,
    }
}
pub fn gsharpmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 8,
    }
}

pub fn amaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 9,
    }
}
pub fn amin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 9,
    }
}
pub fn asharpmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 10,
    }
}
pub fn asharpmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 10,
    }
}

pub fn bmaj() -> Scale {
    Scale {
        notes: MAJ,
        shift: BASE_SHIFT + 11,
    }
}
pub fn bmin() -> Scale {
    Scale {
        notes: MIN,
        shift: BASE_SHIFT + 11,
    }
}
