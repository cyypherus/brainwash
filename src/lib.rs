#[global_allocator]
static A: rlsf::GlobalTlsf = rlsf::GlobalTlsf::new();

pub mod compile;
pub mod delay;
pub mod effect;
pub mod env;
pub mod filter;
pub mod live;
pub mod osc;
pub mod patch;
pub mod persist;
pub mod preset;
pub mod sample;
pub mod scale;
pub mod time;
pub mod track;
pub mod voice;

pub use scale::Scale;
