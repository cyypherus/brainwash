use brainwash::compile::{CompiledPatch, PatchEngine};
use brainwash::patch::{Module, Patch, Resonance};
use brainwash::time::{Hertz, SampleRate};

fn main() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut patch = Patch::new();
    let osc = patch.insert(brainwash::preset::sine(Hertz::new(440.0).unwrap()));
    let filter = patch.insert(Module::Filter {
        input: brainwash::sample::Sample::ZERO,
        cutoff: Hertz::new(1_000.0).unwrap(),
        resonance: Resonance::new(0.707).unwrap(),
    });
    patch
        .connect(patch.output_port(osc, 0).unwrap(), filter)
        .unwrap();
    patch.output(patch.output_port(filter, 0).unwrap()).unwrap();
    let compiled = CompiledPatch::new(&patch, rate).unwrap();
    let mut engine = PatchEngine::new(compiled);
    let frame = engine.next();
    println!("{}", frame.left().value());
}
