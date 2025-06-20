use brainwash::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_sequence_basic_usage(c: &mut Criterion) {
    c.bench_function("sequence_basic_usage", |b| {
        let mut seq = seq(
            0,
            vec![chord(&[60, 64, 67]), note(62), chord(&[59, 62, 66]), rest()],
        );
        let mut signal = Signal::new(44100);
        let mut position = 0.0;

        b.iter(|| {
            position += 0.1;
            if position > 1.0 {
                position = 0.0;
            }
            let keys = seq.output(black_box(position), &mut signal);
            black_box(keys);
        });
    });
}

criterion_group!(benches, bench_sequence_basic_usage);
criterion_main!(benches);
