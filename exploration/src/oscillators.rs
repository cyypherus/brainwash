use crate::Key;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub enum Wave {
    Sin,
    Square,
    Triangle,
    SawUp,
    SawDown,
    WhiteNoise,
}

#[derive(Clone, Debug)]
pub struct Osc {
    wave: Wave,
    frequency: Option<f32>,
    semitone_shift: f32,
    phase_offset: f32,
    unipolar: bool,
    phase: u32,
    noise_seed: u32,
}

impl Osc {
    pub fn new(wave: Wave) -> Self {
        Osc {
            wave,
            frequency: None,
            semitone_shift: 0.0,
            phase_offset: 0.0,
            unipolar: false,
            phase: 0,
            noise_seed: 22222,
        }
    }

    pub fn hz(&mut self, frequency: f32) -> &mut Self {
        self.frequency = Some(frequency);
        self
    }

    pub fn shift(&mut self, semitones: f32) -> &mut Self {
        self.semitone_shift = semitones;
        self
    }

    pub fn offset(&mut self, offset: f32) -> &mut Self {
        self.phase_offset = offset;
        self
    }

    pub fn unipolar(&mut self) -> &mut Self {
        self.unipolar = true;
        self
    }

    pub fn play(&mut self, frequency: f32) -> f32 {
        const SAMPLE_RATE: f32 = 44100.0;

        let freq = if let Some(fixed_freq) = self.frequency {
            fixed_freq * 2.0_f32.powf(self.semitone_shift / 12.0)
        } else {
            frequency * 2.0_f32.powf(self.semitone_shift / 12.0)
        };

        let phase_increment = ((freq as f64 / SAMPLE_RATE as f64) * (u32::MAX as f64 + 1.0)) as u32;
        self.phase = self.phase.wrapping_add(phase_increment);

        let phase = self.phase as f32 / (u32::MAX as f32 + 1.0);
        let adjusted_phase = (phase + self.phase_offset / (2.0 * PI)) % 1.0;

        match self.wave {
            Wave::Sin => {
                let bipolar_sample = (adjusted_phase * 2.0 * PI).sin();
                if self.unipolar {
                    (bipolar_sample + 1.0) * 0.5
                } else {
                    bipolar_sample
                }
            }
            Wave::Square => {
                let bipolar_sample = if adjusted_phase < 0.5 { 1.0 } else { -1.0 };
                if self.unipolar {
                    (bipolar_sample + 1.0) * 0.5
                } else {
                    bipolar_sample
                }
            }
            Wave::Triangle => {
                let bipolar_sample = if adjusted_phase < 0.5 {
                    -1.0 + 4.0 * adjusted_phase
                } else {
                    3.0 - 4.0 * adjusted_phase
                };
                if self.unipolar {
                    (bipolar_sample + 1.0) * 0.5
                } else {
                    bipolar_sample
                }
            }
            Wave::SawUp => {
                let bipolar_sample = -1.0 + 2.0 * adjusted_phase;
                if self.unipolar {
                    (bipolar_sample + 1.0) * 0.5
                } else {
                    bipolar_sample
                }
            }
            Wave::SawDown => {
                let bipolar_sample = 1.0 - 2.0 * adjusted_phase;
                if self.unipolar {
                    (bipolar_sample + 1.0) * 0.5
                } else {
                    bipolar_sample
                }
            }
            Wave::WhiteNoise => {
                self.noise_seed = self
                    .noise_seed
                    .wrapping_mul(196314165)
                    .wrapping_add(907633515);
                let bipolar_sample = (self.noise_seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                if self.unipolar {
                    (bipolar_sample + 1.0) * 0.5
                } else {
                    bipolar_sample
                }
            }
        }
    }
}

pub fn sin() -> Osc {
    Osc::new(Wave::Sin)
}

pub fn squ() -> Osc {
    Osc::new(Wave::Square)
}

pub fn tri() -> Osc {
    Osc::new(Wave::Triangle)
}

pub fn saw() -> Osc {
    Osc::new(Wave::SawUp)
}

pub fn rsaw() -> Osc {
    Osc::new(Wave::SawDown)
}

pub fn noise() -> Osc {
    Osc::new(Wave::WhiteNoise)
}
