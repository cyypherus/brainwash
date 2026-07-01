use crate::sample::{Sample, Unit};
use crate::time::{Duration, SampleRate};

#[derive(Clone, Debug, PartialEq)]
pub struct Delay {
    samples: Vec<Sample>,
    index: usize,
    feedback: Unit,
}

impl Delay {
    pub fn new(rate: SampleRate, time: Duration, feedback: Unit) -> Option<Self> {
        let len = usize::try_from(time.samples(rate).value()).ok()?;
        if len == 0 {
            return None;
        }
        Some(Self {
            samples: vec![Sample::ZERO; len],
            index: 0,
            feedback,
        })
    }

    pub fn process(&mut self, input: Sample) -> Sample {
        let delayed = self.samples[self.index];
        self.samples[self.index] = input.add(delayed.attenuate(self.feedback));
        self.index = (self.index + 1) % self.samples.len();
        delayed
    }
}
