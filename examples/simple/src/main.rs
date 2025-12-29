use brainwash::*;
use dioxus_devtools::{connect_subsecond, subsecond};

fn main() {
    connect_subsecond();
    let bpm = 110.;
    let base_bars = 4.;
    let mut clock = Clock::default();
    clock.bpm(bpm).bars(base_bars);
    let scale = cmin();
    let mut reverb = Reverb::default();
    let mut lpf1 = LowpassFilter::default();

    let track_notation = "
    {
        {0%3%5}/{1%3%5%7}/{2%4%6}/{1%3%6}
        %
        (12/13/14/(15/16+/15/14)/12)
    }
    ";
    let mut track = Track::from_notation(track_notation, &scale).expect("Failed to parse track");
    let mut keyboard: Keyboard<(ADSR, Osc)> =
        Keyboard::with_builder(|| (ADSR::default(), Osc::default()));

    play_live(move |s| {
        subsecond::call(|| {
            keyboard.update(track.play(clock.output(s)), s);

            let mut output = 0.;

            keyboard.per_key(|(adsr, osc), key| {
                let env = adsr
                    .att(0.3)
                    .dec(0.1)
                    .sus(0.4)
                    .rel(0.2)
                    .output(key.state, s);

                output += osc.saw().freq(key.freq).gain(env).output(s);
            });

            let filtered = lpf1.freq(0.05).output(output, s);
            output = mix(&[filtered, output]);

            let reverbed = reverb.damp(0.1).roomsize(0.9).output(output);
            output = mix(&[reverbed, output]);

            output * 0.3
        })
    })
    .expect("Error with live audio");
}
