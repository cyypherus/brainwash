use dioxus_devtools::{connect_subsecond, subsecond};
use signal::*;

fn main() {
    connect_subsecond();
    play_live(synth).expect("Error with live audio");
    // save_wav(synth, "test.wav", 3., 44100).expect("Error saving audio");
    // graph(synth).expect("Error with graph");
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        // let seq = seq!([
        //     chord(&[0, 4, 7]),
        //     chord(&[5, 9, 12]),
        //     chord(&[7, 11, 14]),
        //     chord(&[9, 7, 11, 16]),
        // ])
        // .tempo(100.)
        // .bars(1)
        // .output(s);
        let sc = cmin();
        let seq = seq!([note(sc.note(0)), rest(), rest()])
            .tempo(100.)
            .bars(1)
            .output(s);
        for (i, key) in seq.iter().enumerate() {
            let env = adsr!().att(0.01).rel(2.).output(key.on, key.note, s);
            sin!().pitch(key.pitch).atten(env).play(s).output();
            // if i == 0 {
            s.graph(format!("t{}", i).as_str(), env, 100000);
            // }
            // if key.on {
            // s.graph("note", key.note as f32, 100000);
            // }
        }
    });
}
