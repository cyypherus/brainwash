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
        .bars(1)
        .output(s);
        // for key in seq {
        let key = seq.first().unwrap();
        let env = adsr!()
            .att(0.02)
            .att_curve(-0.1)
            .dec(0.15)
            .dec_curve(0.2)
            .sus(0.7)
            .rel(0.4)
            .rel_curve(-0.15)
            .output(key.on, key.note, s);
        tri!()
            .pitch(key.pitch + 12. + (env * 0.1))
            .atten(env)
            .play(s)
            .output();
        // }
    });
}
