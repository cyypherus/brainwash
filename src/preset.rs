use crate::patch::{
    BinaryOp, Composition, CompressorRatio, Gain, InputKind, Module, Patch, UnaryOp,
};
use crate::sample::{Sample, Unit};
use crate::time::{Duration, Hertz, Seconds};

pub fn sine(frequency: Hertz) -> Module {
    oscillator("Sine", frequency, |patch, phase| {
        let radians = binary(patch, BinaryOp::Multiply, 0.0, std::f32::consts::TAU);
        connect(patch, phase, radians, InputKind::A);
        let sine = unary(patch, UnaryOp::Sine, 0.0);
        connect(patch, radians, sine, InputKind::In);
        sine
    })
}

pub fn square(frequency: Hertz) -> Module {
    oscillator("Square", frequency, |patch, phase| {
        let condition = binary(patch, BinaryOp::LessThan, 0.0, 0.5);
        connect(patch, phase, condition, InputKind::A);
        let output = patch.insert(Module::Switch {
            select: Sample::ZERO,
            a: Sample::new(-1.0).unwrap(),
            b: Sample::new(1.0).unwrap(),
        });
        connect(patch, condition, output, InputKind::Select);
        output
    })
}

pub fn triangle(frequency: Hertz) -> Module {
    oscillator("Triangle", frequency, |patch, phase| {
        let centered = binary(patch, BinaryOp::Subtract, 0.0, 0.5);
        connect(patch, phase, centered, InputKind::A);
        let absolute = unary(patch, UnaryOp::Absolute, 0.0);
        connect(patch, centered, absolute, InputKind::In);
        let scaled = binary(patch, BinaryOp::Multiply, 0.0, 4.0);
        connect(patch, absolute, scaled, InputKind::A);
        let output = binary(patch, BinaryOp::Subtract, 1.0, 0.0);
        connect(patch, scaled, output, InputKind::B);
        output
    })
}

pub fn saw(frequency: Hertz) -> Module {
    oscillator("Saw", frequency, |patch, phase| {
        let scaled = binary(patch, BinaryOp::Multiply, 0.0, 2.0);
        connect(patch, phase, scaled, InputKind::A);
        let output = binary(patch, BinaryOp::Subtract, 0.0, 1.0);
        connect(patch, scaled, output, InputKind::A);
        output
    })
}

pub fn reverse_saw(frequency: Hertz) -> Module {
    oscillator("Reverse Saw", frequency, |patch, phase| {
        let scaled = binary(patch, BinaryOp::Multiply, 0.0, 2.0);
        connect(patch, phase, scaled, InputKind::A);
        let output = binary(patch, BinaryOp::Subtract, 1.0, 0.0);
        connect(patch, scaled, output, InputKind::B);
        output
    })
}

fn oscillator(
    name: &'static str,
    frequency: Hertz,
    shape: impl FnOnce(&mut Patch, crate::patch::ModuleId) -> crate::patch::ModuleId,
) -> Module {
    let mut patch = Patch::new();
    let frequency_input = patch.insert(Module::Input {
        kind: InputKind::Freq,
        default: Sample::new(frequency.value()).unwrap(),
    });
    let phase = patch.insert(Module::Phase { frequency });
    connect(&mut patch, frequency_input, phase, InputKind::Freq);
    let output = shape(&mut patch, phase);
    set_output(&mut patch, output);
    let composition_output = patch.output_id().unwrap();
    Module::Composition(Box::new(
        Composition::new(
            name,
            patch,
            [("Frequency".to_string(), InputKind::Freq, frequency_input)],
            [("Output".to_string(), composition_output)],
        )
        .unwrap(),
    ))
}

pub fn transpose(semitones: Sample) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let semitones_input = input(&mut patch, InputKind::Semitones, semitones.value());
    let octaves = binary(&mut patch, BinaryOp::Divide, 0.0, 12.0);
    connect(&mut patch, semitones_input, octaves, InputKind::A);
    let ratio = binary(&mut patch, BinaryOp::Power, 2.0, 0.0);
    connect(&mut patch, octaves, ratio, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, 1.0);
    connect(&mut patch, signal, output, InputKind::A);
    connect(&mut patch, ratio, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Transpose",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Semitones", InputKind::Semitones, semitones_input),
        ],
    )
}

pub fn degree_gate(target: i32) -> Module {
    let mut patch = Patch::new();
    let degree = patch.insert(Module::Degree);
    let target_match = binary(&mut patch, BinaryOp::Equal, 0.0, target as f32);
    connect(&mut patch, degree, target_match, InputKind::A);
    let gate = patch.insert(Module::Gate);
    let gate_high = binary(&mut patch, BinaryOp::GreaterThan, 0.0, 0.5);
    connect(&mut patch, gate, gate_high, InputKind::A);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.0);
    connect(&mut patch, target_match, output, InputKind::A);
    connect(&mut patch, gate_high, output, InputKind::B);
    set_output(&mut patch, output);
    composition("Degree Gate", patch, [])
}

pub fn gain(gain: Gain) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let gain_input = input(&mut patch, InputKind::Gain, gain.value());
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, gain.value());
    connect(&mut patch, signal, output, InputKind::A);
    connect(&mut patch, gain_input, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Gain",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Gain", InputKind::Gain, gain_input),
        ],
    )
}

pub fn distortion(drive: Sample, asymmetry: Sample) -> Module {
    saturation(
        crate::patch::SaturationCurve::Tube,
        Sample::ZERO,
        drive,
        asymmetry,
    )
}

pub(crate) fn saturation(
    curve: crate::patch::SaturationCurve,
    signal: Sample,
    drive: Sample,
    asymmetry: Sample,
) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, signal.value());
    let drive_input = input(&mut patch, InputKind::Drive, drive.value());
    let asymmetry_input = input(&mut patch, InputKind::Asym, asymmetry.value());
    let offset = binary(&mut patch, BinaryOp::Add, 0.0, asymmetry.value());
    connect(&mut patch, signal, offset, InputKind::A);
    connect(&mut patch, asymmetry_input, offset, InputKind::B);
    let driven = binary(&mut patch, BinaryOp::Multiply, 0.0, drive.value());
    connect(&mut patch, offset, driven, InputKind::A);
    connect(&mut patch, drive_input, driven, InputKind::B);
    let shaped = patch.insert(match curve {
        crate::patch::SaturationCurve::Tube => tube(),
        crate::patch::SaturationCurve::Tape => tape(),
        crate::patch::SaturationCurve::Fuzz => fuzz(),
        crate::patch::SaturationCurve::Fold => fold(),
        crate::patch::SaturationCurve::Clip => clip(),
    });
    connect(&mut patch, driven, shaped, InputKind::In);
    set_output(&mut patch, shaped);
    composition(
        "Distortion",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Drive", InputKind::Drive, drive_input),
            ("Asymmetry", InputKind::Asym, asymmetry_input),
        ],
    )
}

