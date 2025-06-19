use dioxus_devtools::{connect_subsecond, subsecond};
use signal::*;

fn main() {
    connect_subsecond();
    println!("Starting live synthesizer...");
    if let Err(e) = play_live(synth) {
        eprintln!("Error with live audio: {}", e);
    }
    // save_wav(synth, "test.wav", 3., 44100).expect("oof");
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        let seq = seq!([
            chord(&[0, 4, 7]),
            chord(&[5, 9, 12]),
            chord(&[7, 11, 14]),
            chord(&[9, 7, 11, 16]),
        ])
        .tempo(100.)
        .bars(4)
        .output(s);
        for key in seq {
            let env = adsr!(0.0, 1., 1., 1.).output(key.on, key.note, s);
            tri!()
                .pitch(key.pitch + 12. + (env * 0.1))
                .atten(env)
                .play(s)
                .output();
        }
    });
}
