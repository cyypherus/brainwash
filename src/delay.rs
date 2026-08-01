use crate::sample::{Sample, Unit};
use crate::time::{Duration, SampleRate};

#[derive(Clone, Debug, PartialEq)]
pub struct Delay {
    samples: Vec<Sample>,
    index: usize,
    rate: SampleRate,
    default_samples: usize,
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
            rate,
            default_samples: len,
            feedback,
        })
    }

    pub fn process(&mut self, input: Sample) -> Sample {
        let delayed = self.tap(None);
        self.samples[self.index] = input.add(delayed.attenuate(self.feedback));
        self.index = (self.index + 1) % self.samples.len();
        delayed
    }

    pub(crate) fn process_at(
        &mut self,
        input: Sample,
        seconds: Option<Sample>,
        feedback: Option<Sample>,
    ) -> Sample {
        let delayed = self.tap(seconds);
        let feedback = feedback
            .and_then(|feedback| Unit::new(feedback.value()))
            .unwrap_or(self.feedback);
        self.samples[self.index] = input.add(delayed.attenuate(feedback));
        self.index = (self.index + 1) % self.samples.len();
        delayed
    }

    pub(crate) fn tap(&self, seconds: Option<Sample>) -> Sample {
        let samples = seconds
            .map(|seconds| seconds.value().max(0.0) * self.rate.value() as f32)
            .unwrap_or(self.default_samples as f32)
            .clamp(1.0, self.samples.len() as f32);
        let position = self.index as f32 - samples;
        let len = self.samples.len() as f32;
        let position = position.rem_euclid(len);
        let index = position.floor() as usize;
        let fraction = position - index as f32;
        let len = self.samples.len();
        let y0 = self.samples[(index + len - 1) % len].value();
        let y1 = self.samples[index].value();
        let y2 = self.samples[(index + 1) % len].value();
        let y3 = self.samples[(index + 2) % len].value();
        let c0 = y1;
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
        Sample::raw(((c3 * fraction + c2) * fraction + c1) * fraction + c0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_at_uses_connected_feedback_without_reallocation() {
        let mut delay = Delay::new(
            SampleRate::new(10).unwrap(),
            Duration::Samples(crate::time::Samples::new(1)),
            Unit::ZERO,
        )
        .unwrap();
        let capacity = delay.samples.capacity();

        assert_eq!(
            delay
                .process_at(Sample::raw(1.0), None, Some(Sample::raw(0.5)))
                .value(),
            0.0
        );
        assert_eq!(
            delay
                .process_at(Sample::ZERO, None, Some(Sample::raw(0.5)))
                .value(),
            1.0
        );
        assert_eq!(
            delay
                .process_at(Sample::ZERO, None, Some(Sample::raw(0.0)))
                .value(),
            0.5
        );
        assert_eq!(delay.samples.capacity(), capacity);
    }
}