pub fn tube() -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let positive = binary(&mut patch, BinaryOp::GreaterThan, 0.0, 0.0);
    connect(&mut patch, signal, positive, InputKind::A);
    let positive_curve = unary(&mut patch, UnaryOp::HyperbolicTangent, 0.0);
    connect(&mut patch, signal, positive_curve, InputKind::In);
    let negative_drive = binary(&mut patch, BinaryOp::Multiply, 0.0, 1.25);
    connect(&mut patch, signal, negative_drive, InputKind::A);
    let negative_curve = unary(&mut patch, UnaryOp::HyperbolicTangent, 0.0);
    connect(&mut patch, negative_drive, negative_curve, InputKind::In);
    let negative_level = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.8);
    connect(&mut patch, negative_curve, negative_level, InputKind::A);
    let output = patch.insert(Module::Switch {
        select: Sample::ZERO,
        a: Sample::ZERO,
        b: Sample::ZERO,
    });
    connect(&mut patch, positive, output, InputKind::Select);
    connect(&mut patch, negative_level, output, InputKind::A);
    connect(&mut patch, positive_curve, output, InputKind::B);
    set_output(&mut patch, output);
    composition("Tube", patch, [("Input", InputKind::In, signal)])
}

pub fn tape() -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let driven = binary(&mut patch, BinaryOp::Multiply, 0.0, 1.5);
    connect(&mut patch, signal, driven, InputKind::A);
    let curve = unary(&mut patch, UnaryOp::Arctangent, 0.0);
    connect(&mut patch, driven, curve, InputKind::In);
    let normalized = binary(&mut patch, BinaryOp::Divide, 0.0, 1.5_f32.atan());
    connect(&mut patch, curve, normalized, InputKind::A);
    let upper = binary(&mut patch, BinaryOp::Minimum, 0.0, 1.0);
    connect(&mut patch, normalized, upper, InputKind::A);
    let output = binary(&mut patch, BinaryOp::Maximum, 0.0, -1.0);
    connect(&mut patch, upper, output, InputKind::A);
    set_output(&mut patch, output);
    composition("Tape", patch, [("Input", InputKind::In, signal)])
}

pub fn fuzz() -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let sign = unary(&mut patch, UnaryOp::Sign, 0.0);
    connect(&mut patch, signal, sign, InputKind::In);
    let magnitude = unary(&mut patch, UnaryOp::Absolute, 0.0);
    connect(&mut patch, signal, magnitude, InputKind::In);
    let scaled = binary(&mut patch, BinaryOp::Multiply, 0.0, -3.0);
    connect(&mut patch, magnitude, scaled, InputKind::A);
    let exponential = unary(&mut patch, UnaryOp::Exponential, 0.0);
    connect(&mut patch, scaled, exponential, InputKind::In);
    let saturated = binary(&mut patch, BinaryOp::Subtract, 1.0, 0.0);
    connect(&mut patch, exponential, saturated, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.0);
    connect(&mut patch, sign, output, InputKind::A);
    connect(&mut patch, saturated, output, InputKind::B);
    set_output(&mut patch, output);
    composition("Fuzz", patch, [("Input", InputKind::In, signal)])
}

pub fn clip() -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let upper = binary(&mut patch, BinaryOp::Minimum, 0.0, 1.0);
    connect(&mut patch, signal, upper, InputKind::A);
    let output = binary(&mut patch, BinaryOp::Maximum, 0.0, -1.0);
    connect(&mut patch, upper, output, InputKind::A);
    set_output(&mut patch, output);
    composition("Clip", patch, [("Input", InputKind::In, signal)])
}

pub fn fold() -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let offset = binary(&mut patch, BinaryOp::Add, 0.0, 1.0);
    connect(&mut patch, signal, offset, InputKind::A);
    let wrapped = binary(&mut patch, BinaryOp::Remainder, 0.0, 4.0);
    connect(&mut patch, offset, wrapped, InputKind::A);
    let centered = binary(&mut patch, BinaryOp::Subtract, 0.0, 2.0);
    connect(&mut patch, wrapped, centered, InputKind::A);
    let magnitude = unary(&mut patch, UnaryOp::Absolute, 0.0);
    connect(&mut patch, centered, magnitude, InputKind::In);
    let output = binary(&mut patch, BinaryOp::Subtract, 0.0, 1.0);
    connect(&mut patch, magnitude, output, InputKind::A);
    set_output(&mut patch, output);
    composition("Fold", patch, [("Input", InputKind::In, signal)])
}

pub fn scale_offset(gain: Sample, offset: Sample) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let gain_input = input(&mut patch, InputKind::Gain, gain.value());
    let offset_input = input(&mut patch, InputKind::Value, offset.value());
    let scaled = binary(&mut patch, BinaryOp::Multiply, 0.0, gain.value());
    connect(&mut patch, signal, scaled, InputKind::A);
    connect(&mut patch, gain_input, scaled, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, offset.value());
    connect(&mut patch, offset_input, output, InputKind::B);
    connect(&mut patch, scaled, output, InputKind::A);
    set_output(&mut patch, output);
    composition(
        "Scale / Offset",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Gain", InputKind::Gain, gain_input),
            ("Offset", InputKind::Value, offset_input),
        ],
    )
}

pub fn envelope_follower(attack: Seconds, release: Seconds) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let attack_input = input(&mut patch, InputKind::Attack, attack.value());
    let release_input = input(&mut patch, InputKind::Release, release.value());
    let absolute = unary(&mut patch, UnaryOp::Absolute, 0.0);
    let envelope = patch.insert(Module::Slew {
        input: Sample::ZERO,
        rise: attack,
        fall: release,
    });
    connect(&mut patch, signal, absolute, InputKind::In);
    connect(&mut patch, absolute, envelope, InputKind::In);
    connect(&mut patch, attack_input, envelope, InputKind::Rise);
    connect(&mut patch, release_input, envelope, InputKind::Fall);
    set_output(&mut patch, envelope);
    composition(
        "Envelope Follower",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Attack", InputKind::Attack, attack_input),
            ("Release", InputKind::Release, release_input),
        ],
    )
}

