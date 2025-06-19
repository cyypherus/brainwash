use dioxus_devtools::{connect_subsecond, subsecond};
use signal::*;

fn main() {
    connect_subsecond();
    // play_live(synth).expect("Error with live audio");
    // save_wav(synth, "test.wav", 3., 44100).expect("Error saving audio");
    graph(synth).expect("Error with graph");
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        let clock = clock!().bpm(100.).output(s);
        let cmin = cmin();
        let seq = seq!([
            note(cmin.note(0)),
            note(cmin.note(2)),
            note(cmin.note(1)),
            note(cmin.note(-1))
        ])
        .cycles(1)
        .output(clock, s);

        for (i, key) in seq.iter().enumerate() {
            let env = adsr!().att(0.01).rel(2.).output(key.on, key.note, s);

            sin!().pitch(key.pitch).atten(env).play(s).output();
            s.graph(format!("t{}", i).as_str(), env, 100000);
        }
    });
}
