use crate::{Signal, utils};

#[derive(Clone, Copy)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    SawUp,
    SawDown,
    WhiteNoise,
}

pub struct Osc {
    computed_sample: f32,
    wave_type: Wave,
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    unipolar: bool,
    pub(crate) phase_accumulator: u32,
    noise_seed: u32,
    shift_semitones: f32,
}

impl Default for Osc {
    fn default() -> Self {
        Self::new(Wave::Sine)
    }
}

pub fn sin() -> Wave {
    Wave::Sine
}

pub fn square() -> Wave {
    Wave::Square
}

pub fn triangle() -> Wave {
    Wave::Triangle
}

pub fn saw() -> Wave {
    Wave::SawUp
}

pub fn rsaw() -> Wave {
    Wave::SawDown
}

pub fn noise() -> Wave {
    Wave::WhiteNoise
}

impl Osc {
    fn new(wave_type: Wave) -> Self {
        Self {
            wave_type,
            pitch: 0.0,
            frequency: 440.0,
            attenuation: 1.0,
            phase_offset: 0.0,
            computed_sample: 0.0,
            unipolar: false,
            phase_accumulator: 0,
            noise_seed: 22222,
            shift_semitones: 0.0,
        }
    }

    pub fn saw(&mut self) -> &mut Self {
        self.wave_type = saw();
        self
    }

    pub fn rsaw(&mut self) -> &mut Self {
        self.wave_type = rsaw();
        self
    }

    pub fn tri(&mut self) -> &mut Self {
        self.wave_type = triangle();
        self
    }

    pub fn squ(&mut self) -> &mut Self {
        self.wave_type = square();
        self
    }

    pub fn sin(&mut self) -> &mut Self {
        self.wave_type = sin();
        self
    }

    pub fn noise(&mut self) -> &mut Self {
        self.wave_type = noise();
        self
    }

    pub fn shift(&mut self, semitones: f32) -> &mut Self {
        self.shift_semitones = semitones;
        self
    }

    pub fn freq(&mut self, f: f32) -> &mut Self {
        self.frequency = f;
        self
    }

    pub fn gain(&mut self, a: f32) -> &mut Self {
        self.attenuation = a.clamp(0., 1.);
        self
    }

    pub fn unipolar(&mut self) -> &mut Self {
        self.unipolar = true;
        self
    }

    pub fn output_phase(&mut self, phase: f32) -> f32 {
        self.calculate_phase_based(phase);
        self.computed_sample
    }

    pub fn output(&mut self, signal: &mut Signal) -> f32 {
        self.calculate_time_based(signal);
        self.computed_sample
    }

    fn calculate_time_based(&mut self, signal: &mut Signal) {
        let sample_rate = signal.sample_rate as f32;
        let shifted_freq = self.frequency * 2.0_f32.powf(self.shift_semitones / 12.0);

        let phase_increment =
            ((shifted_freq as f64 / sample_rate as f64) * (u32::MAX as f64 + 1.0)) as u32;

        self.phase_accumulator = self.phase_accumulator.wrapping_add(phase_increment);
        let phase = self.phase_accumulator as f32 / (u32::MAX as f32 + 1.0);
        self.calculate_phase_based(phase);
    }

    fn calculate_phase_based(&mut self, phase: f32) {
        let adjusted_phase = (phase + self.phase_offset / (2.0 * std::f32::consts::PI)) % 1.0;

        let sample = match self.wave_type {
            Wave::Sine => {
                let bipolar_sample = (adjusted_phase * 2.0 * std::f32::consts::PI).sin();
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
        };

        self.computed_sample = sample * self.attenuation;
    }
}
