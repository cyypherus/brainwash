mod audio;
mod envelopes;
mod oscillators;
mod sequencing;

pub use audio::*;
pub use envelopes::*;
pub use oscillators::*;
pub use sequencing::*;

#[derive(Debug)]
pub struct Signal {
    pub current_sample: f32,
    pub sample_rate: usize,
    pub position: usize,
    pub pitch_triggers: [(bool, u32); 128],
}

impl Signal {
    pub fn new(sample_rate: usize) -> Self {
        Signal {
            current_sample: 0.0,
            sample_rate,
            position: 0,
            pitch_triggers: [(false, 0); 128],
        }
    }

    pub fn add_sample(&mut self, sample: f32) {
        self.current_sample += sample;
    }

    pub fn advance(&mut self) {
        self.position += 1;
        self.current_sample = 0.0;
    }

    pub fn reset(&mut self) {
        self.position = 0;
        self.current_sample = 0.0;
    }

    pub fn get_current_sample(&self) -> f32 {
        self.current_sample
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
