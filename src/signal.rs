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

    pub fn reset(&mut self) {
        self.position = 0;
        self.current_sample = 0.0;
    }

    pub fn get_current_sample(&self) -> f32 {
        self.current_sample
    }

    pub fn set_global_volume(&mut self, volume: f32) {
        self.global_volume = volume.max(0.0);
    }

    pub fn get_global_volume(&self) -> f32 {
        self.global_volume
    }

    pub fn get_time_seconds(&self) -> f32 {
        self.position as f32 / self.sample_rate as f32
    }
}
