use brainwash::effect::{Distortion, Drive};
use brainwash::osc::{Oscillator, Wave};
use brainwash::time::{Hertz, SampleRate};

fn main() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut osc = Oscillator::new(Wave::Square, rate, Hertz::new(880.0).unwrap());
    let hit = Distortion::Tanh.process(osc.next(), Drive::new(3.0).unwrap());
    println!("{}", hit.clipped().value());
}
