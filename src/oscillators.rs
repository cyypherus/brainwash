use crate::{Signal, utils};

pub(crate) struct OscillatorState {
    pub(crate) phase_accumulator: u32,
}

#[derive(Clone, Copy)]
pub enum WaveType {
    Sine,
    Square,
    Triangle,
    SawUp,
    SawDown,
}

pub struct Oscillator {
    id: usize,
    index: usize,
    wave_type: WaveType,
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    computed_sample: f32,
    is_bipolar: bool,
}

pub fn sin(id: usize) -> Oscillator {
    Oscillator::new(id, WaveType::Sine)
}

pub fn squ(id: usize) -> Oscillator {
    Oscillator::new(id, WaveType::Square)
}

pub fn tri(id: usize) -> Oscillator {
    Oscillator::new(id, WaveType::Triangle)
}

/// Saw up
pub fn saw(id: usize) -> Oscillator {
    Oscillator::new(id, WaveType::SawUp)
}

/// Saw down
pub fn rsaw(id: usize) -> Oscillator {
    Oscillator::new(id, WaveType::SawDown)
}

impl Oscillator {
    fn new(id: usize, wave_type: WaveType) -> Self {
        Self {
            id,
            index: 0,
            wave_type,
            pitch: 0.0,
            frequency: 440.0,
            attenuation: 1.0,
            phase_offset: 0.0,
            computed_sample: 0.0,
            is_bipolar: false,
        }
    }

    pub fn phase(mut self, p: f32) -> Self {
        self.phase_offset = p;
        self
    }

    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p;
        self.frequency = utils::note_to_freq(p);
        self
    }

    pub fn freq(mut self, f: f32) -> Self {
        self.frequency = f;
        self
    }

    pub fn atten(mut self, a: f32) -> Self {
        self.attenuation = a;
        self
    }

    pub fn bipolar(mut self) -> Self {
        self.is_bipolar = true;
        self
    }

    pub fn play(mut self, signal: &mut Signal) -> Self {
        self.calculate_time_based(signal);
        signal.add_sample(self.computed_sample);
        self
    }

    pub fn run(mut self, signal: &mut Signal) -> Self {
        self.calculate_time_based(signal);
        self
    }

    pub fn value_at(mut self, phase: f32) -> Self {
        self.calculate_phase_based(phase);
        self
    }

    pub fn output(self) -> f32 {
        self.computed_sample
    }

    pub fn index(mut self, id: usize) -> Self {
        self.index = id;
        self
    }

    fn calculate_time_based(&mut self, signal: &mut Signal) {
        let sample_rate = signal.sample_rate as f32;
        let state = signal.get_oscillator_state(self.id as i32, self.index as i32);

        let phase_increment =
            ((self.frequency as f64 / sample_rate as f64) * (u32::MAX as f64 + 1.0)) as u32;

        state.phase_accumulator = state.phase_accumulator.wrapping_add(phase_increment);
        let phase = state.phase_accumulator as f32 / (u32::MAX as f32 + 1.0);
        self.calculate_phase_based(phase);
    }

    fn calculate_phase_based(&mut self, phase: f32) {
        let adjusted_phase = (phase + self.phase_offset / (2.0 * std::f32::consts::PI)) % 1.0;

        let sample = match self.wave_type {
            WaveType::Sine => {
                let bipolar_sample = (adjusted_phase * 2.0 * std::f32::consts::PI).sin();
                if self.is_bipolar {
                    bipolar_sample
                } else {
                    (bipolar_sample + 1.0) * 0.5
                }
            }
            WaveType::Square => {
                let bipolar_sample = if adjusted_phase < 0.5 { 1.0 } else { -1.0 };
                if self.is_bipolar {
                    bipolar_sample
                } else {
                    (bipolar_sample + 1.0) * 0.5
                }
            }
            WaveType::Triangle => {
                let bipolar_sample = if adjusted_phase < 0.5 {
                    -1.0 + 4.0 * adjusted_phase
                } else {
                    3.0 - 4.0 * adjusted_phase
                };
                if self.is_bipolar {
                    bipolar_sample
                } else {
                    (bipolar_sample + 1.0) * 0.5
                }
            }
            WaveType::SawUp => {
                let bipolar_sample = -1.0 + 2.0 * adjusted_phase;
                if self.is_bipolar {
                    bipolar_sample
                } else {
                    (bipolar_sample + 1.0) * 0.5
                }
            }
            WaveType::SawDown => {
                let bipolar_sample = 1.0 - 2.0 * adjusted_phase;
                if self.is_bipolar {
                    bipolar_sample
                } else {
                    (bipolar_sample + 1.0) * 0.5
                }
            }
        };

        self.computed_sample = sample * self.attenuation;
    }
}
