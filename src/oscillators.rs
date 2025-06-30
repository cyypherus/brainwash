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
        }
    }

    pub fn wave(&mut self, wave_type: Wave) -> &mut Self {
        self.wave_type = wave_type;
        self
    }

    pub fn phase_offset(&mut self, p: f32) -> &mut Self {
        self.phase_offset = p;
        self
    }

    pub fn pitch(&mut self, p: f32) -> &mut Self {
        self.pitch = p;
        self.frequency = utils::note_to_freq(p);
        self
    }

    pub fn freq(&mut self, f: f32) -> &mut Self {
        self.frequency = f;
        self
    }

    pub fn atten(&mut self, a: f32) -> &mut Self {
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

        let phase_increment =
            ((self.frequency as f64 / sample_rate as f64) * (u32::MAX as f64 + 1.0)) as u32;

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
