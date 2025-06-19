use dioxus_devtools::{connect_subsecond, subsecond};
use signal::*;

fn main() {
    connect_subsecond();
    println!("Starting live synthesizer...");
    if let Err(e) = play_live(synth) {
        eprintln!("Error with live audio: {}", e);
    }
}

fn synth(s: &mut Signal) {
    subsecond::call(|| {
        let seq = sequence([
            chord(&[0, 4, 7]),
            chord(&[5, 9, 12]),
            chord(&[7, 11, 14]),
            chord(&[0, 4, 7]),
        ])
        .tempo(40.)
        .output(s);
        for (pitch, note) in seq {
            let env = adsr(0.01, 0.4, 0.1, 0.).output(note, s);
            tri().pitch(pitch - 12.).atten(env).play(s).output();
            sin().pitch(pitch + 12.).play(s).output();
        }
    });
}
