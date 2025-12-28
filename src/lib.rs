#[global_allocator]
static A: rlsf::GlobalTlsf = rlsf::GlobalTlsf::new();

#[cfg(all(debug_assertions, feature = "no-alloc"))]
#[global_allocator]
static A: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

mod allpass;
mod clock;
mod comb;
mod distortion;
mod envelopes;
mod filters;
#[cfg(feature = "live")]
mod live;
mod keyboard;
mod oscillators;
mod ramp;
mod reverb;
mod scale;
mod signal;
mod track;
#[cfg(feature = "tui")]
mod tui;
mod utils;
#[cfg(feature = "wav")]
mod wav;

pub use clock::*;
pub use distortion::*;
pub use envelopes::*;
pub use filters::*;
#[cfg(feature = "live")]
pub use live::*;
pub use keyboard::*;
pub use oscillators::*;
pub use ramp::*;
pub use reverb::*;
pub use scale::*;
pub use signal::*;
pub use track::*;
#[cfg(feature = "tui")]
pub use tui::*;
pub use utils::*;
#[cfg(feature = "wav")]
pub use wav::*;
