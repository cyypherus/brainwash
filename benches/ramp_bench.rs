use brainwash::delay::Delay;
use brainwash::sample::{Sample, Unit};
use brainwash::time::{Duration, SampleRate, Samples};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_delay(c: &mut Criterion) {
    c.bench_function("delay", |b| {
        let rate = SampleRate::new(44_100).unwrap();
        let mut delay = Delay::new(
            rate,
            Duration::Samples(Samples::new(128)),
            Unit::new(0.25).unwrap(),
        )
        .unwrap();
        b.iter(|| {
            black_box(delay.process(Sample::new(0.5).unwrap()));
        });
    });
}

criterion_group!(benches, bench_delay);
criterion_main!(benches);
