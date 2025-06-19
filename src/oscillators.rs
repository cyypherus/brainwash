use crate::{Signal, utils};

pub struct SineOscillator {
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    computed_sample: f32,
}

pub struct SquareOscillator {
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    computed_sample: f32,
}

pub struct TriangleOscillator {
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
    computed_sample: f32,
}

pub fn sin(_id: usize) -> SineOscillator {
    SineOscillator {
        pitch: 0.0,
        frequency: 440.0,
        attenuation: 1.0,
        phase_offset: 0.0,
        computed_sample: 0.0,
    }
}

pub fn squ(_id: usize) -> SquareOscillator {
    SquareOscillator {
        pitch: 0.0,
        frequency: 440.0,
        attenuation: 1.0,
        phase_offset: 0.0,
        computed_sample: 0.0,
    }
}

pub fn tri(_id: usize) -> TriangleOscillator {
    TriangleOscillator {
        pitch: 0.0,
        frequency: 440.0,
        attenuation: 1.0,
        phase_offset: 0.0,
        computed_sample: 0.0,
    }
}

impl SineOscillator {
    pub fn phase(mut self, p: f32) -> Self {
        self.phase_offset = p;
        self
    }

    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p;
        self.frequency = utils::note_to_freq(p);
        self
    }

    pub fn atten(mut self, a: f32) -> Self {
        self.attenuation = a;
        self
    }

    pub fn play(mut self, signal: &mut Signal) -> Self {
        self.calculate(signal);
        signal.add_sample(self.computed_sample);
        self
    }

    pub fn run(mut self, signal: &mut Signal) -> Self {
        self.calculate(signal);
        self
    }

    fn calculate(&mut self, signal: &mut Signal) {
        let time = signal.position as f32 / signal.sample_rate as f32;
        let phase = (time * self.frequency * 2.0 * std::f32::consts::PI) + self.phase_offset;
        let sample = phase.sin() * self.attenuation;
        self.computed_sample = sample;
    }

    pub fn output(self) -> f32 {
        self.computed_sample
    }
}

impl SquareOscillator {
    pub fn phase(mut self, p: f32) -> Self {
        self.phase_offset = p;
        self
    }

    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p;
        self.frequency = utils::note_to_freq(p);
        self
    }

    pub fn atten(mut self, a: f32) -> Self {
        self.attenuation = a;
        self
    }

    pub fn play(mut self, signal: &mut Signal) -> Self {
        self.calculate(signal);
        signal.add_sample(self.computed_sample);
        self
    }

    pub fn run(mut self, signal: &mut Signal) -> Self {
        self.calculate(signal);
        self
    }

    fn calculate(&mut self, signal: &mut Signal) {
        let time = signal.position as f32 / signal.sample_rate as f32;
        let phase = (time * self.frequency * 2.0 * std::f32::consts::PI) + self.phase_offset;
        let sample = if phase.sin() >= 0.0 { 1.0 } else { -1.0 } * self.attenuation;
        self.computed_sample = sample;
    }

    pub fn output(self) -> f32 {
        self.computed_sample
    }
}

impl TriangleOscillator {
    pub fn phase(mut self, p: f32) -> Self {
        self.phase_offset = p;
        self
    }

    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch = p;
        self.frequency = utils::note_to_freq(p);
        self
    }

    pub fn atten(mut self, a: f32) -> Self {
        self.attenuation = a;
        self
    }

    pub fn play(mut self, signal: &mut Signal) -> Self {
        self.calculate(signal);
        signal.add_sample(self.computed_sample);
        self
    }

    pub fn run(mut self, signal: &mut Signal) -> Self {
        self.calculate(signal);
        self
    }

    fn calculate(&mut self, signal: &mut Signal) {
        let time = signal.position as f32 / signal.sample_rate as f32;
        let phase = ((time * self.frequency + self.phase_offset / (2.0 * std::f32::consts::PI))
            % 1.0)
            * 2.0
            * std::f32::consts::PI;

        let sample = if phase < std::f32::consts::PI {
            -1.0 + (2.0 * phase / std::f32::consts::PI)
        } else {
            3.0 - (2.0 * phase / std::f32::consts::PI)
        } * self.attenuation;

        self.computed_sample = sample;
    }

    pub fn output(self) -> f32 {
        self.computed_sample
    }
}
