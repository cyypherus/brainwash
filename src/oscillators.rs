use crate::{Signal, utils};

#[derive(Clone, Copy)]
pub enum WaveType {
    Sine,
    Square,
    Triangle,
    SawUp,
    SawDown,
}

pub struct Oscillator {
    wave_type: WaveType,
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    computed_sample: f32,
}

pub fn sin(_id: usize) -> Oscillator {
    Oscillator::new(WaveType::Sine)
}

pub fn squ(_id: usize) -> Oscillator {
    Oscillator::new(WaveType::Square)
}

pub fn tri(_id: usize) -> Oscillator {
    Oscillator::new(WaveType::Triangle)
}

pub fn saw(_id: usize) -> Oscillator {
    Oscillator::new(WaveType::SawUp)
}

pub fn rsaw(_id: usize) -> Oscillator {
    Oscillator::new(WaveType::SawDown)
}

impl Oscillator {
    fn new(wave_type: WaveType) -> Self {
        Self {
            wave_type,
            pitch: 0.0,
            frequency: 440.0,
            attenuation: 1.0,
            phase_offset: 0.0,
            computed_sample: 0.0,
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

    pub fn play(mut self, signal: &mut Signal) -> Self {
        self.calculate_time_based(signal);
        signal.add_sample(self.computed_sample);
        self
    }

    pub fn run(mut self, signal: &mut Signal) -> Self {
        self.calculate_time_based(signal);
        self
    }

    pub fn at_phase(mut self, phase: f32) -> Self {
        self.calculate_phase_based(phase);
        self
    }

    pub fn output(self) -> f32 {
        self.computed_sample
    }

    fn calculate_time_based(&mut self, signal: &Signal) {
        let time = signal.position as f32 / signal.sample_rate as f32;
        let phase = (time * self.frequency) % 1.0;
        self.calculate_phase_based(phase);
    }

    fn calculate_phase_based(&mut self, phase: f32) {
        let adjusted_phase = (phase + self.phase_offset / (2.0 * std::f32::consts::PI)) % 1.0;

        let sample = match self.wave_type {
            WaveType::Sine => (adjusted_phase * 2.0 * std::f32::consts::PI).sin(),
            WaveType::Square => {
                if adjusted_phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveType::Triangle => {
                if adjusted_phase < 0.5 {
                    -1.0 + 4.0 * adjusted_phase
                } else {
                    3.0 - 4.0 * adjusted_phase
                }
            }
            WaveType::SawUp => -1.0 + 2.0 * adjusted_phase,
            WaveType::SawDown => 1.0 - 2.0 * adjusted_phase,
        };

        self.computed_sample = sample * self.attenuation;
    }
}
