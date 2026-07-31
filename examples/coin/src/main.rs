use brainwash::osc::{Oscillator, Wave};
use brainwash::sample::Sample;
use brainwash::time::{Hertz, SampleRate};

fn main() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut osc = Oscillator::new(Wave::Square, rate, Hertz::new(880.0).unwrap());
    let hit = Sample::new((osc.next().value() * 3.0).tanh()).unwrap();
    println!("{}", hit.clipped().value());
}
