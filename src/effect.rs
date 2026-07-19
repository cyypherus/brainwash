use crate::sample::Sample;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distortion {
    Tube,
    Tape,
    Fuzz,
    Clip,
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
            Distortion::Tube => {
                if driven >= 0.0 {
                    driven.tanh()
                } else {
                    (driven * 1.25).tanh() * 0.8
                }
            }
            Distortion::Tape => ((driven * 1.5).atan() / 1.5_f32.atan()).clamp(-1.0, 1.0),
            Distortion::Fuzz => driven.signum() * (1.0 - (-3.0 * driven.abs()).exp()),
            Distortion::Clip => driven.clamp(-1.0, 1.0),
            Distortion::Fold => ((driven + 1.0).rem_euclid(4.0) - 2.0).abs() - 1.0,
        };
        Sample::raw(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distortion_types_have_distinct_transfer_functions() {
        let input = Sample::new(0.5).unwrap();
        let values = [
            Distortion::Tube,
            Distortion::Tape,
            Distortion::Fuzz,
            Distortion::Fold,
            Distortion::Clip,
        ]
        .map(|kind| kind.process(input).value());
        for (index, value) in values.iter().enumerate() {
            assert!(values[index + 1..].iter().all(|other| value != other));
        }
    }
}
