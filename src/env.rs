use crate::sample::Unit;
use crate::time::{Duration, SampleRate, Samples};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdsrShape {
    attack: Duration,
    decay: Duration,
    sustain: Unit,
    release: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Adsr {
    shape: AdsrShape,
    rate: SampleRate,
    gate: Gate,
    level: Unit,
    elapsed: Samples,
}

impl AdsrShape {
    pub fn new(attack: Duration, decay: Duration, sustain: Unit, release: Duration) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
        }
    }
}

impl Adsr {
    pub fn new(shape: AdsrShape, rate: SampleRate) -> Self {
        Self {
            shape,
            rate,
            gate: Gate::Low,
            level: Unit::ZERO,
            elapsed: Samples::ZERO,
        }
    }

    pub fn set_gate(&mut self, gate: Gate) {
        if self.gate != gate {
            self.elapsed = Samples::ZERO;
        }
        self.gate = gate;
    }

    pub fn next(&mut self) -> Unit {
        let level = match self.gate {
            Gate::High => self.rise_level(),
            Gate::Low => self.fall_level(),
        };
        self.elapsed = Samples::new(self.elapsed.value() + 1);
        self.level = level;
        level
    }

    fn rise_level(self) -> Unit {
        let attack = self.shape.attack.samples(self.rate).value().max(1) as f32;
        let decay = self.shape.decay.samples(self.rate).value().max(1) as f32;
        let elapsed = self.elapsed.value() as f32;
        if elapsed < attack {
            Unit::new(elapsed / attack).unwrap_or(Unit::ONE)
        } else if elapsed < attack + decay {
            let amount = (elapsed - attack) / decay;
            let value = 1.0 + (self.shape.sustain.value() - 1.0) * amount;
            Unit::new(value).unwrap_or(self.shape.sustain)
        } else {
            self.shape.sustain
        }
    }

    fn fall_level(self) -> Unit {
        let release = self.shape.release.samples(self.rate).value().max(1) as f32;
        let amount = 1.0 - (self.elapsed.value() as f32 / release).clamp(0.0, 1.0);
        Unit::new(self.level.value() * amount).unwrap_or(Unit::ZERO)
    }
}
