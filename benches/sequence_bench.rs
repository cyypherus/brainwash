use brainwash::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_sequence_basic_usage(c: &mut Criterion) {
    c.bench_function("sequence_basic_usage", |b| {
        let mut seq = Sequence::default();
        seq.elements(chords([
            tri([60, 64, 67]),
            note(62),
            tri([60, 64, 67]),
            rest(),
        ]));
        let mut position = 0.0;

        b.iter(|| {
            position += 0.1;
            if position > 1.0 {
                position = 0.0;
            }
            let keys = seq.output(black_box(position));
            black_box(keys);
        });
    });
}

criterion_group!(benches, bench_sequence_basic_usage);
criterion_main!(benches);
