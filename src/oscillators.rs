use crate::{Signal, utils};

#[derive(Clone, Copy)]
pub enum WaveType {
    Sine,
    Square,
    Triangle,
    SawUp,
    SawDown,
}

#[derive(Default)]
pub struct Sine {
    osc: Oscillator,
}
impl OscillatorType for Sine {
    fn oscillator(&mut self) -> &mut Oscillator {
        &mut self.osc
    }
}

pub struct Square {
    osc: Oscillator,
}
impl Default for Square {
    fn default() -> Self {
        Self {
            osc: Oscillator::new(WaveType::Square),
        }
    }
}
impl OscillatorType for Square {
    fn oscillator(&mut self) -> &mut Oscillator {
        &mut self.osc
    }
}

pub struct Triangle {
    osc: Oscillator,
}
impl Default for Triangle {
    fn default() -> Self {
        Self {
            osc: Oscillator::new(WaveType::Triangle),
        }
    }
}
impl OscillatorType for Triangle {
    fn oscillator(&mut self) -> &mut Oscillator {
        &mut self.osc
    }
}

pub struct SawUp {
    osc: Oscillator,
}
impl Default for SawUp {
    fn default() -> Self {
        Self {
            osc: Oscillator::new(WaveType::SawUp),
        }
    }
}
impl OscillatorType for SawUp {
    fn oscillator(&mut self) -> &mut Oscillator {
        &mut self.osc
    }
}

pub struct SawDown {
    osc: Oscillator,
}
impl Default for SawDown {
    fn default() -> Self {
        Self {
            osc: Oscillator::new(WaveType::SawDown),
        }
    }
}
impl OscillatorType for SawDown {
    fn oscillator(&mut self) -> &mut Oscillator {
        &mut self.osc
    }
}

pub struct Oscillator {
    index: usize,
    computed_sample: f32,
    wave_type: WaveType,
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    is_bipolar: bool,
    pub(crate) phase_accumulator: u32,
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new(WaveType::Sine)
    }
}

pub trait OscillatorType {
    fn oscillator(&mut self) -> &mut Oscillator;

    fn phase(&mut self, p: f32) -> &mut Self {
        self.oscillator().phase_offset = p;
        self
    }

    fn pitch(&mut self, p: f32) -> &mut Self {
        self.oscillator().pitch = p;
        self.oscillator().frequency = utils::note_to_freq(p);
        self
    }

    fn freq(&mut self, f: f32) -> &mut Self {
        self.oscillator().frequency = f;
        self
    }

    fn atten(&mut self, a: f32) -> &mut Self {
        self.oscillator().attenuation = a;
        self
    }

    fn bipolar(&mut self) -> &mut Self {
        self.oscillator().is_bipolar = true;
        self
    }

    fn play(&mut self, signal: &mut Signal) -> &mut Self {
        self.oscillator().calculate_time_based(signal);
        signal.add_sample(self.oscillator().computed_sample);
        self
    }

    fn run(&mut self, signal: &mut Signal) -> &mut Self {
        self.oscillator().calculate_time_based(signal);
        self
    }

    fn value_at(&mut self, phase: f32) -> &mut Self {
        self.oscillator().calculate_phase_based(phase);
        self
    }

    fn output(&mut self) -> f32 {
        self.oscillator().computed_sample
    }
}

impl Oscillator {
    fn new(wave_type: WaveType) -> Self {
        Self {
            index: 0,
            wave_type,
            pitch: 0.0,
            frequency: 440.0,
            attenuation: 1.0,
            phase_offset: 0.0,
            computed_sample: 0.0,
            is_bipolar: false,
            phase_accumulator: 0,
        }
    }

    pub fn index(mut self, id: usize) -> Self {
        self.index = id;
        self
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
