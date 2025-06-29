use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    // save_wav(synth, "test.wav", 5., 44100).expect("Error saving audio");
    // graph(synth).expect("Error with graph");
    let mut seq = Sequence::default();
    let mut clock = Clock::default();
    let mut mm = MultiModule::<(ADSR, Osc), (), 28>::default();
    let mut scale = cmin();
    play_live(move |s| {
        subsecond::call(|| {
            let bpm = 100.;
            let base_bars = 2.;
            let clock = clock.bpm(bpm).bars(base_bars).output(s);
            let cmin = scale.shift(-6);
            let sq = seq
                .elements(chords([
                    tri([cmin.note(0), cmin.note(2), cmin.note(4)]),
                    tri([cmin.note(-1), cmin.note(2), cmin.note(4)]),
                    tri([cmin.note(-2), cmin.note(2), cmin.note(3)]),
                    tet([cmin.note(-2), cmin.note(0), cmin.note(2), cmin.note(4)]),
                ]))
                .output(clock);
            let mut output = 0.;

            mm.per_key(sq, |(adsr1, osc3), _, key| {
                let env = adsr1.att(0.1).dec(0.1).trigger(key.on).output(s);
                output += osc3.wave(saw()).pitch(key.pitch).atten(env).output(s);
            });
            output * 0.1
        })
    })
    .expect("Error with live audio");
}
