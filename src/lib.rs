mod envelopes;
mod oscillators;
mod sequencing;

pub use envelopes::*;
pub use oscillators::*;
pub use sequencing::*;

#[derive(Debug, Default)]
pub struct Signal {
    pub samples: Vec<f32>,
    pub sample_rate: usize,
    pub position: usize,
}

impl Signal {
    pub fn new(sample_rate: usize) -> Self {
        Signal {
            samples: Vec::new(),
            sample_rate,
            position: 0,
        }
    }

    pub fn add_sample(&mut self, sample: f32) {
        if self.position < self.samples.len() {
            self.samples[self.position] += sample;
        } else {
            self.samples.push(sample);
        }
    }

    pub fn advance(&mut self) {
        self.position += 1;
    }

    pub fn reset(&mut self) {
        self.position = 0;
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

pub mod utils {
    pub fn midi_to_freq(note: f32) -> f32 {
        440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
    }

    pub fn note_to_freq(note: f32) -> f32 {
        midi_to_freq(note + 60.0)
    }
}

pub struct Vol(pub f32);
pub struct Time(pub f32);

pub fn vol(amount: f32) -> Vol {
    Vol(amount)
}

pub fn time(seconds: f32) -> Time {
    Time(seconds)
}
