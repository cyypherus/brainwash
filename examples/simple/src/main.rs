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
        let seq = seq!([
            chord(&[0, 4, 7]),
            chord(&[5, 9, 12]),
            chord(&[7, 11, 14]),
            chord(&[9, 7, 11, 16]),
        ])
        .tempo(40.)
        .output(s);
        for key in seq {
            let env = adsr!(0.01, 0.4, 0.1, 0.).output(key.on, key.note, s);
            dbg!(env);
            tri!().pitch(key.pitch - 12.).play(s).output();
            sin!().pitch(key.pitch + 12.).play(s).output();
        }
    });
}
