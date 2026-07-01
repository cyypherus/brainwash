use brainwash::osc::{Oscillator, Wave};
use brainwash::time::{Hertz, SampleRate};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_oscillator(c: &mut Criterion) {
    c.bench_function("oscillator", |b| {
        let rate = SampleRate::new(44_100).unwrap();
        let mut osc = Oscillator::new(Wave::Sine, rate, Hertz::new(440.0).unwrap());
        b.iter(|| {
            black_box(osc.next());
        });
    });
}

criterion_group!(benches, bench_oscillator);
criterion_main!(benches);
