use crate::Signal;

pub struct Clock {
    bpm: f32,
}

pub fn clock() -> Clock {
    Clock { bpm: 120.0 }
}

impl Clock {
    pub fn bpm(mut self, bpm: f32) -> Self {
        self.bpm = bpm.max(1.0);
        self
    }

    pub fn output(&self, signal: &Signal) -> f32 {
        let beats_per_second = self.bpm / 60.0;
        let samples_per_beat = signal.sample_rate as f32 / beats_per_second;
        let samples_per_bar = samples_per_beat * 4.0;

        let position_in_bar = (signal.position as f32) % samples_per_bar;
        position_in_bar / samples_per_bar
    }

    pub fn beat(&self, signal: &Signal) -> f32 {
        let beats_per_second = self.bpm / 60.0;
        let samples_per_beat = signal.sample_rate as f32 / beats_per_second;

        let position_in_beat = (signal.position as f32) % samples_per_beat;
        position_in_beat / samples_per_beat
    }

    pub fn bar_samples(&self, sample_rate: usize) -> usize {
        let beats_per_second = self.bpm / 60.0;
        let samples_per_beat = sample_rate as f32 / beats_per_second;
        (samples_per_beat * 4.0) as usize
    }
}
