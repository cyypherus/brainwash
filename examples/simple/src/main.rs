use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    // save_wav(synth, "test.wav", 5., 44100).expect("Error saving audio");
    // graph(synth).expect("Error with graph");
    play_live(synth).expect("Error with live audio");
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        s.global_volume = 0.2;
        let main_clock = clock(id!()).bpm(100.).bars(4.).output(s);
        let half_clock = clock(id!()).bpm(100.).bars(2.).output(s);
        let cmin = cmin().shift(-3);
        let sq = seq(
            id!(),
            [
                chord([cmin.note(0), cmin.note(2), cmin.note(4)]),
                chord([cmin.note(4), cmin.note(3), cmin.note(5)]),
                chord([cmin.note(2), cmin.note(4), cmin.note(6)]),
                chord([cmin.note(1), cmin.note(4), cmin.note(6)]),
            ],
        )
        .output(main_clock, s);

        let lfo1 = saw(id!()).value_at(main_clock).output().clamp(0.4, 0.7);
        let lfo2 = saw(id!()).value_at(main_clock).output();

        for key in sq {
            let env = adsr(id!())
                .index(key.index)
                .att(0.001)
                .sus(1.)
                .dec(0.1)
                .trigger(key.on)
                .output(s);
            let mut pitch = key.pitch;
            if key.on {
                pitch = ramp(id!())
                    .index(key.on_index)
                    .value(key.pitch)
                    .time(0.3)
                    .output(s);
            }
            let m = squ(id!()).index(key.index).pitch(pitch).run(s).output();
            rsaw(id!())
                .index(key.index)
                .pitch(pitch + (m * lfo2))
                .atten(env * lfo1)
                .play(s);
        }

        let lfo3 = sin(id!()).value_at(main_clock).output();
        let sq = seq(
            id!(),
            [
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
            ],
        )
        .output(lfo3, s);
        for (i, key) in sq.iter().enumerate() {
            let env = adsr(id!())
                .index(i)
                .att(0.001)
                .sus(0.)
                .trigger(key.on)
                .output(s);
            sin(id!())
                .index(i)
                .pitch(key.pitch + 12.)
                .atten(env)
                .play(s);
        }

        let kick = seq(id!(), [note(0), note(0), note(0), note(0)]).output(half_clock, s);
        for (i, key) in kick.iter().enumerate() {
            let env = adsr(id!())
                .index(i)
                .att(0.)
                .sus(0.)
                .trigger(key.on)
                .output(s);
            let m = rsaw(id!()).index(i).pitch(env).atten(env).run(s).output() * 24.;
            let m2 = rsaw(id!())
                .index(i)
                .pitch(env + 0.001)
                .atten(env)
                .run(s)
                .output()
                * 24.;
            squ(id!()).index(i).pitch(m + m2 - 24.).atten(env).play(s);
            squ(id!()).index(i).pitch(m + m2 - 36.).atten(env).play(s);
        }

        let clap = seq(
            id!(),
            [
                rest(),
                note(0),
                rest(),
                note(0),
                rest(),
                note(0),
                rest(),
                note(0),
            ],
        )
        .output(half_clock, s);
        for (i, key) in clap.iter().enumerate() {
            let env = adsr(id!())
                .index(i)
                .att(0.)
                .dec(0.1)
                .sus(0.)
                .trigger(key.on)
                .output(s);
            let m = rsaw(id!()).index(i).pitch(env).atten(env).run(s).output() * 24.;
            let m2 = rsaw(id!())
                .index(i)
                .pitch(env + 0.001)
                .atten(env)
                .run(s)
                .output()
                * 24.;
            saw(id!())
                .index(i)
                .pitch(m + 24. + m2)
                .atten(env * 0.8)
                .play(s);
            saw(id!())
                .index(i)
                .pitch(m + 12. + m2)
                .atten(env * 0.8)
                .play(s);
        }
    });
}