pub fn compressor_gain(threshold: Unit, ratio: CompressorRatio) -> Module {
    let mut patch = Patch::new();
    let level = input(&mut patch, InputKind::In, 0.0);
    let threshold_input = input(&mut patch, InputKind::Thresh, threshold.value());
    let ratio_input = input(&mut patch, InputKind::Ratio, ratio.value());
    let over = binary(&mut patch, BinaryOp::Divide, 0.0, threshold.value());
    let inverse_ratio = binary(&mut patch, BinaryOp::Divide, 1.0, ratio.value());
    let exponent = binary(&mut patch, BinaryOp::Subtract, 0.0, 1.0);
    let compressed = binary(&mut patch, BinaryOp::Power, 0.0, 0.0);
    let above_threshold = binary(&mut patch, BinaryOp::GreaterThan, 0.0, threshold.value());
    let reduction = patch.insert(Module::Switch {
        select: Sample::ZERO,
        a: Sample::new(1.0).unwrap(),
        b: Sample::ZERO,
    });
    connect(&mut patch, level, over, InputKind::A);
    connect(&mut patch, threshold_input, over, InputKind::B);
    connect(&mut patch, ratio_input, inverse_ratio, InputKind::B);
    connect(&mut patch, inverse_ratio, exponent, InputKind::A);
    connect(&mut patch, over, compressed, InputKind::A);
    connect(&mut patch, exponent, compressed, InputKind::B);
    connect(&mut patch, level, above_threshold, InputKind::A);
    connect(&mut patch, threshold_input, above_threshold, InputKind::B);
    connect(&mut patch, above_threshold, reduction, InputKind::Select);
    connect(&mut patch, compressed, reduction, InputKind::B);
    set_output(&mut patch, reduction);
    composition(
        "Gain Computer",
        patch,
        [
            ("Level", InputKind::In, level),
            ("Threshold", InputKind::Thresh, threshold_input),
            ("Ratio", InputKind::Ratio, ratio_input),
        ],
    )
}

pub fn compressor(
    threshold: Unit,
    ratio: CompressorRatio,
    attack: Seconds,
    release: Seconds,
    makeup: Gain,
) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let threshold_input = input(&mut patch, InputKind::Thresh, threshold.value());
    let ratio_input = input(&mut patch, InputKind::Ratio, ratio.value());
    let attack_input = input(&mut patch, InputKind::Attack, attack.value());
    let release_input = input(&mut patch, InputKind::Release, release.value());
    let makeup_input = input(&mut patch, InputKind::Gain, makeup.value());
    let envelope = patch.insert(envelope_follower(attack, release));
    connect(&mut patch, signal, envelope, InputKind::In);
    connect(&mut patch, attack_input, envelope, InputKind::Attack);
    connect(&mut patch, release_input, envelope, InputKind::Release);
    let reduction = patch.insert(compressor_gain(threshold, ratio));
    connect(&mut patch, envelope, reduction, InputKind::In);
    connect(&mut patch, threshold_input, reduction, InputKind::Thresh);
    connect(&mut patch, ratio_input, reduction, InputKind::Ratio);
    let apply_reduction = patch.insert(gain(Gain::new(1.0).unwrap()));
    connect(&mut patch, signal, apply_reduction, InputKind::In);
    connect(&mut patch, reduction, apply_reduction, InputKind::Gain);
    let apply_makeup = patch.insert(gain(makeup));
    connect(&mut patch, apply_reduction, apply_makeup, InputKind::In);
    connect(&mut patch, makeup_input, apply_makeup, InputKind::Gain);
    set_output(&mut patch, apply_makeup);
    composition(
        "Compressor",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Threshold", InputKind::Thresh, threshold_input),
            ("Ratio", InputKind::Ratio, ratio_input),
            ("Attack", InputKind::Attack, attack_input),
            ("Release", InputKind::Release, release_input),
            ("Makeup", InputKind::Gain, makeup_input),
        ],
    )
}

pub fn adsr(attack: Unit, sustain: Unit) -> Module {
    let mut patch = Patch::new();
    let rise = input(&mut patch, InputKind::Rise, 0.0);
    let fall = input(&mut patch, InputKind::Fall, 0.0);
    let attack_input = input(&mut patch, InputKind::Attack, attack.value());
    let sustain_input = input(&mut patch, InputKind::Sustain, sustain.value());
    let attack_value = binary(&mut patch, BinaryOp::Divide, 0.0, attack.value());
    connect(&mut patch, rise, attack_value, InputKind::A);
    connect(&mut patch, attack_input, attack_value, InputKind::B);
    let elapsed_decay = binary(&mut patch, BinaryOp::Subtract, 0.0, attack.value());
    connect(&mut patch, rise, elapsed_decay, InputKind::A);
    connect(&mut patch, attack_input, elapsed_decay, InputKind::B);
    let decay_length = binary(&mut patch, BinaryOp::Subtract, 1.0, attack.value());
    connect(&mut patch, attack_input, decay_length, InputKind::B);
    let decay_phase = binary(&mut patch, BinaryOp::Divide, 0.0, 1.0 - attack.value());
    connect(&mut patch, elapsed_decay, decay_phase, InputKind::A);
    connect(&mut patch, decay_length, decay_phase, InputKind::B);
    let sustain_delta = binary(&mut patch, BinaryOp::Subtract, sustain.value(), 1.0);
    connect(&mut patch, sustain_input, sustain_delta, InputKind::A);
    let decay_value = patch.insert(scale_offset(
        Sample::new(sustain.value() - 1.0).unwrap(),
        Sample::new(1.0).unwrap(),
    ));
    connect(&mut patch, decay_phase, decay_value, InputKind::In);
    connect(&mut patch, sustain_delta, decay_value, InputKind::Gain);
    let decaying = binary(&mut patch, BinaryOp::GreaterThan, 0.0, attack.value());
    connect(&mut patch, rise, decaying, InputKind::A);
    connect(&mut patch, attack_input, decaying, InputKind::B);
    let shaped = patch.insert(Module::Switch {
        select: Sample::ZERO,
        a: Sample::ZERO,
        b: Sample::ZERO,
    });
    connect(&mut patch, decaying, shaped, InputKind::Select);
    connect(&mut patch, attack_value, shaped, InputKind::A);
    connect(&mut patch, decay_value, shaped, InputKind::B);
    let release = binary(&mut patch, BinaryOp::Subtract, 1.0, 0.0);
    connect(&mut patch, fall, release, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, 1.0);
    connect(&mut patch, shaped, output, InputKind::A);
    connect(&mut patch, release, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "ADSR",
        patch,
        [
            ("Rise", InputKind::Rise, rise),
            ("Fall", InputKind::Fall, fall),
            ("Attack", InputKind::Attack, attack_input),
            ("Sustain", InputKind::Sustain, sustain_input),
        ],
    )
}

