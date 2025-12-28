// #[cfg(feature = "live")]
pub mod envelopes;
pub mod filters;
pub mod instrument;
pub mod live;
pub mod oscillators;
pub mod reverbs;
pub mod scale;
pub mod track;
pub mod utils;
// #[cfg(feature = "wav")]
pub mod wav;

pub use envelopes::{Adsr, adsr, custom, lead, pad, pluck, stab};
pub use filters::{Allpass, Comb, Filter, Highpass, Lowpass, allpass, comb, highpass, lowpass};
pub use instrument::{Instrument, Key, PressState};
pub use oscillators::{Osc, Wave, noise, rsaw, saw, sin, squ, tri};
pub use reverbs::{SimpleReverb, simple_reverb};
pub use scale::{Scale, chromatic, cmaj, cmin};
pub use track::ParsedTrack;
pub use utils::{gain, mix};

pub type Param = fn(Key) -> f32;

// #[cfg(feature = "live")]
pub use live::play_live;
// #[cfg(feature = "wav")]

pub use wav::save_wav;

pub(crate) const SAMPLE_RATE: f32 = 44100.0;

#[global_allocator]
static A: rlsf::GlobalTlsf = rlsf::GlobalTlsf::new();

#[cfg(all(debug_assertions, feature = "no-alloc"))]
#[global_allocator]
static A: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
