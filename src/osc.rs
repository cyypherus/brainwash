use crate::sample::Sample;
use crate::time::{Hertz, SampleRate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    Saw,
    ReverseSaw,
    Noise,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oscillator {
    wave: Wave,
    rate: SampleRate,
    frequency: Hertz,
    phase: f32,
    noise: u32,
}

impl Oscillator {
    pub fn new(wave: Wave, rate: SampleRate, frequency: Hertz) -> Self {
        Self {
            wave,
            rate,
            frequency,
            phase: 0.0,
            noise: 0x1234_5678,
        }
    }

    pub fn next(&mut self) -> Sample {
        let value = match self.wave {
            Wave::Sine => (self.phase * std::f32::consts::TAU).sin(),
            Wave::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Triangle => 1.0 - 4.0 * (self.phase - 0.5).abs(),
            Wave::Saw => self.phase * 2.0 - 1.0,
            Wave::ReverseSaw => 1.0 - self.phase * 2.0,
            Wave::Noise => self.noise(),
        };
        self.advance();
        Sample::raw(value)
    }

    pub fn set_frequency(&mut self, frequency: Hertz) {
        self.frequency = frequency;
    }

    fn advance(&mut self) {
        self.phase += self.frequency.value() / self.rate.value() as f32;
        self.phase -= self.phase.floor();
    }

    fn noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_saw_is_the_inverse_of_saw() {
        let rate = SampleRate::new(4).unwrap();
        let frequency = Hertz::new(1.0).unwrap();
        let mut saw = Oscillator::new(Wave::Saw, rate, frequency);
        let mut reverse = Oscillator::new(Wave::ReverseSaw, rate, frequency);
        for _ in 0..4 {
            assert!((saw.next().value() + reverse.next().value()).abs() < f32::EPSILON);
        }
    }
}
