use brainwash::env::{Adsr, AdsrShape, Gate};
use brainwash::sample::Unit;
use brainwash::time::{Duration, SampleRate, Seconds};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_adsr(c: &mut Criterion) {
    c.bench_function("adsr", |b| {
        let rate = SampleRate::new(44_100).unwrap();
        let shape = AdsrShape::new(
            Duration::Seconds(Seconds::new(0.01).unwrap()),
            Duration::Seconds(Seconds::new(0.05).unwrap()),
            Unit::new(0.7).unwrap(),
            Duration::Seconds(Seconds::new(0.1).unwrap()),
        );
        let mut adsr = Adsr::new(shape, rate);
        b.iter(|| {
            adsr.set_gate(Gate::High);
            black_box(adsr.next());
        });
    });
}

criterion_group!(benches, bench_adsr);
criterion_main!(benches);
