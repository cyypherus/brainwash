use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    // save_wav(synth, "test.wav", 5., 44100).expect("Error saving audio");
    // graph(synth).expect("Error with graph");
    play_live(SimpleSynth::default()).expect("Error with live audio");
}

#[derive(Default)]
struct SimpleSynth {
    seq: Sequence,
    clock: Clock,
    lfo1: Osc,
    lfo2: Osc,
    mm: MultiModule<(ADSR, Osc, Osc), Ramp, 5>,
}

impl Synth for SimpleSynth {
    fn output(&mut self, s: &mut Signal) {
        subsecond::call(|| {
            s.global_volume = 0.2;
            let main_clock = self.clock.bpm(100.).bars(4.).output(s);
            let cmin = cmin().shift(-3);
            let sq = self
                .seq
                .elements([
                    chord([cmin.note(0), cmin.note(2), cmin.note(4)]),
                    chord([cmin.note(4), cmin.note(3), cmin.note(5)]),
                    chord([cmin.note(2), cmin.note(4), cmin.note(6)]),
                    chord([cmin.note(1), cmin.note(4), cmin.note(6)]),
                ])
                .output(main_clock);

            let lfo1 = self
                .lfo1
                .wave(saw())
                .value_at(main_clock)
                .output()
                .clamp(0.4, 0.7);

            let lfo2 = self.lfo2.wave(saw()).value_at(main_clock).output();

            self.mm.per_key(sq, |(adsr, osc1, osc2), ramp, key| {
                let env = adsr.att(0.001).sus(0.8).dec(0.1).trigger(key.on).output(s);
                let mut pitch = key.pitch;
                if let Some(ramp) = ramp {
                    pitch = ramp.value(key.pitch).time(0.3).output(s);
                }
                let m = osc1.wave(saw()).pitch(pitch).run(s).output();
                osc2.wave(square())
                    .pitch(pitch + (m * lfo2))
                    .atten(env * lfo1)
                    .play(s);
            });
        });
    }
}
