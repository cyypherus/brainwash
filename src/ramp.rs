use crate::Signal;

#[derive(Debug)]
pub struct Ramp {
    ramp_time: f32,
    new_target: f32,
    pub(crate) current_value: Option<f32>,
    pub(crate) target_value: f32,
    pub(crate) start_value: f32,
    pub(crate) start_time: Option<usize>,
}

impl Default for Ramp {
    fn default() -> Self {
        Ramp {
            ramp_time: 0.1,
            new_target: 0.0,
            current_value: None,
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
        self.new_target = target;
        self
    }

    pub fn output(&mut self, signal: &mut Signal) -> f32 {
        let current_time = signal.position;
        let sample_rate = signal.sample_rate as f32;

        let Some(ref mut current_value) = self.current_value else {
            self.current_value = Some(self.new_target);
            return self.target_value;
        };

        if (self.target_value - self.new_target).abs() > f32::EPSILON {
            self.start_value = *current_value;
            self.target_value = self.new_target;
            self.start_time = Some(current_time);
        }

        if let Some(start_time) = self.start_time {
            let elapsed = (current_time - start_time) as f32 / sample_rate;

            if elapsed >= self.ramp_time {
                *current_value = self.target_value;
                self.start_time = None;
            } else {
                let progress = elapsed / self.ramp_time;
                let new_value =
                    self.start_value + (self.target_value - self.start_value) * progress;
                *current_value = new_value;
            }
        }

        *current_value
    }

    pub fn copy_state_from(&mut self, other: &Ramp) {
        self.current_value = other.current_value;
        self.target_value = other.target_value;
        self.start_value = other.start_value;
        self.start_time = other.start_time;
        self.new_target = other.new_target;
    }
}
