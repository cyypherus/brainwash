use crate::{OscillatorType, SawUp, Signal};

pub struct Clock {
    bpm: f32,
    bars: f32,
    osc: SawUp,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            bpm: 100.,
            bars: 1.,
            osc: SawUp::default(),
        }
    }
}

impl Clock {
    pub fn bpm(&mut self, bpm: f32) -> &mut Self {
        self.bpm = bpm.max(1.0);
        self
    }

    pub fn bars(&mut self, bars: f32) -> &mut Self {
        self.bars = bars;
        self
    }

    pub fn output(&mut self, signal: &mut Signal) -> f32 {
        let beats_per_minute = self.bpm;
        let beats_per_second = beats_per_minute / 60.0;
        let bars_per_second = beats_per_second / 4.0;
        let frequency = bars_per_second / self.bars;

        self.osc.freq(frequency);
        self.osc.run(signal);
        self.osc.output()
    }
}
