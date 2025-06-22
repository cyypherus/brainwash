use crate::Signal;

pub trait Synth: Sync + Send + 'static {
    fn output(&mut self, signal: &mut Signal);
}
