use crate::sample::Sample;
use crate::time::{Hertz, SampleRate};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lowpass {
    rate: SampleRate,
    cutoff: Hertz,
    value: Sample,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Highpass {
    lowpass: Lowpass,
}

impl Lowpass {
    pub fn new(rate: SampleRate, cutoff: Hertz) -> Self {
        Self {
            rate,
            cutoff,
            value: Sample::ZERO,
        }
    }

    pub fn process(&mut self, input: Sample) -> Sample {
        let alpha = (self.cutoff.value() / self.rate.value() as f32).clamp(0.0, 1.0);
        self.value = self.value.add(input.sub(self.value).scale(alpha));
        self.value
    }
}

impl Highpass {
    pub fn new(rate: SampleRate, cutoff: Hertz) -> Self {
        Self {
            lowpass: Lowpass::new(rate, cutoff),
        }
    }

    pub fn process(&mut self, input: Sample) -> Sample {
        input.sub(self.lowpass.process(input))
    }
}
