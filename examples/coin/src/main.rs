use brainwash::compile::CompiledPatch;
use brainwash::patch::Patch;
use brainwash::sample::Unit;
use brainwash::time::{Hertz, SampleRate};

fn main() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut patch = Patch::new();
    let oscillator = patch.insert(brainwash::preset::square(
        Hertz::new(880.0).unwrap(),
        Unit::ONE,
        false,
    ));
    patch
        .output(patch.output_port(oscillator, 0).unwrap())
        .unwrap();
    let mut oscillator = CompiledPatch::new(&patch, rate).unwrap();
    let hit =
        brainwash::sample::Sample::new((oscillator.next().left().value() * 3.0).tanh()).unwrap();
    println!("{}", hit.clipped().value());
}
