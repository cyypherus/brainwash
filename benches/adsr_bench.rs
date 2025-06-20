use brainwash::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_adsr_basic_usage(c: &mut Criterion) {
    c.bench_function("adsr_basic_usage", |b| {
        let mut signal = Signal::new(44100);
        let mut trigger = false;
        b.iter(|| {
            trigger = !trigger;
            let envelope = adsr(0)
                .att(black_box(0.1))
                .dec(black_box(0.2))
                .sus(black_box(0.7))
                .rel(black_box(0.3))
                .trigger(black_box(trigger));
            let output = envelope.output(&mut signal);
            signal.advance();
            black_box(output);
        });
    });
}

criterion_group!(benches, bench_adsr_basic_usage);
criterion_main!(benches);