pub fn flanger(rate: Hertz, depth: Unit, feedback: Unit) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let rate_input = input(&mut patch, InputKind::Rate, rate.value());
    let depth_input = input(&mut patch, InputKind::Depth, depth.value());
    let feedback_input = input(&mut patch, InputKind::Feedback, feedback.value());
    let lfo = patch.insert(Module::Oscillator {
        waveform: crate::patch::Waveform::Sine,
        frequency: rate,
    });
    connect(&mut patch, rate_input, lfo, InputKind::Freq);
    let unipolar = patch.insert(scale_offset(
        Sample::new(0.5).unwrap(),
        Sample::new(0.5).unwrap(),
    ));
    connect(&mut patch, lfo, unipolar, InputKind::In);
    let depth_amount = binary(&mut patch, BinaryOp::Multiply, 0.0, depth.value());
    connect(&mut patch, unipolar, depth_amount, InputKind::A);
    connect(&mut patch, depth_input, depth_amount, InputKind::B);
    let delay_time = patch.insert(scale_offset(
        Sample::new(0.02495).unwrap(),
        Sample::new(0.00005).unwrap(),
    ));
    connect(&mut patch, depth_amount, delay_time, InputKind::In);
    let delay = patch.insert(Module::Delay {
        input: Sample::ZERO,
        time: Duration::Seconds(crate::time::Seconds::new(0.025).unwrap()),
        feedback,
    });
    connect(&mut patch, signal, delay, InputKind::In);
    connect(&mut patch, delay_time, delay, InputKind::Time);
    connect(&mut patch, feedback_input, delay, InputKind::Feedback);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, signal, output, InputKind::A);
    connect(&mut patch, delay, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Flanger",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Rate", InputKind::Rate, rate_input),
            ("Depth", InputKind::Depth, depth_input),
            ("Feedback", InputKind::Feedback, feedback_input),
        ],
    )
}

pub fn reverb(room: Unit, damp: Unit, modulation: Unit, diffusion: Unit) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let diffusion_input = input(&mut patch, InputKind::Diff, diffusion.value());
    let diffuser = patch.insert(reverb_diffuser(diffusion));
    connect(&mut patch, signal, diffuser, InputKind::In);
    connect(&mut patch, diffusion_input, diffuser, InputKind::Diff);
    let tank = patch.insert(reverb_tank(room, damp, modulation));
    connect(&mut patch, diffuser, tank, InputKind::In);
    connect(&mut patch, room_input, tank, InputKind::Room);
    connect(&mut patch, damp_input, tank, InputKind::Damp);
    connect(&mut patch, modulation_input, tank, InputKind::Mod);
    set_output(&mut patch, tank);
    composition(
        "Reverb",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Room", InputKind::Room, room_input),
            ("Damping", InputKind::Damp, damp_input),
            ("Modulation", InputKind::Mod, modulation_input),
            ("Diffusion", InputKind::Diff, diffusion_input),
        ],
    )
}

fn reverb_diffuser(diffusion: Unit) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let diffusion_input = input(&mut patch, InputKind::Diff, diffusion.value());
    let mut diffused = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.5);
    connect(&mut patch, signal, diffused, InputKind::A);
    for (samples, coefficient) in [(142, 0.75), (107, 0.75), (379, 0.625), (277, 0.625)] {
        let stage = patch.insert(reverb_diffuser_stage(samples, coefficient, diffusion));
        connect(&mut patch, diffused, stage, InputKind::In);
        connect(&mut patch, diffusion_input, stage, InputKind::Diff);
        diffused = stage;
    }
    set_output(&mut patch, diffused);
    composition(
        "Input Diffuser",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Diffusion", InputKind::Diff, diffusion_input),
        ],
    )
}

fn reverb_diffuser_stage(samples: usize, coefficient: f32, diffusion: Unit) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let diffusion_input = input(&mut patch, InputKind::Diff, diffusion.value());
    let scaled = binary(&mut patch, BinaryOp::Multiply, 0.0, coefficient);
    connect(&mut patch, diffusion_input, scaled, InputKind::A);
    let allpass = patch.insert(Module::Allpass {
        input: Sample::ZERO,
        time: Duration::Seconds(Seconds::new(samples as f32 / 29_761.0).unwrap()),
        feedback: Unit::new(coefficient * diffusion.value()).unwrap(),
    });
    connect(&mut patch, signal, allpass, InputKind::In);
    connect(&mut patch, scaled, allpass, InputKind::Feedback);
    set_output(&mut patch, allpass);
    composition(
        "Diffuser Stage",
        patch,
        [
            ("Input", InputKind::In, signal),
            ("Diffusion", InputKind::Diff, diffusion_input),
        ],
    )
}

