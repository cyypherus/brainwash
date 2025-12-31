use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    let bpm = 100.;
    let base_bars = 4. * 4.;
    let mut clock = Clock::default();
    clock.bpm(bpm).bars(base_bars);
    let scale = cmin();
    let mut reverb = Reverb::default();
    let mut lpf1 = LowpassFilter::default();
    let mut flng = Flanger::default();
    let mut del = Delay::default();

    let new_track_notation = "
    {
        {0&4&7}/{2&5&9}/{4&7&11}/{5&9&12}
        &
        ((9/11/12/14)/(12/11/9/11)/(14/16/14/12)/(11/9/7/9))
    }
    {
        {7&11&14}/{9&12&16}/{11&14&18}/{12&16&19}
        &
        ((16/14/12/11)/(14/12/11/9)/(12/14/16/18)/(14/12/11/9))
    }
    {
        {4&7&11}/{5&9&12}/{7&11&14}/{9&12&16}
        &
        ((11/9/7/4)/(9/7/4/2)/(7/9/11/12)/(9/7/4/2))
    }
    {
        {0&4&7}/{2&5&9}/{4&7&11}/{5&9&12}
        &
        ((9/11/12/14)/(16/14/12/11)/(14/16/18/19)/(16/14/12/11))
    }
    ";
    let mut new_track =
        Track::parse(new_track_notation, &scale).expect("Failed to parse new track");
    let mut keyboard: Keyboard<(GateRamp, GateRamp, ADSR, GateRamp, ADSR, Osc, Osc, Osc)> =
        Keyboard::with_builder(|| {
            (
                GateRamp::default(),
                GateRamp::default(),
                ADSR::default(),
                GateRamp::default(),
                ADSR::default(),
                Osc::default(),
                Osc::default(),
                Osc::default(),
            )
        });

    play_live(move |s| {
        subsecond::call(|| {
            let mut output = 0.;
            keyboard.update(new_track.play(clock.output(s)), s);
            keyboard.per_key(
                |(rise_ramp, fall_ramp, adsr, vib_rise, vibrato_env, osc1, osc2, osc3), key| {
                    let gate = if key.pressed() { 1.0 } else { 0.0 };
                    let rise = rise_ramp.rise().time(0.01).output(gate, s);
                    let fall = fall_ramp.fall().time(0.3).output(gate, s);
                    let env = adsr.stab().output(rise, fall);

                    let vib_r = vib_rise.rise().time(0.5).output(gate, s);
                    let vibrato_depth = vibrato_env.pad().output(vib_r, 0.0);
                    let vibrato = osc2.sin().freq(5.).gain(env).output(s) * 2. * vibrato_depth;
                    output += osc1.sin().freq(key.freq + vibrato).gain(env).output(s)
                        + osc3.squ().freq(key.freq).shift(-12.).gain(env).output(s);
                },
            );
            // output = flng.freq(0.05).depth(1.).feedback(0.8).output(output, s) * 0.3;

            let filtered = lpf1.freq(0.1).output(output, s);
            output += filtered * 0.5;

            let tap = del.delay(6000.).tap();
            let reverbed = reverb.damp(0.).roomsize(1.).output(tap);
            // output += reverbed * 0.3;
            output = del.output(output + (tap * 0.5));

            output * 0.1
        })
    })
    .expect("Error with live audio");
}
