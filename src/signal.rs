pub struct Signal {
    pub current_sample: f32,
    pub sample_rate: usize,
    pub position: usize,
    pub global_volume: f32,
}

impl Signal {
    pub fn new(sample_rate: usize) -> Self {
        Signal {
            current_sample: 0.0,
            sample_rate,
            position: 0,
            global_volume: 1.0,
        }
    }

    pub fn add_sample(&mut self, sample: f32) {
        let val = sample * self.global_volume;
        self.current_sample += val;
        if self.current_sample > 1. {
            self.current_sample = 0.;
        }
    }

    pub fn advance(&mut self) {
        self.position += 1;
        self.current_sample = 0.0;
    }
}