fn reverb_tank(room: Unit, damp: Unit, modulation: Unit) -> Module {
    let mut patch = Patch::new();
    let diffused = input(&mut patch, InputKind::In, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let decay = patch.insert(scale_offset(
        Sample::new(0.69).unwrap(),
        Sample::new(0.3).unwrap(),
    ));
    connect(&mut patch, room_input, decay, InputKind::In);
    let tuning = [
        (672, 8.0, 0.5),
        (1572, 7.0, 0.6),
        (2356, 6.0, 0.7),
        (3163, 5.0, 0.8),
        (908, 9.0, 0.55),
        (1800, 8.0, 0.65),
        (2656, 7.0, 0.75),
        (3720, 6.0, 0.85),
    ];
    let channels = tuning.map(|(samples, _, _)| {
        let delay = patch.insert(Module::Delay {
            input: Sample::ZERO,
            time: Duration::Seconds(Seconds::new((samples + 32) as f32 / 29_761.0).unwrap()),
            feedback: Unit::ZERO,
        });
        let tap = patch.insert_delay_tap(delay, Unit::ONE).unwrap();
        (tap, delay)
    });
    let bank_taps = channels.map(|(_, delay)| patch.insert_delay_tap(delay, Unit::ONE).unwrap());
    let banks = [0, 4].map(|start| {
        let bank = patch.insert(reverb_control_bank(
            std::array::from_fn(|channel| tuning[start + channel]),
            room,
            damp,
            modulation,
        ));
        connect(&mut patch, modulation_input, bank, InputKind::Mod);
        connect(&mut patch, diffused, bank, InputKind::Diff);
        connect(&mut patch, decay, bank, InputKind::Gain);
        connect(&mut patch, damp_input, bank, InputKind::Damp);
        for channel in 0..4 {
            connect(
                &mut patch,
                bank_taps[start + channel],
                bank,
                REVERB_CHANNEL_KINDS[channel],
            );
            for (slot, kind) in [InputKind::Time, InputKind::In].into_iter().enumerate() {
                let output = patch
                    .output_port(bank, (channel * 2 + slot) as u16)
                    .unwrap();
                let input = patch.input_port(channels[start + channel].1, kind).unwrap();
                patch.connect_input(output, input).unwrap();
            }
        }
        bank
    });
    let matrix = patch.insert(reverb_matrix());
    for ((tap, _), kind) in channels.into_iter().zip(REVERB_CHANNEL_KINDS) {
        connect(&mut patch, tap, matrix, kind);
    }
    for bank in banks {
        connect(&mut patch, matrix, bank, InputKind::Feedback);
    }
    patch.output(patch.output_port(matrix, 1).unwrap()).unwrap();
    composition(
        "FDN Tank",
        patch,
        [
            ("Input", InputKind::In, diffused),
            ("Room", InputKind::Room, room_input),
            ("Modulation", InputKind::Mod, modulation_input),
            ("Damping", InputKind::Damp, damp_input),
        ],
    )
}

const REVERB_CHANNEL_KINDS: [InputKind; 8] = [
    InputKind::Channel1,
    InputKind::Channel2,
    InputKind::Channel3,
    InputKind::Channel4,
    InputKind::Channel5,
    InputKind::Channel6,
    InputKind::Channel7,
    InputKind::Channel8,
];
const REVERB_CHANNEL_LABELS: [&str; 8] = [
    "Channel 1",
    "Channel 2",
    "Channel 3",
    "Channel 4",
    "Channel 5",
    "Channel 6",
    "Channel 7",
    "Channel 8",
];
fn reverb_matrix() -> Module {
    let mut patch = Patch::new();
    let channels = REVERB_CHANNEL_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let reflection = patch.insert(reverb_reflection());
    let output = patch.insert(reverb_output_decoder());
    for (channel, kind) in channels.into_iter().zip(REVERB_CHANNEL_KINDS) {
        connect(&mut patch, channel, reflection, kind);
        connect(&mut patch, channel, output, kind);
    }
    connect(&mut patch, reflection, output, InputKind::Feedback);
    let reflection = patch.output_port(reflection, 0).unwrap();
    let output = patch.output_port(output, 0).unwrap();
    patch.output(output).unwrap();
    Module::Composition(Box::new(
        Composition::new(
            "Feedback Matrix",
            patch,
            channels.into_iter().enumerate().map(|(index, channel)| {
                (
                    REVERB_CHANNEL_LABELS[index].to_string(),
                    REVERB_CHANNEL_KINDS[index],
                    channel,
                )
            }),
            [
                ("Reflection".to_string(), reflection),
                ("Output".to_string(), output),
            ],
        )
        .unwrap(),
    ))
}

fn reverb_reflection() -> Module {
    let mut patch = Patch::new();
    let inputs = REVERB_CHANNEL_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let mut sums = inputs.to_vec();
    while sums.len() > 1 {
        sums = sums
            .chunks_exact(2)
            .map(|pair| {
                let sum = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
                connect(&mut patch, pair[0], sum, InputKind::A);
                connect(&mut patch, pair[1], sum, InputKind::B);
                sum
            })
            .collect();
    }
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, -0.25);
    connect(&mut patch, sums[0], output, InputKind::A);
    composition_outputs(
        "Reflection",
        patch,
        REVERB_CHANNEL_KINDS
            .into_iter()
            .enumerate()
            .map(|(index, kind)| (REVERB_CHANNEL_LABELS[index], kind, inputs[index])),
        [output],
    )
}

fn reverb_output_decoder() -> Module {
    let mut patch = Patch::new();
    let delayed = REVERB_CHANNEL_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let left = patch.insert(reverb_output_row(
        [
            ("Channel 1", InputKind::Channel1),
            ("Channel 3", InputKind::Channel3),
            ("Channel 5", InputKind::Channel5),
            ("Channel 7", InputKind::Channel7),
        ],
        [1.0, 1.0, -1.0, 1.0],
    ));
    let right = patch.insert(reverb_output_row(
        [
            ("Channel 2", InputKind::Channel2),
            ("Channel 4", InputKind::Channel4),
            ("Channel 6", InputKind::Channel6),
            ("Channel 8", InputKind::Channel8),
        ],
        [1.0, -1.0, 1.0, 1.0],
    ));
    for (channel, row) in [(0, left), (2, left), (4, left), (6, left)] {
        connect(
            &mut patch,
            delayed[channel],
            row,
            REVERB_CHANNEL_KINDS[channel],
        );
    }
    for (channel, row) in [(1, right), (3, right), (5, right), (7, right)] {
        connect(
            &mut patch,
            delayed[channel],
            row,
            REVERB_CHANNEL_KINDS[channel],
        );
    }
    connect(&mut patch, reflection, left, InputKind::Feedback);
    connect(&mut patch, reflection, right, InputKind::Feedback);
    let mono = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, left, mono, InputKind::A);
    connect(&mut patch, right, mono, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.25);
    connect(&mut patch, mono, output, InputKind::A);
    composition_outputs(
        "Output Decoder",
        patch,
        REVERB_CHANNEL_KINDS
            .into_iter()
            .enumerate()
            .map(|(index, kind)| (REVERB_CHANNEL_LABELS[index], kind, delayed[index]))
            .chain([("Reflection", InputKind::Feedback, reflection)]),
        [output],
    )
}

