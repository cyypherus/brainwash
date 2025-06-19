use crate::Signal;

pub(crate) struct ClockState {
    position: usize,
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
        let state = signal
            .clock_state
            .entry(self.id as i32)
            .or_insert(ClockState { position: 0 });

        let beats_per_second = self.bpm / 60.0;
        let samples_per_beat = signal.sample_rate as f32 / beats_per_second;
        let samples_per_bar = samples_per_beat * 4.0;

        let position_in_bar = (state.position as f32) % samples_per_bar;
        state.position += 1;

        position_in_bar / samples_per_bar
    }
}
