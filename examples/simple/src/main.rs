use brainwash::compile::{CompiledPatch, PatchEngine};
use brainwash::patch::{Module, Patch, Wave};
use brainwash::sample::Unit;
use brainwash::time::{Hertz, SampleRate};

fn main() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut patch = Patch::new();
    let osc = patch.insert(Module::Osc {
        wave: Wave::Sine,
        frequency: Hertz::new(440.0).unwrap(),
        gain: Unit::ONE,
        unipolar: false,
    });
    let filter = patch.insert(Module::Lowpass {
        cutoff: Hertz::new(1_000.0).unwrap(),
    });
    patch.connect(osc, filter).unwrap();
    patch.output(filter).unwrap();
    let compiled = CompiledPatch::new(&patch, rate).unwrap();
    let mut engine = PatchEngine::new(compiled);
    let frame = engine.next();
    println!("{}", frame.left().value());
}
