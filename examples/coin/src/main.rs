use brainwash::*;

fn main() {
    let bpm = 240.;
    let mut clock = Clock::default();
    clock.bpm(bpm).bars(1.);
    let freq_env = Envelope::new(vec![
        point(0., 0.4),
        point(0.03, 0.4),
        point(0.04, 0.6),
        curve(0.7, 0.6),
        point(1., 0.7),
    ]);
    let env = Envelope::new(vec![
        point(0., 1.),
        curve(0.03, 0.2),
        point(0.04, 0.9),
        curve(0.3, 0.2),
        point(1., 0.),
    ]);
    let mut osc1 = Osc::default();
    let mut osc2 = Osc::default();
    let mut osc3 = Osc::default();

    play_live(move |s| {
        let mut output = 0.;
        let time = clock.output(s);
        let freq_env = freq_env.output(time);
        let env = env.output(time);
        let freq = 440. + (400. * freq_env);
        let n = osc3.noise().gain(env).output(s);
        output += osc1.sin().freq(freq).gain(env).output(s)
            + osc2.rsaw().freq(freq - 10. + n).gain(env).output(s)
            + (n * 0.1);

        output * 0.2
    })
    .expect("Error with live audio");
}
