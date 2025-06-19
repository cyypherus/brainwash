use signal::*;

fn main() {
    println!("Starting live synthesizer...");

    if let Err(e) = play_live(synth) {
        eprintln!("Error with live audio: {}", e);
    }
    // save_wav(synth, "hi.wav");
}

fn synth(s: &mut Signal) {
    let sequence: Vec<f32> =
        sequence([chord(&[0, 4, 7]), chord(&[7, 11, 14]), chord(&[8, 10, 12])])
            .tempo(90.)
            .output(s);
    let env = adsr(a(vol(1.), time(0.1)), ds(vol(0.7), time(0.3)), r(time(0.1))).output(s);

    dbg!(env);
    for pitch in sequence {
        sin().phase(0.).pitch(pitch).atten(env).play(s);
        // let sq: f32 = square().play(s).output();
        // triangle().pitch(pitch + sq).play(s).output();
    }
}
