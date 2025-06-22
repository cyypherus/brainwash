use crate::Signal;

pub struct Ramp {
    index: usize,
    ramp_time: f32,
    target: f32,
    pub(crate) current_value: f32,
    pub(crate) target_value: f32,
    pub(crate) start_value: f32,
    pub(crate) start_time: Option<usize>,
}

impl Default for Ramp {
    fn default() -> Self {
        Ramp {
            index: 0,
            ramp_time: 0.1,
            target: 0.0,
            current_value: 0.0,
            target_value: 0.0,
            start_value: 0.0,
            start_time: None,
        }
    }
}

impl Ramp {
    pub fn time(&mut self, seconds: f32) -> &mut Self {
        self.ramp_time = seconds.max(0.001);
        self
    }

    pub fn value(&mut self, target: f32) -> &mut Self {
        self.target = target;
        self
    }

    pub fn index(mut self, id: usize) -> Self {
        self.index = id;
        self
    }

    pub fn output(&mut self, signal: &mut Signal) -> f32 {
        let current_time = signal.position;
        let sample_rate = signal.sample_rate as f32;

        if (self.target_value - self.target).abs() > f32::EPSILON {
            self.start_value = self.current_value;
            self.target_value = self.target;
            self.start_time = Some(current_time);
        }

        if let Some(start_time) = self.start_time {
            let elapsed = (current_time - start_time) as f32 / sample_rate;

            if elapsed >= self.ramp_time {
                self.current_value = self.target_value;
                self.start_time = None;
            } else {
                let progress = elapsed / self.ramp_time;
                self.current_value =
                    self.start_value + (self.target_value - self.start_value) * progress;
            }
        }

        self.current_value
    }
}
