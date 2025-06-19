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
        // let seq = sequence([
        //     chord(&[0, 4, 7]),
        //     chord(&[0, 4, 9]),
        //     chord(&[0, 4, 11]),
        //     chord(&[0, 4, 12]),
        // ])
        // .tempo(90.)
        // .output(s);
        // let seq = sequence([
        //     note(0),
        //     note(1),
        //     note(2),
        //     note(3),
        //     note(4),
        //     note(5),
        //     note(6),
        //     note(7),
        //     note(8),
        //     note(9),
        //     note(10),
        //     note(11),
        //     note(12),
        // ])
        // .tempo(100.)
        // .output(s);
        let seq = sequence([
            chord(&[0, 4, 7]),   // C major
            chord(&[5, 9, 12]),  // F major
            chord(&[7, 11, 14]), // G major
            chord(&[0, 4, 7]),   // C major
        ])
        .tempo(40.)
        .output(s);
        for (pitch, note) in seq {
            let env = adsr(a(time(0.01)), ds(vol(0.4), time(0.1)), r(time(0.))).output(note, s);
            // let t1 = squ().pitch(pitch).atten(0.001).run(s).output();
            // tri().pitch(pitch).phase(t1).atten(env).play(s).output();
            tri().pitch(pitch - 12.).atten(env).play(s).output();
            sin().pitch(pitch + 12.).play(s).output();
        }
    });
}
