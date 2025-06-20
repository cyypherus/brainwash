mod audio;
mod clock;
mod envelopes;
mod graph;
mod oscillators;
mod sequencing;
mod signal;

pub use audio::*;
pub use brainwash_macros::*;
pub use clock::*;
pub use envelopes::*;
pub use graph::*;
pub use oscillators::*;
pub use sequencing::*;
pub use signal::*;

pub mod utils {
    pub fn midi_to_freq(note: f32) -> f32 {
        440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
    }

    pub fn note_to_freq(note: f32) -> f32 {
        midi_to_freq(note + 60.0)
    }
}
