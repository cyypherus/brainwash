use crate::{Signal, utils};

pub struct SineOscillator {
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
}

pub struct SquareOscillator {
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
}

pub struct TriangleOscillator {
    pitch: f32,
    frequency: f32,
    attenuation: f32,
    phase_offset: f32,
}

pub fn sin() -> SineOscillator {
    SineOscillator {
        pitch: 0.0,
        frequency: 440.0,
        attenuation: 1.0,
        phase_offset: 0.0,
    }
}

pub fn square() -> SquareOscillator {
    SquareOscillator {
        pitch: 0.0,
        frequency: 440.0,
        attenuation: 1.0,
        phase_offset: 0.0,
    }
}

pub fn triangle() -> TriangleOscillator {
    TriangleOscillator {
        pitch: 0.0,
        frequency: 440.0,
        attenuation: 1.0,
        phase_offset: 0.0,
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

    pub fn play(self, signal: &mut Signal) -> Self {
        let time = signal.position as f32 / signal.sample_rate as f32;
        let phase = (time * self.frequency * 2.0 * std::f32::consts::PI) + self.phase_offset;
        let sample = phase.sin() * self.attenuation;
        signal.add_sample(sample);
        self
    }

    pub fn output(self) -> f32 {
        0.0
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

    pub fn play(self, signal: &mut Signal) -> Self {
        let time = signal.position as f32 / signal.sample_rate as f32;
        let phase = (time * self.frequency * 2.0 * std::f32::consts::PI) + self.phase_offset;
        let sample = if phase.sin() >= 0.0 { 1.0 } else { -1.0 } * self.attenuation;
        signal.add_sample(sample);
        self
    }

    pub fn output(self) -> f32 {
        self.attenuation
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

    pub fn play(self, signal: &mut Signal) -> Self {
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

        signal.add_sample(sample);
        self
    }

    pub fn output(self) -> f32 {
        0.0
    }
}