fn reverb_output_row(channels: [(&'static str, InputKind); 4], coefficients: [f32; 4]) -> Module {
    let mut patch = Patch::new();
    let delayed = channels.map(|(_, kind)| input(&mut patch, kind, 0.0));
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let pairs = [0, 2].map(|first| {
        let pair = patch.insert(reverb_weighted_output_pair(
            [channels[first], channels[first + 1]],
            [coefficients[first], coefficients[first + 1]],
        ));
        connect(&mut patch, delayed[first], pair, channels[first].1);
        connect(&mut patch, delayed[first + 1], pair, channels[first + 1].1);
        connect(&mut patch, reflection, pair, InputKind::Feedback);
        pair
    });
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, pairs[0], output, InputKind::A);
    connect(&mut patch, pairs[1], output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "FDN Output Row",
        patch,
        channels
            .into_iter()
            .enumerate()
            .map(|(channel, (label, kind))| (label, kind, delayed[channel]))
            .chain([("Reflection", InputKind::Feedback, reflection)]),
    )
}

fn reverb_weighted_output_pair(
    channels: [(&'static str, InputKind); 2],
    coefficients: [f32; 2],
) -> Module {
    let mut patch = Patch::new();
    let delayed = channels.map(|(_, kind)| input(&mut patch, kind, 0.0));
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let weighted = delayed
        .iter()
        .copied()
        .enumerate()
        .map(|(channel, delayed)| {
            let with_reflection = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
            connect(&mut patch, delayed, with_reflection, InputKind::A);
            connect(&mut patch, reflection, with_reflection, InputKind::B);
            if coefficients[channel] == 1.0 {
                with_reflection
            } else {
                let weighted = binary(&mut patch, BinaryOp::Multiply, 0.0, coefficients[channel]);
                connect(&mut patch, with_reflection, weighted, InputKind::A);
                weighted
            }
        })
        .collect::<Vec<_>>();
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, weighted[0], output, InputKind::A);
    connect(&mut patch, weighted[1], output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "FDN Weighted Pair",
        patch,
        channels
            .into_iter()
            .enumerate()
            .map(|(channel, (label, kind))| (label, kind, delayed[channel]))
            .chain([("Reflection", InputKind::Feedback, reflection)]),
    )
}

fn reverb_delay_time(samples: usize, depth: f32, rate: f32, modulation: Unit) -> Module {
    let mut patch = Patch::new();
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let oscillator = patch.insert(Module::Oscillator {
        waveform: crate::patch::Waveform::Sine,
        frequency: Hertz::new(rate).unwrap(),
    });
    let depth = binary(&mut patch, BinaryOp::Multiply, 0.0, depth / 29_761.0);
    connect(&mut patch, oscillator, depth, InputKind::A);
    let modulated = binary(&mut patch, BinaryOp::Multiply, 0.0, modulation.value());
    connect(&mut patch, depth, modulated, InputKind::A);
    connect(&mut patch, modulation_input, modulated, InputKind::B);
    let time = binary(&mut patch, BinaryOp::Add, samples as f32 / 29_761.0, 0.0);
    connect(&mut patch, modulated, time, InputKind::B);
    set_output(&mut patch, time);
    composition(
        "Delay Modulation",
        patch,
        [("Modulation", InputKind::Mod, modulation_input)],
    )
}

fn reverb_modulation_bank(tuning: [(usize, f32, f32); 4], modulation: Unit) -> Module {
    let mut patch = Patch::new();
    let control = input(&mut patch, InputKind::Mod, modulation.value());
    let outputs = tuning.map(|(samples, depth, rate)| {
        let time = patch.insert(reverb_delay_time(samples, depth, rate, modulation));
        connect(&mut patch, control, time, InputKind::Mod);
        time
    });
    composition_outputs(
        "Modulation Bank",
        patch,
        [("Modulation", InputKind::Mod, control)],
        outputs,
    )
}

fn reverb_control_bank(
    tuning: [(usize, f32, f32); 4],
    room: Unit,
    damp: Unit,
    modulation: Unit,
) -> Module {
    let mut patch = Patch::new();
    let channels = [
        InputKind::Channel1,
        InputKind::Channel2,
        InputKind::Channel3,
        InputKind::Channel4,
    ]
    .map(|kind| input(&mut patch, kind, 0.0));
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let damping = input(&mut patch, InputKind::Damp, damp.value());
    let gain = input(&mut patch, InputKind::Gain, room.value() * 0.69 + 0.3);
    let signal = input(&mut patch, InputKind::Diff, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let times = patch.insert(reverb_modulation_bank(tuning, modulation));
    connect(&mut patch, modulation_input, times, InputKind::Mod);
    let feedback = patch.insert(reverb_feedback_bank(room, damp));
    for (index, channel) in channels.into_iter().enumerate() {
        connect(&mut patch, channel, feedback, REVERB_CHANNEL_KINDS[index]);
    }
    connect(&mut patch, damping, feedback, InputKind::Damp);
    connect(&mut patch, gain, feedback, InputKind::Gain);
    connect(&mut patch, signal, feedback, InputKind::Diff);
    connect(&mut patch, reflection, feedback, InputKind::Feedback);
    let outputs = (0..4)
        .flat_map(|channel| {
            [
                (
                    format!("Time {}", channel + 1),
                    patch.output_port(times, channel).unwrap(),
                ),
                (
                    format!("Signal {}", channel + 1),
                    patch.output_port(feedback, channel).unwrap(),
                ),
            ]
        })
        .collect::<Vec<_>>();
    patch.output(outputs[0].1).unwrap();
    Module::Composition(Box::new(
        Composition::new(
            "Channel Controls",
            patch,
            channels
                .into_iter()
                .enumerate()
                .map(|(index, channel)| {
                    (
                        REVERB_CHANNEL_LABELS[index].to_string(),
                        REVERB_CHANNEL_KINDS[index],
                        channel,
                    )
                })
                .chain([
                    ("Modulation".to_string(), InputKind::Mod, modulation_input),
                    ("Damping".to_string(), InputKind::Damp, damping),
                    ("Decay".to_string(), InputKind::Gain, gain),
                    ("Input".to_string(), InputKind::Diff, signal),
                    ("Reflection".to_string(), InputKind::Feedback, reflection),
                ]),
            outputs,
        )
        .unwrap(),
    ))
}

fn reverb_feedback_bank(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let channels = [
        InputKind::Channel1,
        InputKind::Channel2,
        InputKind::Channel3,
        InputKind::Channel4,
    ]
    .map(|kind| input(&mut patch, kind, 0.0));
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let signal = input(&mut patch, InputKind::Diff, 0.0);
    let gain = input(&mut patch, InputKind::Gain, room.value() * 0.69 + 0.3);
    let damping = input(&mut patch, InputKind::Damp, damp.value());
    let outputs = channels.map(|channel| {
        let feedback = patch.insert(reverb_feedback_path(room, damp));
        connect(&mut patch, channel, feedback, InputKind::In);
        connect(&mut patch, reflection, feedback, InputKind::Feedback);
        connect(&mut patch, signal, feedback, InputKind::Diff);
        connect(&mut patch, gain, feedback, InputKind::Gain);
        connect(&mut patch, damping, feedback, InputKind::Damp);
        feedback
    });
    composition_outputs(
        "Feedback Bank",
        patch,
        channels
            .into_iter()
            .enumerate()
            .map(|(index, channel)| {
                (
                    REVERB_CHANNEL_LABELS[index],
                    REVERB_CHANNEL_KINDS[index],
                    channel,
                )
            })
            .chain([
                ("Reflection", InputKind::Feedback, reflection),
                ("Input", InputKind::Diff, signal),
                ("Decay", InputKind::Gain, gain),
                ("Damping", InputKind::Damp, damping),
            ]),
        outputs,
    )
}

fn reverb_feedback_path(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let delayed = input(&mut patch, InputKind::In, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let diffused = input(&mut patch, InputKind::Diff, 0.0);
    let gain = input(&mut patch, InputKind::Gain, room.value() * 0.69 + 0.3);
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let mixed = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, delayed, mixed, InputKind::A);
    connect(&mut patch, reflection, mixed, InputKind::B);
    let damping = patch.insert(Module::Damp {
        input: Sample::ZERO,
        coefficient: damp,
    });
    connect(&mut patch, mixed, damping, InputKind::In);
    connect(&mut patch, damp_input, damping, InputKind::Damp);
    let decayed = binary(&mut patch, BinaryOp::Multiply, 0.0, room.value());
    connect(&mut patch, damping, decayed, InputKind::A);
    connect(&mut patch, gain, decayed, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, diffused, output, InputKind::B);
    connect(&mut patch, decayed, output, InputKind::A);
    set_output(&mut patch, output);
    composition(
        "Feedback Path",
        patch,
        [
            ("Delayed", InputKind::In, delayed),
            ("Reflection", InputKind::Feedback, reflection),
            ("Damping", InputKind::Damp, damp_input),
            ("Decay", InputKind::Gain, gain),
            ("Diffused", InputKind::Diff, diffused),
        ],
    )
}

fn connect(
    patch: &mut Patch,
    from: crate::patch::ModuleId,
    to: crate::patch::ModuleId,
    input: InputKind,
) {
    let input = patch.input_port(to, input).unwrap();
    let from = patch.output_port(from, 0).unwrap();
    patch.connect_input(from, input).unwrap();
}

fn set_output(patch: &mut Patch, module: crate::patch::ModuleId) {
    let port = patch.output_port(module, 0).unwrap();
    patch.output(port).unwrap();
}

fn input(patch: &mut Patch, kind: InputKind, default: f32) -> crate::patch::ModuleId {
    patch.insert(Module::Input {
        kind,
        default: Sample::new(default).unwrap(),
    })
}

#[cfg(test)]
mod reverb_tests {
    use super::*;
    use crate::compile::{CompiledPatch, PatchControls};
    use crate::reverb::Reverb as MainReverb;
    use crate::time::SampleRate;

    #[test]
    fn composed_reverb_matches_main_fdn_response() {
        let room = 0.7;
        let damp = 0.2;
        let modulation = 0.0;
        let diffusion = 0.8;
        let rate = 29_761;
        let mut patch = Patch::new();
        let input = patch.insert(Module::Gate);
        let reverb = patch.insert(super::reverb(
            Unit::new(room).unwrap(),
            Unit::new(damp).unwrap(),
            Unit::new(modulation).unwrap(),
            Unit::new(diffusion).unwrap(),
        ));
        connect(&mut patch, input, reverb, InputKind::In);
        set_output(&mut patch, reverb);
        let mut composed = CompiledPatch::new(&patch, SampleRate::new(rate).unwrap()).unwrap();
        let mut reference = MainReverb::new(rate as f32);
        reference
            .roomsize(room)
            .damp(damp)
            .mod_depth(modulation)
            .diffusion(diffusion);
        let mut heard = false;
        for frame in 0..12_000 {
            let signal = if frame == 0 { 1.0 } else { 0.0 };
            let actual = composed
                .next_with_controls(PatchControls {
                    gate: signal,
                    ..PatchControls::default()
                })
                .left()
                .value();
            let expected = reference.output(signal);
            heard |= expected.abs() > 0.000_001;
            assert!(
                (actual - expected).abs() < 0.000_001,
                "frame {frame}: {actual} != {expected}"
            );
        }
        assert!(heard);
    }
}

fn binary(patch: &mut Patch, op: BinaryOp, a: f32, b: f32) -> crate::patch::ModuleId {
    patch.insert(Module::Binary {
        op,
        a: Sample::new(a).unwrap(),
        b: Sample::new(b).unwrap(),
    })
}

fn unary(patch: &mut Patch, op: UnaryOp, input: f32) -> crate::patch::ModuleId {
    patch.insert(Module::Unary {
        op,
        input: Sample::new(input).unwrap(),
    })
}

fn composition(
    name: &'static str,
    patch: Patch,
    inputs: impl IntoIterator<Item = (&'static str, InputKind, crate::patch::ModuleId)>,
) -> Module {
    let output_port = patch.output_id().unwrap();
    Module::Composition(Box::new(
        Composition::new(
            name,
            patch,
            inputs
                .into_iter()
                .map(|(label, kind, module)| (label.to_string(), kind, module)),
            [("Output".to_string(), output_port)],
        )
        .unwrap(),
    ))
}

fn composition_outputs(
    name: &'static str,
    mut patch: Patch,
    inputs: impl IntoIterator<Item = (&'static str, InputKind, crate::patch::ModuleId)>,
    outputs: impl IntoIterator<Item = crate::patch::ModuleId>,
) -> Module {
    let outputs = outputs
        .into_iter()
        .enumerate()
        .map(|(index, module)| {
            (
                format!("Output {}", index + 1),
                patch.output_port(module, 0).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    patch.output(outputs[0].1).unwrap();
    Module::Composition(Box::new(
        Composition::new(
            name,
            patch,
            inputs
                .into_iter()
                .map(|(label, kind, module)| (label.to_string(), kind, module)),
            outputs,
        )
        .unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::CompiledPatch;
    use crate::time::SampleRate;

    #[test]
    fn reusable_blocks_accept_connected_parameters() {
        let cases = [
            (
                scale_offset(Sample::new(1.0).unwrap(), Sample::ZERO),
                vec![
                    (InputKind::In, 0.25),
                    (InputKind::Gain, 4.0),
                    (InputKind::Value, -0.5),
                ],
                0.5,
            ),
            (
                gain(Gain::new(1.0).unwrap()),
                vec![(InputKind::In, 0.25), (InputKind::Gain, 3.0)],
                0.75,
            ),
            (
                compressor_gain(Unit::new(0.5).unwrap(), CompressorRatio::new(2.0).unwrap()),
                vec![
                    (InputKind::In, 1.0),
                    (InputKind::Thresh, 0.25),
                    (InputKind::Ratio, 2.0),
                ],
                0.5,
            ),
            (
                envelope_follower(Seconds::new(0.1).unwrap(), Seconds::new(0.1).unwrap()),
                vec![
                    (InputKind::In, -0.75),
                    (InputKind::Attack, 0.0),
                    (InputKind::Release, 0.0),
                ],
                0.75,
            ),
        ];
        for (module, controls, expected) in cases {
            let mut patch = Patch::new();
            let effect = patch.insert(module);
            for (kind, value) in controls {
                let constant = patch.insert(Module::Constant(Sample::new(value).unwrap()));
                connect(&mut patch, constant, effect, kind);
            }
            set_output(&mut patch, effect);
            let mut compiled =
                CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
            let actual = (0..32)
                .map(|_| compiled.next().left().value())
                .last()
                .unwrap();
            assert!(
                (actual - expected).abs() < 0.000001,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn adsr_is_a_composed_arithmetic_graph() {
        let mut patch = Patch::new();
        let rise = patch.insert(Module::Constant(Sample::new(0.125).unwrap()));
        let envelope = patch.insert(adsr(Unit::new(0.25).unwrap(), Unit::new(0.5).unwrap()));
        connect(&mut patch, rise, envelope, InputKind::Rise);
        set_output(&mut patch, envelope);
        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();

        assert!((compiled.next().left().value() - 0.5).abs() < 0.001);
    }

    #[test]
    fn waveform_compositions_are_phase_transforms() {
        let rate = SampleRate::new(4).unwrap();
        let frequency = Hertz::new(1.0).unwrap();
        let cases = [
            (sine(frequency), [0.0, 1.0, 0.0, -1.0]),
            (square(frequency), [1.0, 1.0, -1.0, -1.0]),
            (triangle(frequency), [-1.0, 0.0, 1.0, 0.0]),
            (saw(frequency), [-1.0, -0.5, 0.0, 0.5]),
            (reverse_saw(frequency), [1.0, 0.5, 0.0, -0.5]),
        ];

        for (waveform, expected) in cases {
            let Module::Composition(graph) = &waveform else {
                panic!()
            };
            assert!(
                graph
                    .patch()
                    .modules()
                    .iter()
                    .any(|(_, module)| matches!(module, Module::Phase { .. }))
            );
            assert_eq!(graph.inputs().len(), 1);
            assert_eq!(graph.inputs()[0].kind(), InputKind::Freq);
            let mut patch = Patch::new();
            let waveform = patch.insert(waveform);
            set_output(&mut patch, waveform);
            let mut compiled = CompiledPatch::new(&patch, rate).unwrap();
            for expected in expected {
                assert!((compiled.next().left().value() - expected).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn degree_gate_is_a_composed_degree_equality() {
        let module = degree_gate(4);
        let Module::Composition(graph) = &module else {
            panic!()
        };
        assert!(graph.patch().modules().iter().any(|(_, module)| matches!(
            module,
            Module::Binary {
                op: BinaryOp::Equal,
                ..
            }
        )));
        let mut patch = Patch::new();
        let gate = patch.insert(module);
        set_output(&mut patch, gate);
        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
        assert_eq!(
            compiled
                .next_with_controls(crate::compile::PatchControls {
                    frequency: None,
                    gate: 1.0,
                    degree: 4,
                    expression: 0.0,
                })
                .left()
                .value(),
            1.0
        );
        assert_eq!(
            compiled
                .next_with_controls(crate::compile::PatchControls {
                    frequency: None,
                    gate: 1.0,
                    degree: 5,
                    expression: 0.0,
                })
                .left()
                .value(),
            0.0
        );
    }

    #[test]
    fn flanger_is_a_composed_delay_graph() {
        let mut patch = Patch::new();
        let signal = patch.insert(Module::Constant(Sample::new(0.25).unwrap()));
        let effect = patch.insert(flanger(
            Hertz::new(0.5).unwrap(),
            Unit::new(0.5).unwrap(),
            Unit::new(0.25).unwrap(),
        ));
        connect(&mut patch, signal, effect, InputKind::In);
        set_output(&mut patch, effect);
        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();

        assert!(compiled.next().left().value().is_finite());
    }

    #[test]
    fn transfer_presets_are_composed_from_scalar_units() {
        let expected = [
            2.0_f32.tanh(),
            (3.0_f32.atan() / 1.5_f32.atan()).min(1.0),
            1.0 - (-6.0_f32).exp(),
            0.0,
            1.0,
        ];
        for (module, expected) in [tube(), tape(), fuzz(), fold(), clip()]
            .into_iter()
            .zip(expected)
        {
            let mut patch = Patch::new();
            let signal = patch.insert(Module::Constant(Sample::new(2.0).unwrap()));
            let curve = patch.insert(module);
            connect(&mut patch, signal, curve, InputKind::In);
            set_output(&mut patch, curve);
            let mut compiled =
                CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
            assert!((compiled.next().left().value() - expected).abs() < 0.001);
        }
    }
}
