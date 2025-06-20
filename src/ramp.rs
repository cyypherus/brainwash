use crate::Signal;

pub(crate) struct RampState {
    pub(crate) current_value: f32,
    pub(crate) target_value: f32,
    pub(crate) start_value: f32,
    pub(crate) start_time: Option<usize>,
}

pub struct Ramp {
    id: usize,
    index: usize,
    ramp_time: f32,
    target: f32,
}

pub fn ramp(id: usize) -> Ramp {
    Ramp {
        id,
        index: 0,
        ramp_time: 0.1,
        target: 0.0,
    }
}

impl Ramp {
    pub fn time(mut self, seconds: f32) -> Self {
        self.ramp_time = seconds.max(0.001);
        self
    }

    pub fn value(mut self, target: f32) -> Self {
        self.target = target;
        self
    }

    pub fn index(mut self, id: usize) -> Self {
        self.index = id;
        self
    }

    pub fn output(self, signal: &mut Signal) -> f32 {
        let current_time = signal.position;
        let sample_rate = signal.sample_rate as f32;
        let state = signal.get_ramp_state(self.id as i32, self.index as i32);

        if (state.target_value - self.target).abs() > f32::EPSILON {
            state.start_value = state.current_value;
            state.target_value = self.target;
            state.start_time = Some(current_time);
        }

        if let Some(start_time) = state.start_time {
            let elapsed = (current_time - start_time) as f32 / sample_rate;

            if elapsed >= self.ramp_time {
                state.current_value = state.target_value;
                state.start_time = None;
            } else {
                let progress = elapsed / self.ramp_time;
                state.current_value =
                    state.start_value + (state.target_value - state.start_value) * progress;
            }
        }

        state.current_value
    }
}
