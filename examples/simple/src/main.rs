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

    let new_track_notation = "
    {
        {0%4%7}/{2%5%9}/{4%7%11}/{5%9%12}
        %
        ((9/11/12/14)/(12/11/9/11)/(14/16/14/12)/(11/9/7/9))
    }
    {
        {7%11%14}/{9%12%16}/{11%14%18}/{12%16%19}
        %
        ((16/14/12/11)/(14/12/11/9)/(12/14/16/18)/(14/12/11/9))
    }
    {
        {4%7%11}/{5%9%12}/{7%11%14}/{9%12%16}
        %
        ((11/9/7/4)/(9/7/4/2)/(7/9/11/12)/(9/7/4/2))
    }
    {
        {0%4%7}/{2%5%9}/{4%7%11}/{5%9%12}
        %
        ((9/11/12/14)/(16/14/12/11)/(14/16/18/19)/(16/14/12/11))
    }
    ";
    let mut new_track =
        Track::from_notation(new_track_notation, &scale).expect("Failed to parse new track");
    let mut keyboard: Keyboard<(ADSR, ADSR, Osc, Osc, Osc)> = Keyboard::with_builder(|| {
        (
            ADSR::default(),
            ADSR::default(),
            Osc::default(),
            Osc::default(),
            Osc::default(),
        )
    });

    play_live(move |s| {
        subsecond::call(|| {
            keyboard.update(new_track.play(clock.output(s)), s);

            let mut output = 0.;

            keyboard.per_key(|(adsr, vibrato_env, osc1, osc2, osc3), key| {
                let env = adsr
                    .pad()
                    // .att(0.05)
                    // .dec(0.02)
                    // .sus(0.7)
                    // .rel(0.1)
                    .output(key.state, s);

                // let vibrato_depth = vibrato_env.pad().output(key.state, s);
                // let vibrato = osc2.sin().freq(5.).gain(env).output(s) * 2. * vibrato_depth;
                let thing = osc1.saw().freq(key.freq + 0.001).shift(-12.).output(s);
                output += osc3.square().freq(key.freq + thing).gain(env).output(s);
                // output += osc1
                //     .sin()
                //     .freq(key.freq)
                //     // .freq(key.freq + vibrato)
                //     .gain(env)
                //     .output(s);
                // + osc3.sin().freq(key.freq).shift(-12.).gain(env).output(s);
            });

            // let filtered = lpf1.freq(0.1).output(output, s);
            // output += filtered * 0.5;

            // let reverbed = reverb.damp(0.).roomsize(1.).output(output);
            // output += reverbed * 0.3;

            output * 0.1
        })
    })
    .expect("Error with live audio");
}
