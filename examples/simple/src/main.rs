use dioxus_devtools::{connect_subsecond, subsecond};
use signal::*;

fn main() {
    connect_subsecond();
    // save_wav(synth, "test.wav", 5., 44100).expect("Error saving audio");
    // graph(synth).expect("Error with graph");
    play_live(synth).expect("Error with live audio");
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        let clock = clock!().bpm(140.).output(s);
        let cmin = cmin();
        let seq = seq!([
            chord(&[cmin.note(0), cmin.note(2), cmin.note(4)]),
            chord(&[cmin.note(1), cmin.note(3), cmin.note(5)]),
            chord(&[cmin.note(2), cmin.note(4), cmin.note(6)]),
            chord(&[cmin.note(3), cmin.note(5), cmin.note(7)]),
        ])
        .bars(1)
        .output(clock, s);

        for (i, key) in seq.iter().enumerate() {
            let env = adsr!()
                .att(0.001)
                .sus(0.)
                .dec(0.1)
                .output(key.on, key.note, s);
            let lfo = rsaw!().at_phase(clock).output();
            let m = sin!().pitch(key.pitch).atten(lfo * 0.1).run(s).output();
            saw!().pitch(key.pitch + m).atten(env).play(s);
            s.graph(format!("t{}", i).as_str(), env, 200000);
        }
    });
}
