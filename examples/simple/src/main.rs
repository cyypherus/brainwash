use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    let mut seq = Sequence::default();
    let mut clock = Clock::default();
    let mut mm = MultiModule::<(ADSR, Osc), Ramp, 28>::default();
    let mut scale = cmin();
    let mut reverb = Reverb::default();
    let mut lpf = LowpassFilter::default();
    // save_wav("t.wav", 4., 44100, move |s| {
    play_live(move |s| {
        subsecond::call(|| {
            let bpm = 110.;
            let base_bars = 4.;
            let clock = clock.bpm(bpm).bars(base_bars).output(s);
            let cmin = scale.shift(-6);
            let sq = seq
                .elements([
                    tri([cmin.note(0), cmin.note(2), cmin.note(4)]),
                    tri([cmin.note(-1), cmin.note(2), cmin.note(4)]),
                    tri([cmin.note(-2), cmin.note(2), cmin.note(3)]),
                    tet([cmin.note(-2), cmin.note(0), cmin.note(2), cmin.note(4)]),
                ])
                .output(clock);
            let mut output = 0.;

            mm.per_key(sq, |(adsr1, osc), rmp, key| {
                let env = adsr1
                    .att(0.3)
                    .dec(0.1)
                    .sus(0.4)
                    .rel(0.2)
                    .trigger(key.on)
                    .output(s);
                let mut pitch = key.pitch;
                if let Some(rmp) = rmp {
                    pitch = rmp.time(0.2).value(pitch).output(s);
                }

                output += osc.wave(saw()).pitch(pitch).atten(env).output(s);
            });

            output = mix(lpf.freq(0.2).output(output, s), output, 0.);

            output = mix(reverb.damp(0.1).roomsize(0.9).output(output), output, 0.3);

            output * 0.1
        })
    })
    .expect("Error with live audio");
}
