use assert_no_alloc::assert_no_alloc;
use brainwash::compile::{CompiledPatch, PatchControls, PatchEngine, UpdateRejected};
use brainwash::live::RealtimePatchEngine;
use brainwash::patch::{InputKind, Module, Patch, Wave};
use brainwash::sample::Unit;
use brainwash::scale::cmin;
use brainwash::time::{Hertz, SampleRate};
use brainwash::track::Track;

#[test]
fn patch_engine_plays_and_updates_patch() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut engine = PatchEngine::new(compiled_patch(Wave::Square, 110.0, rate));

    for _ in 0..16 {
        assert_frame(engine.next());
    }

    let saw = compiled_patch(Wave::Saw, 220.0, rate);
    assert_no_alloc(|| engine.replace(saw).unwrap());
    assert!(engine.take_retired().is_none());

    for _ in 0..128 {
        assert_frame(engine.next());
    }

    let rejected = compiled_patch(Wave::Sine, 440.0, rate);
    let Err(UpdateRejected::RetiredPatchPending(rejected)) = engine.replace(rejected) else {
        panic!("engine should retain the old patch until non-realtime code takes it");
    };
    let mut retired = None;
    assert_no_alloc(|| retired = engine.take_retired());
    assert!(retired.is_some());

    engine.replace(rejected).unwrap();

    let rejected = compiled_patch(Wave::Triangle, 330.0, rate);
    let Err(UpdateRejected::Busy(_)) = engine.replace(rejected) else {
        panic!("engine should reject overlapping live updates");
    };

    for _ in 0..128 {
        assert_frame(engine.next());
    }

    assert!(engine.take_retired().is_some());
    assert_frame(engine.next());
}

#[test]
fn patch_engine_next_does_not_allocate_while_playing_or_updating() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut engine = PatchEngine::new(compiled_patch(Wave::Square, 110.0, rate));

    assert_no_alloc(|| {
        for _ in 0..64 {
            assert_frame(engine.next());
        }
    });

    engine
        .replace(compiled_patch(Wave::Saw, 220.0, rate))
        .unwrap();

    assert_no_alloc(|| {
        for _ in 0..128 {
            assert_frame(engine.next());
        }
    });

    assert!(engine.take_retired().is_some());

    assert_no_alloc(|| {
        for _ in 0..64 {
            assert_frame(engine.next());
        }
    });
}

#[test]
fn realtime_patch_exchange_updates_without_audio_thread_allocation() {
    let rate = SampleRate::new(44_100).unwrap();
    let (mut engine, mut exchange) =
        RealtimePatchEngine::new(compiled_patch(Wave::Square, 110.0, rate));

    assert!(
        exchange
            .submit(compiled_patch(Wave::Saw, 220.0, rate))
            .is_none()
    );

    assert_no_alloc(|| {
        for _ in 0..129 {
            assert_frame(engine.next());
        }
    });

    let retired = exchange.take_retired();
    assert!(retired.is_some());

    assert!(
        exchange
            .submit(compiled_patch(Wave::Triangle, 330.0, rate))
            .is_none()
    );

    assert_no_alloc(|| {
        for _ in 0..129 {
            assert_frame(engine.next());
        }
    });

    assert!(exchange.take_retired().is_some());
}

#[test]
fn realtime_patch_exchange_handles_retired_backpressure_without_allocation() {
    let rate = SampleRate::new(44_100).unwrap();
    let (mut engine, mut exchange) =
        RealtimePatchEngine::new(compiled_patch(Wave::Square, 110.0, rate));

    assert!(
        exchange
            .submit(compiled_patch(Wave::Saw, 220.0, rate))
            .is_none()
    );
    assert_no_alloc(|| {
        for _ in 0..129 {
            assert_frame(engine.next());
        }
    });

    assert!(
        exchange
            .submit(compiled_patch(Wave::Triangle, 330.0, rate))
            .is_none()
    );
    assert_no_alloc(|| {
        for _ in 0..129 {
            assert_frame(engine.next());
        }
    });

    assert!(
        exchange
            .submit(compiled_patch(Wave::Sine, 440.0, rate))
            .is_none()
    );
    assert_no_alloc(|| {
        for _ in 0..129 {
            assert_frame(engine.next());
        }
    });

    assert!(exchange.take_retired().is_some());
    assert!(exchange.take_retired().is_some());
    assert_no_alloc(|| assert_frame(engine.next()));
    assert!(exchange.take_retired().is_some());
}

