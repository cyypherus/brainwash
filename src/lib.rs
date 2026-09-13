#[global_allocator]
static A: rlsf::GlobalTlsf = rlsf::GlobalTlsf::new();

pub mod compile;
pub mod delay;
pub mod env;
pub mod live;
pub mod osc;
pub mod patch;
pub mod persist;
pub mod preset;
pub mod sample;
pub mod scale;
pub mod sequence;
pub mod time;

#[cfg(test)]
#[allow(dead_code)]
mod reverb;
