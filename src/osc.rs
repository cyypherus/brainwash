use crate::time::{Hertz, SampleRate};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase {
    rate: SampleRate,
    phase: f32,
}

impl Phase {
    pub fn new(rate: SampleRate) -> Self {
        Self { rate, phase: 0.0 }
    }

    pub fn next(&mut self, frequency: Hertz) -> f32 {
        let value = self.phase;
        self.phase += frequency.value() / self.rate.value() as f32;
        self.phase -= self.phase.floor();
        value
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Noise(u32);

impl Noise {
    pub fn new() -> Self {
        Self(0x1234_5678)
    }

    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_wraps_at_the_sample_rate() {
        let rate = SampleRate::new(4).unwrap();
        let frequency = Hertz::new(1.0).unwrap();
        let mut phase = Phase::new(rate);
        assert_eq!(phase.next(frequency), 0.0);
        assert_eq!(phase.next(frequency), 0.25);
        assert_eq!(phase.next(frequency), 0.5);
        assert_eq!(phase.next(frequency), 0.75);
        assert_eq!(phase.next(frequency), 0.0);
    }

    #[test]
    fn noise_is_continuous_bipolar_sample_generation() {
        let mut noise = Noise::new();
        let first = noise.next();
        let second = noise.next();
        assert!((-1.0..=1.0).contains(&first));
        assert!((-1.0..=1.0).contains(&second));
        assert_ne!(first, second);
    }
}
