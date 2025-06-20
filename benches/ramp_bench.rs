use brainwash::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_ramp_basic_usage(c: &mut Criterion) {
    c.bench_function("ramp_basic_usage", |b| {
        let mut signal = Signal::new(44100);
        let mut target = 0.0;
        b.iter(|| {
            target += 0.1;
            if target > 10.0 {
                target = 0.0;
            }
            let ramper = ramp(0).time(black_box(0.5)).to(black_box(target));
            let output = ramper.output(&mut signal);
            signal.advance();
            black_box(output);
        });
    });
}

criterion_group!(benches, bench_ramp_basic_usage);
criterion_main!(benches);