#[test]
fn patch_controls_drive_oscillator_frequency() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut patch = Patch::new();
    let freq = patch.insert(Module::Freq);
    let osc = patch.insert(Module::Osc {
        wave: Wave::Saw,
        frequency: Hertz::new(110.0).unwrap(),
        gain: Unit::ONE,
        unipolar: false,
    });
    let port = patch.input_port(osc, InputKind::Freq).unwrap();
    patch.connect_input(freq, port).unwrap();
    patch.output(osc).unwrap();
    let mut compiled = CompiledPatch::new(&patch, rate).unwrap();

    let controls = PatchControls {
        frequency: Hertz::new(880.0),
        gate: 1.0,
        degree: 0,
    };

    assert_no_alloc(|| {
        for _ in 0..64 {
            assert_frame(compiled.next_with_controls(controls));
        }
    });
}

#[test]
fn track_play_into_does_not_allocate() {
    let mut track = Track::parse("(0/2/4/7)", &cmin()).unwrap();
    let mut events = [None; 64];

    assert_no_alloc(|| {
        let count = track.play_into(0.25, &mut events);
        assert!(count > 0);
    });
}

#[test]
fn live_realtime_source_excludes_forbidden_primitives() {
    let live = include_str!("../src/live.rs");
    for token in [
        concat!("un", "safe"),
        concat!("Unsafe", "Cell"),
        concat!("Atomic", "Ptr"),
        concat!("into", "_raw"),
        concat!("from", "_raw"),
        concat!("Mut", "ex"),
        concat!("Rw", "Lock"),
        concat!(".lo", "ck("),
        concat!("try", "_lock"),
    ] {
        assert!(!live.contains(token), "{token}");
    }
}

#[test]
fn realtime_function_bodies_exclude_allocator_shapes() {
    let live = include_str!("../src/live.rs");
    for name in [
        "fn next_with_controls",
        "fn return_retired",
        "fn accept_pending",
    ] {
        let body = function_body(live, name);
        for token in [
            "Vec::",
            "vec!",
            "Box::",
            ".collect(",
            concat!("un", "safe"),
            concat!("Mut", "ex"),
            concat!("Rw", "Lock"),
            concat!(".lo", "ck("),
            concat!("try", "_lock"),
        ] {
            assert!(!body.contains(token), "{name} {token}");
        }
    }

    let compile = include_str!("../src/compile.rs");
    for name in [
        "pub fn replace",
        "pub fn take_retired",
        "pub fn next_with_controls",
        "fn input_values",
    ] {
        let body = function_body(compile, name);
        for token in [
            "Vec::",
            "vec!",
            "Box::",
            ".collect(",
            concat!("un", "safe"),
            concat!("Mut", "ex"),
            concat!("Rw", "Lock"),
            concat!(".lo", "ck("),
            concat!("try", "_lock"),
        ] {
            assert!(!body.contains(token), "{name} {token}");
        }
    }
}

fn compiled_patch(wave: Wave, frequency: f32, rate: SampleRate) -> CompiledPatch {
    let mut patch = Patch::new();
    let osc = patch.insert(Module::Osc {
        wave,
        frequency: Hertz::new(frequency).unwrap(),
        gain: Unit::ONE,
        unipolar: false,
    });
    patch.output(osc).unwrap();
    CompiledPatch::new(&patch, rate).unwrap()
}

fn assert_frame(frame: brainwash::sample::Frame) {
    assert!(frame.left().value().is_finite());
    assert!(frame.right().value().is_finite());
}

fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
    let start = source.find(name).unwrap();
    let open = source[start..].find('{').unwrap() + start;
    let mut depth = 0usize;
    for (offset, byte) in source[open..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open..=open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("{name}");
}
