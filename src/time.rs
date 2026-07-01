use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(NonZeroU32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Samples(u64);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Seconds(f32);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Hertz(f32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Duration {
    Samples(Samples),
    Seconds(Seconds),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    rate: SampleRate,
    position: Samples,
}

impl SampleRate {
    pub fn new(value: u32) -> Option<Self> {
        NonZeroU32::new(value).map(Self)
    }

    pub fn value(self) -> u32 {
        self.0.get()
    }
}

impl Samples {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

impl Seconds {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value >= 0.0).then_some(Self(value))
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

impl Hertz {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

impl Duration {
    pub fn samples(self, rate: SampleRate) -> Samples {
        match self {
            Duration::Samples(samples) => samples,
            Duration::Seconds(seconds) => {
                Samples((seconds.value() * rate.value() as f32).round().max(0.0) as u64)
            }
        }
    }
}

impl Clock {
    pub fn new(rate: SampleRate) -> Self {
        Self {
            rate,
            position: Samples::ZERO,
        }
    }

    pub fn rate(self) -> SampleRate {
        self.rate
    }

    pub fn position(self) -> Samples {
        self.position
    }

    pub fn tick(&mut self) -> Samples {
        let current = self.position;
        self.position = Samples(self.position.0 + 1);
        current
    }
}
