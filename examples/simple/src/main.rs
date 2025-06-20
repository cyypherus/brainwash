use dioxus_devtools::{connect_subsecond, subsecond};
use signal::*;

fn main() {
    connect_subsecond();
    // save_wav(synth, "test.wav", 5., 44100).expect("Error saving audio");
    graph(synth).expect("Error with graph");
    // play_live(synth).expect("Error with live audio");
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        let clock = clock!().bpm(100.).output(s);
        let cmin = cmin().shift(-3);
        let seq = seq!([
            chord(&[cmin.note(0), cmin.note(2), cmin.note(4)]),
            chord(&[cmin.note(0), cmin.note(2), cmin.note(4)]),
            chord(&[cmin.note(2), cmin.note(4), cmin.note(6)]),
            chord(&[cmin.note(2), cmin.note(4), cmin.note(6), cmin.note(8)]),
        ])
        .bars(1)
        .output(clock, s);

        let lfo1 = saw!().at_phase(clock).output().max(0.4);
        let lfo2 = saw!().at_phase(clock).output();
        for (i, key) in seq.iter().enumerate() {
            let env = adsr!()
                .index(i)
                .att(0.001)
                .sus(1.)
                .dec(0.1)
                .trigger(key.on)
                .output(s);
            let m = squ!().index(i).pitch(key.pitch).run(s).output();
            rsaw!()
                .index(i)
                .pitch(key.pitch + (m * lfo2))
                .atten(env * lfo1)
                .play(s);
        }
        let seq = seq!([
            note(cmin.note(6)),
            note(cmin.note(7)),
            rest(),
            rest(),
            note(cmin.note(9)),
            rest(),
            rest(),
            rest(),
            sub([note(cmin.note(10)), note(cmin.note(11))]),
            note(cmin.note(9)),
            rest(),
            rest(),
            note(cmin.note(7)),
            rest(),
            rest(),
            rest(),
        ])
        .bars(2)
        .output(clock, s);
        for (i, key) in seq.iter().enumerate() {
            let env = adsr!()
                .index(i)
                .att(0.001)
                .sus(0.)
                .trigger(key.on)
                .output(s);
            sin!().index(i).pitch(key.pitch + 12.).atten(env).play(s);
        }
    });
}
