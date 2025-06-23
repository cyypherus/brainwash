pub struct Distortion {
    drive: f32,
    gain: f32,
}

impl Default for Distortion {
    fn default() -> Self {
        Self {
            drive: 1.0,
            gain: 1.0,
        }
    }
}

impl Distortion {
    pub fn drive(&mut self, amount: f32) -> &mut Self {
        self.drive = amount.clamp(0.1, 0.5);
        self
    }

    pub fn gain(&mut self, level: f32) -> &mut Self {
        self.gain = level.clamp(0.0, 1.) * 0.25;
        self
    }

    pub fn output(&self, input: f32) -> f32 {
        let input = (input * self.gain).clamp(-1., 1.);
        // (-0.5..1.5)
        let o = 1.5;
        let distorted = o * input - 0.5 * input * input * input;
        distorted
    }
}
