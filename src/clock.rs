use crate::Signal;

pub(crate) struct ClockState {
    pub(crate) position: usize,
}

pub struct Clock {
    id: usize,
    bpm: f32,
}

pub fn clock(id: usize) -> Clock {
    Clock { id, bpm: 120.0 }
}

impl Clock {
    pub fn bpm(mut self, bpm: f32) -> Self {
        self.bpm = bpm.max(1.0);
        self
    }

    pub fn output(self, signal: &mut Signal) -> f32 {
        let sample_rate = signal.sample_rate;
        let state = signal.get_clock_state(self.id as i32);

        let beats_per_second = self.bpm / 60.0;
        let samples_per_beat = sample_rate as f32 / beats_per_second;
        let samples_per_bar = samples_per_beat * 4.0;

        let position_in_bar = (state.position as f32) % samples_per_bar;
        state.position += 1;

        position_in_bar / samples_per_bar
    }
}
