use brainwash::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_oscillator_basic_usage(c: &mut Criterion) {
    let mut signal = Signal::new(44100);
    c.bench_function("oscillator_sin_usage", |b| {
        b.iter(|| {
            let mut sin = Osc::default();
            let osc = sin
                .wave(Wave::Sine)
                .pitch(black_box(60.0))
                .freq(black_box(440.0));
            let output = osc.output(&mut signal);
            signal.advance();
            black_box(output);
        });
    });
    let mut signal = Signal::new(44100);
    c.bench_function("oscillator_basic_usage", |b| {
        b.iter(|| {
            let mut sin = Osc::default();
            let osc = sin
                .wave(Wave::Square)
                .pitch(black_box(60.0))
                .freq(black_box(440.0));
            let output = osc.output(&mut signal);
            signal.advance();
            black_box(output);
        });
    });
    c.bench_function("oscillator_basic_usage", |b| {
        b.iter(|| {
            let mut sin = Osc::default();
            let osc = sin
                .wave(Wave::Triangle)
                .pitch(black_box(60.0))
                .freq(black_box(440.0));
            let output = osc.output(&mut signal);
            signal.advance();
            black_box(output);
        });
    });
}

criterion_group!(benches, bench_oscillator_basic_usage);
criterion_main!(benches);
