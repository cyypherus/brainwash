use crate::sample::Sample;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distortion {
    Clip,
    Tanh,
    Fold,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Drive(f32);

impl Drive {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value >= 0.0).then_some(Self(value))
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

impl Distortion {
    pub fn process(self, input: Sample) -> Sample {
        let driven = input.value();
        let value = match self {
            Distortion::Clip => driven.clamp(-1.0, 1.0),
            Distortion::Tanh => driven.tanh(),
            Distortion::Fold => ((driven + 1.0).rem_euclid(4.0) - 2.0).abs() - 1.0,
        };
        Sample::raw(value)
    }
}
