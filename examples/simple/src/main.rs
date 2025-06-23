use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    let synth = SimpleSynth::default();
    // save_wav(synth, "test.wav", 5., 44100).expect("Error saving audio");
    // graph(synth).expect("Error with graph");
    play_live(synth).expect("Error with live audio");
}

#[derive(Default)]
struct SimpleSynth {
    seq: Sequence,
    clock: Clock,
    eighth_clock: Clock,
    lfo1: Osc,
    lfo3: Osc,
    rvrb: Reverb,
    dstr: Distortion,
    mm: MultiModule<(ADSR, ADSR, Osc, Osc, Osc), Ramp, 28>,
    seq2: Sequence,
    mm2: MultiModule<(Osc, ADSR), (), 28>,
}

impl Synth for SimpleSynth {
    fn output(&mut self, s: &mut Signal) -> f32 {
        subsecond::call(|| {
            let bpm = 100.;
            let base_bars = 2.;
            let clock = self.clock.bpm(bpm).bars(base_bars).output(s);
            let sixteenths = self
                .eighth_clock
                .bpm(bpm)
                .bars(base_bars / (base_bars * 16.))
                .output(s);
            let cmin = cmin().shift(-6);
            let sq = self
                .seq
                .elements([
                    chord([cmin.note(0), cmin.note(2), cmin.note(4)]),
                    chord([cmin.note(-1), cmin.note(2), cmin.note(4)]),
                    chord([cmin.note(-2), cmin.note(2), cmin.note(3)]),
                    chord([cmin.note(-2), cmin.note(0), cmin.note(2), cmin.note(4)]),
                ])
                .output(clock);

            let lfo1 = self.lfo1.wave(saw()).output_phase(clock).clamp(0.4, 0.7);
            let lfo2 = self.lfo3.wave(sin()).bipolar().output_phase(sixteenths);
            let mut output = 0.;
            self.mm
                .per_key(sq, |(adsr1, adsr2, osc1, osc2, osc3), ramp, key| {
                    let env = adsr1.att(0.05).sus(0.7).dec(0.1).trigger(key.on).output(s);
                    let trem = adsr2
                        .att(0.5)
                        .att_curve(-0.1)
                        .sus(1.)
                        .trigger(key.on)
                        .output(s)
                        * 0.5;
                    let mut pitch = key.pitch;
                    if let Some(ramp) = ramp {
                        pitch = ramp.value(key.pitch).time(0.3).output(s);
                    }
                    let tremolo = trem * lfo2 * 0.3;
                    let m = osc1
                        .wave(saw())
                        .bipolar()
                        .pitch(pitch + 0.001 - 24.)
                        .atten(0.5)
                        .output(s);
                    output += osc2
                        .wave(square())
                        .pitch(pitch + tremolo + (m * lfo1))
                        .atten(env)
                        .output(s);
                    output += osc3
                        .wave(sin())
                        .pitch(pitch + tremolo + 24.)
                        .atten(env)
                        .output(s);
                });

            output = self.dstr.drive(0.5).gain(0.8).output(output);

            let sq = self
                .seq2
                .elements([
                    sub([
                        sub([note(cmin.note(-1)), note(cmin.note(0))]),
                        note(cmin.note(2)),
                        note(cmin.note(3)),
                        rest(),
                        rest(),
                    ]),
                    note(cmin.note(5)),
                    note(cmin.note(6)),
                    note(cmin.note(7)),
                    note(cmin.note(9)),
                    sub([
                        sub([note(cmin.note(10)), note(cmin.note(9))]),
                        note(cmin.note(8)),
                        sub([note(cmin.note(11)), note(cmin.note(12))]),
                        rest(),
                        rest(),
                    ]),
                    note(cmin.note(11)),
                    note(cmin.note(9)),
                ])
                .output(clock);

            self.mm2.per_key(sq, |(osc, adsr), _, key| {
                let env = adsr.att(0.).sus(0.).dec(0.1).trigger(key.on).output(s);
                if key.on {
                    output += osc.wave(sin()).pitch(key.pitch + 12.).atten(env).output(s);
                }
            });

            output = mix(
                output,
                self.rvrb.roomsize(0.9).damp(0.).output(output * 0.2),
                0.7,
            );

            output
        })
    }
}
