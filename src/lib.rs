mod audio;
mod envelopes;
mod graph;
mod oscillators;
mod sequencing;

pub use audio::*;
pub use envelopes::*;
pub use graph::*;
pub use oscillators::*;
pub use sequencing::*;
pub use signal_macros::*;

use std::collections::HashMap;

pub struct Signal {
    pub current_sample: f32,
    pub sample_rate: usize,
    pub position: usize,
    pub adsr_state: HashMap<i32, ADSRState>,
}

impl Signal {
    pub fn new(sample_rate: usize) -> Self {
        Signal {
            current_sample: 0.0,
            sample_rate,
            position: 0,
            adsr_state: HashMap::new(),
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
        self.adsr_state.clear();
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
