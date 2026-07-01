use brainwash::env::Gate;
use brainwash::sample::Unit;
use brainwash::voice::{Note, NoteEvent, Track};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_track(c: &mut Criterion) {
    c.bench_function("track", |b| {
        let track = Track::new(vec![NoteEvent::new(
            Note::new(60).unwrap(),
            Gate::High,
            Unit::ONE,
        )]);
        b.iter(|| {
            black_box(track.events());
        });
    });
}

criterion_group!(benches, bench_track);
criterion_main!(benches);
