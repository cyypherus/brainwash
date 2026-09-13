use brainwash::scale::cmin;
use brainwash::sequence::{Player, Sequence, Stroke, parse_sequence};
use brainwash::time::SampleRate;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::num::NonZeroU16;

fn bench_sequence(c: &mut Criterion) {
    let mut player = Player::new(
        parse_sequence("{(0/2/4/7)&(2/4/7/9)}", &cmin()).unwrap(),
        NonZeroU16::new(120).unwrap(),
        SampleRate::new(44_100).unwrap(),
        cmin(),
    );
    c.bench_function("sequence", |b| b.iter(|| black_box(player.advance())));
    for count in [6, 600, 6000] {
        let strokes = (0..count)
            .map(|index| {
                let start = (index / 6) as f32;
                Stroke::try_from(vec![[start, 60.0, 1.0], [start + 0.5, 60.0, 1.0]]).unwrap()
            })
            .collect();
        let mut player = Player::new(
            Sequence::new(strokes, (count / 6) as u16).unwrap(),
            NonZeroU16::new(120).unwrap(),
            SampleRate::new(44_100).unwrap(),
            cmin(),
        );
        c.bench_function(&format!("sequence/{count}-strokes"), |b| {
            b.iter(|| black_box(player.advance()))
        });
    }
}

criterion_group!(benches, bench_sequence);
criterion_main!(benches);
