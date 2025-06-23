use crate::Signal;

pub trait Synth: Sync + Send + 'static {
    fn output(&mut self, signal: &mut Signal) -> f32;
    fn limited(&mut self, signal: &mut Signal) -> f32 {
        self.output(signal).clamp(-1., 1.)
    }
}
