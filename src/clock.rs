use crate::{Signal, oscillators::saw};

pub fn clock(id: usize) -> Clock {
    Clock {
        id,
        bpm: 120.0,
        bars: 1.0,
    }
}

pub struct Clock {
    id: usize,
    bpm: f32,
    bars: f32,
}

impl Clock {
    pub fn bpm(mut self, bpm: f32) -> Self {
        self.bpm = bpm.max(1.0);
        self
    }

    pub fn bars(mut self, bars: f32) -> Self {
        self.bars = bars;
        self
    }

    pub fn output(self, signal: &mut Signal) -> f32 {
        let beats_per_minute = self.bpm;
        let beats_per_second = beats_per_minute / 60.0;
        let bars_per_second = beats_per_second / 4.0;
        let frequency = bars_per_second / self.bars;

        saw(self.id).freq(frequency).run(signal).output()
    }
}
