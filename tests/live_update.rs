use assert_no_alloc::assert_no_alloc;
use brainwash::compile::{CompiledPatch, PatchControls, PatchEngine, UpdateRejected};
use brainwash::live::{PatchExchange, RealtimePatchEngine};
use brainwash::patch::{Module, Patch, Wave};
use brainwash::sample::Unit;
use brainwash::scale::cmin;
use brainwash::time::{Hertz, SampleRate};
use brainwash::track::Track;
use std::sync::Arc;

#[test]
fn patch_engine_plays_and_updates_patch() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut engine = PatchEngine::new(compiled_patch(Wave::Square, 110.0, rate));

    for _ in 0..16 {
        assert_frame(engine.next());
    }

    let saw = Box::new(compiled_patch(Wave::Saw, 220.0, rate));
    assert_no_alloc(|| engine.replace_boxed(saw).unwrap());
    assert!(engine.take_retired().is_none());

    for _ in 0..128 {
        assert_frame(engine.next());
    }

    let rejected = Box::new(compiled_patch(Wave::Sine, 440.0, rate));
    let Err(UpdateRejected::RetiredPatchPending(rejected)) = engine.replace_boxed(rejected) else {
        panic!("engine should retain the old patch until non-realtime code takes it");
    };
    let mut retired = None;
    assert_no_alloc(|| retired = engine.take_retired_boxed());
    assert!(retired.is_some());

    engine.replace_boxed(rejected).unwrap();

    let rejected = Box::new(compiled_patch(Wave::Triangle, 330.0, rate));
    let Err(UpdateRejected::Busy(_)) = engine.replace_boxed(rejected) else {
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
        .replace_boxed(Box::new(compiled_patch(Wave::Saw, 220.0, rate)))
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
    let exchange = Arc::new(PatchExchange::new());
    let mut engine = RealtimePatchEngine::new(
        Box::new(compiled_patch(Wave::Square, 110.0, rate)),
        Arc::clone(&exchange),
    );

    assert!(
        exchange
            .submit(Box::new(compiled_patch(Wave::Saw, 220.0, rate)))
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
            .submit(Box::new(compiled_patch(Wave::Triangle, 330.0, rate)))
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
fn patch_controls_drive_oscillator_frequency() {
    let rate = SampleRate::new(44_100).unwrap();
    let mut patch = Patch::new();
    let freq = patch.insert(Module::Freq);
    let osc = patch.insert(Module::Osc {
        wave: Wave::Saw,
        frequency: Hertz::new(110.0).unwrap(),
        shift: 0.0,
        gain: Unit::ONE,
        unipolar: false,
    });
    patch.connect(freq, osc).unwrap();
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

fn compiled_patch(wave: Wave, frequency: f32, rate: SampleRate) -> CompiledPatch {
    let mut patch = Patch::new();
    let osc = patch.insert(Module::Osc {
        wave,
        frequency: Hertz::new(frequency).unwrap(),
        shift: 0.0,
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
