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
        let seq = seq!([
            chord(&[0, 4, 7]),
            chord(&[5, 9, 12]),
            chord(&[7, 11, 14]),
            chord(&[9, 7, 11, 16]),
        ])
        .tempo(100.)
        .bars(1)
        .output(s);
        // for key in seq {
        let key = seq.first().unwrap();
        let env = adsr!().att(0.1).sus(1.).output(key.on, key.note, s);

        s.graph_with_window("envelope", env, 100000);

        tri!()
            .pitch(key.pitch + 12. + (env))
            .atten(env)
            .play(s)
            .output();

        // s.graph("t", t);
        // }
    });
}
