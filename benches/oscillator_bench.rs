use brainwash::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_oscillator_basic_usage(c: &mut Criterion) {
    c.bench_function("oscillator_basic_usage", |b| {
        let mut signal = Signal::new(44100);
        b.iter(|| {
            let osc = sin(0).pitch(black_box(60.0)).freq(black_box(440.0));
            let output = osc.run(&mut signal).output();
            signal.advance();
            black_box(output);
        });
    });
}

criterion_group!(benches, bench_oscillator_basic_usage);
criterion_main!(benches);
