use brainwash::compile::{CompiledPatch, PatchEngine};
use brainwash::osc::Wave;
use brainwash::patch::{Module, Patch, Resonance};
use brainwash::sample::Unit;
use brainwash::time::{Hertz, SampleRate};

fn main() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut patch = Patch::new();
    let osc = patch.insert(brainwash::preset::oscillator(
        Wave::Sine,
        Hertz::new(440.0).unwrap(),
        Unit::ONE,
        false,
    ));
    let filter = patch.insert(Module::Lowpass {
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
