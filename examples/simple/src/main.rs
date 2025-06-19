use signal::*;

fn main() {
    println!("Starting live synthesizer...");

    if let Err(e) = play_live(synth) {
        eprintln!("Error with live audio: {}", e);
    }
    // save_wav(synth, "hi.wav");
}

fn synth(s: &mut Signal) {
    let sequence = sequence([chord(&[0, 4, 7]), chord(&[7, 11, 14]), chord(&[8, 10, 12])])
        .tempo(90.)
        .output(s);

    for (pitch, note) in sequence {
        let env = adsr(a(time(0.01)), ds(vol(0.8), time(1.)), r(time(2.))).output(note, s);
        sin().phase(0.).pitch(pitch).atten(env).play(s);
        // let sq: f32 = square().play(s).output();
        // triangle().pitch(pitch + sq).play(s).output();
    }
}
