use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    let mut seq1 = Sequence::default();
    let mut clock = Clock::default();
    let mut bank1 = Bank::<(ADSR, Osc), Ramp, 28>::default();
    let mut scale = cmin();
    let mut reverb = Reverb::default();
    let mut lpf = LowpassFilter::default();
    let mut seq2 = Sequence::default();
    let mut bank2 = Bank::<(ADSR, Osc, Osc), (), 28>::default();
    let mut wn = Osc::default();
    let mut sin_clock = Osc::default();
    let mut tri_clock = Osc::default();

    // save_wav("t.wav", 4., 44100, move |s| {

    play_live(move |s| {
        subsecond::call(|| {
            let bpm = 110.;
            let base_bars = 4.;
            let clock = clock.bpm(bpm).bars(base_bars).output(s);
            let cmin = scale.shift(-6);
            let sq = seq1
                .elements([
                    tri([cmin.note(0), cmin.note(2), cmin.note(4)]),
                    tri([cmin.note(-1), cmin.note(2), cmin.note(4)]),
                    tri([cmin.note(-2), cmin.note(2), cmin.note(3)]),
                    tet([cmin.note(-2), cmin.note(0), cmin.note(2), cmin.note(4)]),
                ])
                .output(clock);
            let mut output = 0.;

            bank1.per_key(sq, |(adsr1, osc), rmp, key| {
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

            output = mix(lpf.freq(0.05).output(output, s), output, 0.2);

            let sq = seq2
                .elements([
                    note(cmin.note(0)),
                    note(cmin.note(2)),
                    note(cmin.note(4)),
                    note(cmin.note(5)),
                    note(cmin.note(5)),
                    note(cmin.note(4)),
                    note(cmin.note(6)),
                    note(cmin.note(2)),
                    note(cmin.note(3)),
                    note(cmin.note(-2)),
                    note(cmin.note(0)),
                    note(cmin.note(2)),
                ])
                .output(
                    (sin_clock.wave(sin()).unipolar().output_phase(clock)
                        + tri_clock
                            .wave(triangle())
                            .phase_offset(0.5)
                            .unipolar()
                            .output_phase(clock))
                        * 0.3,
                );

            let wn = wn.output(s);
            bank2.per_key(sq, |(adsr1, osc1, osc2), _, key| {
                let env = adsr1.att(0.05).dec(0.1).sus(0.4).trigger(key.on).output(s);
                let md = osc2
                    .wave(square())
                    .pitch(key.pitch + wn - 12.)
                    .phase_offset(0.5)
                    .output(s)
                    * 3.
                    * env;
                let signal = osc1
                    .wave(sin())
                    .pitch(key.pitch + 12. + md + wn)
                    .atten(env)
                    .output(s)
                    * 2.;
                output += signal;
            });

            output = mix(reverb.damp(0.1).roomsize(0.9).output(output), output, 0.5);

            output * 0.3
        })
    })
    .expect("Error with live audio");
}
