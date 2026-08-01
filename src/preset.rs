use crate::patch::{
    BinaryOp, Composition, CompressorRatio, Gain, InputKind, Module, Patch, UnaryOp,
};
use crate::sample::{Sample, Unit};
use crate::time::{Duration, Hertz, Seconds};

pub fn sine(frequency: Hertz, gain: Unit, unipolar: bool) -> Module {
    oscillator("Sine", frequency, gain, unipolar, |patch, phase| {
        let radians = binary(patch, BinaryOp::Multiply, 0.0, std::f32::consts::TAU);
        connect(patch, phase, radians, InputKind::A);
        let sine = patch.insert(Module::Unary(UnaryOp::Sine));
        connect(patch, radians, sine, InputKind::In);
        sine
    })
}

pub fn square(frequency: Hertz, gain: Unit, unipolar: bool) -> Module {
    oscillator("Square", frequency, gain, unipolar, |patch, phase| {
        let condition = binary(patch, BinaryOp::LessThan, 0.0, 0.5);
        connect(patch, phase, condition, InputKind::A);
        let output = patch.insert(Module::Switch {
            a: Sample::new(-1.0).unwrap(),
            b: Sample::new(1.0).unwrap(),
        });
        connect(patch, condition, output, InputKind::Select);
        output
    })
}

pub fn triangle(frequency: Hertz, gain: Unit, unipolar: bool) -> Module {
    oscillator("Triangle", frequency, gain, unipolar, |patch, phase| {
        let centered = binary(patch, BinaryOp::Subtract, 0.0, 0.5);
        connect(patch, phase, centered, InputKind::A);
        let absolute = patch.insert(Module::Unary(UnaryOp::Absolute));
        connect(patch, centered, absolute, InputKind::In);
        let scaled = binary(patch, BinaryOp::Multiply, 0.0, 4.0);
        connect(patch, absolute, scaled, InputKind::A);
        let output = binary(patch, BinaryOp::Subtract, 1.0, 0.0);
        connect(patch, scaled, output, InputKind::B);
        output
    })
}

pub fn saw(frequency: Hertz, gain: Unit, unipolar: bool) -> Module {
    oscillator("Saw", frequency, gain, unipolar, |patch, phase| {
        let scaled = binary(patch, BinaryOp::Multiply, 0.0, 2.0);
        connect(patch, phase, scaled, InputKind::A);
        let output = binary(patch, BinaryOp::Subtract, 0.0, 1.0);
        connect(patch, scaled, output, InputKind::A);
        output
    })
}

pub fn reverse_saw(frequency: Hertz, gain: Unit, unipolar: bool) -> Module {
    oscillator("Reverse Saw", frequency, gain, unipolar, |patch, phase| {
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
    gain: Unit,
    unipolar: bool,
    shape: impl FnOnce(&mut Patch, crate::patch::ModuleId) -> crate::patch::ModuleId,
) -> Module {
    let mut patch = Patch::new();
    let frequency_input = patch.insert(Module::Input {
        kind: InputKind::Freq,
        default: Sample::new(frequency.value()).unwrap(),
    });
    let gain_input = patch.insert(Module::Input {
        kind: InputKind::Gain,
        default: Sample::new(gain.value()).unwrap(),
    });
    let phase = patch.insert(Module::Phase { frequency });
    connect(&mut patch, frequency_input, phase, InputKind::Freq);
    let oscillator = shape(&mut patch, phase);
    let signal = if unipolar {
        let offset = patch.insert(Module::Binary {
            op: BinaryOp::Add,
            a: Sample::ZERO,
            b: Sample::new(1.0).unwrap(),
        });
        connect(&mut patch, oscillator, offset, InputKind::A);
        let scale = patch.insert(Module::Binary {
            op: BinaryOp::Multiply,
            a: Sample::ZERO,
            b: Sample::new(0.5).unwrap(),
        });
        connect(&mut patch, offset, scale, InputKind::A);
        scale
    } else {
        oscillator
    };
    let output = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::new(gain.value()).unwrap(),
    });
    connect(&mut patch, signal, output, InputKind::A);
    connect(&mut patch, gain_input, output, InputKind::B);
    set_output(&mut patch, output);
    let composition_output = patch.output_id().unwrap();
    Module::Composition(Box::new(
        Composition::new(
            name,
            patch,
            [
                ("Frequency".to_string(), InputKind::Freq, frequency_input),
                ("Gain".to_string(), InputKind::Gain, gain_input),
            ],
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

pub fn attenuator(gain: Unit) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, gain.value());
    connect(&mut patch, signal, output, InputKind::A);
    set_output(&mut patch, output);
    composition("Attenuator", patch, [("Input", InputKind::In, signal)])
}

pub fn distortion(drive: Sample, asymmetry: Sample) -> Module {
    let mut patch = Patch::new();
    let signal = input(&mut patch, InputKind::In, 0.0);
    let drive_input = input(&mut patch, InputKind::Drive, drive.value());
    let asymmetry_input = input(&mut patch, InputKind::Asym, asymmetry.value());
    let offset = binary(&mut patch, BinaryOp::Add, 0.0, asymmetry.value());
    connect(&mut patch, signal, offset, InputKind::A);
    connect(&mut patch, asymmetry_input, offset, InputKind::B);
    let driven = binary(&mut patch, BinaryOp::Multiply, 0.0, drive.value());
    connect(&mut patch, offset, driven, InputKind::A);
    connect(&mut patch, drive_input, driven, InputKind::B);
    let shaped = patch.insert(tube());
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
    let positive_curve = patch.insert(Module::Unary(UnaryOp::HyperbolicTangent));
    connect(&mut patch, signal, positive_curve, InputKind::In);
    let negative_drive = binary(&mut patch, BinaryOp::Multiply, 0.0, 1.25);
    connect(&mut patch, signal, negative_drive, InputKind::A);
    let negative_curve = patch.insert(Module::Unary(UnaryOp::HyperbolicTangent));
    connect(&mut patch, negative_drive, negative_curve, InputKind::In);
    let negative_level = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.8);
    connect(&mut patch, negative_curve, negative_level, InputKind::A);
    let output = patch.insert(Module::Switch {
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
    let curve = patch.insert(Module::Unary(UnaryOp::Arctangent));
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
    let sign = patch.insert(Module::Unary(UnaryOp::Sign));
    connect(&mut patch, signal, sign, InputKind::In);
    let magnitude = patch.insert(Module::Unary(UnaryOp::Absolute));
    connect(&mut patch, signal, magnitude, InputKind::In);
    let scaled = binary(&mut patch, BinaryOp::Multiply, 0.0, -3.0);
    connect(&mut patch, magnitude, scaled, InputKind::A);
    let exponential = patch.insert(Module::Unary(UnaryOp::Exponential));
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
    let magnitude = patch.insert(Module::Unary(UnaryOp::Absolute));
    connect(&mut patch, centered, magnitude, InputKind::In);
    let output = binary(&mut patch, BinaryOp::Subtract, 0.0, 1.0);
    connect(&mut patch, magnitude, output, InputKind::A);
    set_output(&mut patch, output);
    composition("Fold", patch, [("Input", InputKind::In, signal)])
}

pub fn compressor(
    threshold: Unit,
    ratio: CompressorRatio,
    attack: Seconds,
    release: Seconds,
    makeup: Gain,
) -> Module {
    let mut patch = Patch::new();
    let input = patch.insert(Module::Input {
        kind: InputKind::In,
        default: Sample::ZERO,
    });
    let threshold_input = patch.insert(Module::Input {
        kind: InputKind::Thresh,
        default: Sample::new(threshold.value()).unwrap(),
    });
    let ratio_input = patch.insert(Module::Input {
        kind: InputKind::Ratio,
        default: Sample::new(ratio.value()).unwrap(),
    });
    let attack_input = patch.insert(Module::Input {
        kind: InputKind::Attack,
        default: Sample::new(attack.value()).unwrap(),
    });
    let release_input = patch.insert(Module::Input {
        kind: InputKind::Release,
        default: Sample::new(release.value()).unwrap(),
    });
    let makeup_input = patch.insert(Module::Input {
        kind: InputKind::Gain,
        default: Sample::new(makeup.value()).unwrap(),
    });
    let absolute = patch.insert(Module::Unary(UnaryOp::Absolute));
    let envelope = patch.insert(Module::Slew {
        rise: attack,
        fall: release,
    });
    let over = binary(&mut patch, BinaryOp::Divide, 0.0, threshold.value());
    let inverse_ratio = binary(&mut patch, BinaryOp::Divide, 1.0, ratio.value());
    let exponent = binary(&mut patch, BinaryOp::Subtract, 0.0, 1.0);
    let compressed = binary(&mut patch, BinaryOp::Power, 0.0, 0.0);
    let above_threshold = binary(&mut patch, BinaryOp::GreaterThan, 0.0, threshold.value());
    let reduction = patch.insert(Module::Switch {
        a: Sample::new(1.0).unwrap(),
        b: Sample::ZERO,
    });
    let apply_reduction = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::ZERO,
    });
    let apply_makeup = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::new(makeup.value()).unwrap(),
    });

    connect(&mut patch, input, absolute, InputKind::In);
    connect(&mut patch, absolute, envelope, InputKind::In);
    connect(&mut patch, attack_input, envelope, InputKind::Rise);
    connect(&mut patch, release_input, envelope, InputKind::Fall);
    connect(&mut patch, envelope, over, InputKind::A);
    connect(&mut patch, threshold_input, over, InputKind::B);
    connect(&mut patch, ratio_input, inverse_ratio, InputKind::B);
    connect(&mut patch, inverse_ratio, exponent, InputKind::A);
    connect(&mut patch, over, compressed, InputKind::A);
    connect(&mut patch, exponent, compressed, InputKind::B);
    connect(&mut patch, envelope, above_threshold, InputKind::A);
    connect(&mut patch, threshold_input, above_threshold, InputKind::B);
    connect(&mut patch, above_threshold, reduction, InputKind::Select);
    connect(&mut patch, compressed, reduction, InputKind::B);
    connect(&mut patch, input, apply_reduction, InputKind::A);
    connect(&mut patch, reduction, apply_reduction, InputKind::B);
    connect(&mut patch, apply_reduction, apply_makeup, InputKind::A);
    connect(&mut patch, makeup_input, apply_makeup, InputKind::B);
    set_output(&mut patch, apply_makeup);
    let composition_output = patch.output_id().unwrap();

    Module::Composition(Box::new(
        Composition::new(
            "Compressor",
            patch,
            [
                ("Input".to_string(), InputKind::In, input),
                ("Threshold".to_string(), InputKind::Thresh, threshold_input),
                ("Ratio".to_string(), InputKind::Ratio, ratio_input),
                ("Attack".to_string(), InputKind::Attack, attack_input),
                ("Release".to_string(), InputKind::Release, release_input),
                ("Makeup".to_string(), InputKind::Gain, makeup_input),
            ],
            [("Output".to_string(), composition_output)],
        )
        .unwrap(),
    ))
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
    let decay_delta = binary(&mut patch, BinaryOp::Multiply, 0.0, sustain.value() - 1.0);
    connect(&mut patch, decay_phase, decay_delta, InputKind::A);
    connect(&mut patch, sustain_delta, decay_delta, InputKind::B);
    let decay_value = binary(&mut patch, BinaryOp::Add, 1.0, 0.0);
    connect(&mut patch, decay_delta, decay_value, InputKind::B);
    let decaying = binary(&mut patch, BinaryOp::GreaterThan, 0.0, attack.value());
    connect(&mut patch, rise, decaying, InputKind::A);
    connect(&mut patch, attack_input, decaying, InputKind::B);
    let shaped = patch.insert(Module::Switch {
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
    let lfo = patch.insert(sine(rate, Unit::ONE, true));
    connect(&mut patch, rate_input, lfo, InputKind::Freq);
    let depth_amount = binary(&mut patch, BinaryOp::Multiply, 0.0, depth.value());
    connect(&mut patch, lfo, depth_amount, InputKind::A);
    connect(&mut patch, depth_input, depth_amount, InputKind::B);
    let sweep = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.02495);
    connect(&mut patch, depth_amount, sweep, InputKind::A);
    let delay_time = binary(&mut patch, BinaryOp::Add, 0.00005, 0.0);
    connect(&mut patch, sweep, delay_time, InputKind::B);
    let delay = patch.insert(Module::VariableDelay {
        max_time: Duration::Seconds(crate::time::Seconds::new(0.025).unwrap()),
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
    let delay_samples = [672, 1572, 2356, 3163, 908, 1800, 2656, 3720];
    let depths = [8.0, 7.0, 6.0, 5.0, 9.0, 8.0, 7.0, 6.0];
    let rates = [0.5, 0.6, 0.7, 0.8, 0.55, 0.65, 0.75, 0.85];
    let groups = [0, 4].map(|start| {
        let group = patch.insert(reverb_fdn_voice_group(
            std::array::from_fn(|channel| delay_samples[start + channel]),
            std::array::from_fn(|channel| depths[start + channel]),
            std::array::from_fn(|channel| rates[start + channel]),
            room,
            damp,
            modulation,
        ));
        connect(&mut patch, modulation_input, group, InputKind::Mod);
        connect(&mut patch, damp_input, group, InputKind::Damp);
        connect(&mut patch, room_input, group, InputKind::Room);
        connect(&mut patch, diffused, group, InputKind::Diff);
        group
    });
    let mut delays = Vec::new();
    for delay_samples in delay_samples {
        let delay = patch.insert(Module::Delay {
            time: Duration::Seconds(Seconds::new((delay_samples + 32) as f32 / 29_761.0).unwrap()),
            feedback: Unit::ZERO,
        });
        delays.push(delay);
    }
    let group_delayed = delays
        .iter()
        .map(|delay| patch.insert_delay_tap(*delay, Unit::ONE).unwrap())
        .collect::<Vec<_>>();
    let reflection_delayed = (0..3)
        .map(|_| {
            delays
                .iter()
                .map(|delay| patch.insert_delay_tap(*delay, Unit::ONE).unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let output_delayed = delays
        .iter()
        .map(|delay| patch.insert_delay_tap(*delay, Unit::ONE).unwrap())
        .collect::<Vec<_>>();
    let reflections = reflection_delayed
        .iter()
        .map(|delayed| {
            let reflection = patch.insert(reverb_reflection());
            for (channel, kind) in REVERB_CHANNEL_KINDS.into_iter().enumerate() {
                connect(&mut patch, delayed[channel], reflection, kind);
            }
            reflection
        })
        .collect::<Vec<_>>();
    for (channel, _) in REVERB_CHANNEL_KINDS.into_iter().enumerate() {
        let group = groups[channel / 4];
        connect(
            &mut patch,
            group_delayed[channel],
            group,
            REVERB_GROUP_CHANNEL_KINDS[channel % 4],
        );
        connect_output(
            &mut patch,
            group,
            (channel % 4 * 2) as u16,
            delays[channel],
            InputKind::Time,
        );
        connect_output(
            &mut patch,
            group,
            (channel % 4 * 2 + 1) as u16,
            delays[channel],
            InputKind::In,
        );
    }
    connect(&mut patch, reflections[0], groups[0], InputKind::Feedback);
    connect(&mut patch, reflections[1], groups[1], InputKind::Feedback);
    let output = patch.insert(reverb_output_decoder());
    for (channel, kind) in REVERB_CHANNEL_KINDS.into_iter().enumerate() {
        connect(&mut patch, output_delayed[channel], output, kind);
    }
    connect(&mut patch, reflections[2], output, InputKind::Feedback);
    set_output(&mut patch, output);
    composition(
        "FDN Tank",
        patch,
        [
            ("Input", InputKind::In, diffused),
            ("Room", InputKind::Room, room_input),
            ("Damping", InputKind::Damp, damp_input),
            ("Modulation", InputKind::Mod, modulation_input),
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
const REVERB_GROUP_CHANNEL_KINDS: [InputKind; 4] = [
    InputKind::Channel1,
    InputKind::Channel2,
    InputKind::Channel3,
    InputKind::Channel4,
];

fn reverb_fdn_voice_group(
    samples: [usize; 4],
    depths: [f32; 4],
    rates: [f32; 4],
    room: Unit,
    damp: Unit,
    modulation: Unit,
) -> Module {
    let mut patch = Patch::new();
    let delayed = REVERB_GROUP_CHANNEL_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let diffused = input(&mut patch, InputKind::Diff, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let delay_group = patch.insert(reverb_delay_group(samples, depths, rates, modulation));
    connect(&mut patch, modulation_input, delay_group, InputKind::Mod);
    let feedback_group = patch.insert(reverb_feedback_group(room, damp));
    for (channel, kind) in REVERB_GROUP_CHANNEL_KINDS.into_iter().enumerate() {
        connect(&mut patch, delayed[channel], feedback_group, kind);
    }
    connect(&mut patch, reflection, feedback_group, InputKind::Feedback);
    connect(&mut patch, diffused, feedback_group, InputKind::Diff);
    connect(&mut patch, room_input, feedback_group, InputKind::Room);
    connect(&mut patch, damp_input, feedback_group, InputKind::Damp);
    let outputs = (0..4)
        .flat_map(|channel| {
            [
                (
                    format!("Time {}", channel + 1),
                    patch.output_port(delay_group, channel).unwrap(),
                ),
                (
                    format!("Signal {}", channel + 1),
                    patch.output_port(feedback_group, channel).unwrap(),
                ),
            ]
        })
        .collect::<Vec<_>>();
    patch.output(outputs[0].1).unwrap();
    Module::Composition(Box::new(
        Composition::new(
            "FDN Voice Group",
            patch,
            REVERB_GROUP_CHANNEL_KINDS
                .into_iter()
                .enumerate()
                .map(|(channel, kind)| {
                    (
                        REVERB_CHANNEL_LABELS[channel].to_string(),
                        kind,
                        delayed[channel],
                    )
                })
                .chain([
                    ("Reflection".to_string(), InputKind::Feedback, reflection),
                    ("Diffused".to_string(), InputKind::Diff, diffused),
                    ("Room".to_string(), InputKind::Room, room_input),
                    ("Damping".to_string(), InputKind::Damp, damp_input),
                    ("Modulation".to_string(), InputKind::Mod, modulation_input),
                ]),
            outputs,
        )
        .unwrap(),
    ))
}

fn reverb_delay_group(
    samples: [usize; 4],
    depths: [f32; 4],
    rates: [f32; 4],
    modulation: Unit,
) -> Module {
    let mut patch = Patch::new();
    let input = input(&mut patch, InputKind::Mod, modulation.value());
    let outputs = (0..4)
        .map(|channel| {
            let output = patch.insert(reverb_delay_time(
                samples[channel],
                depths[channel],
                rates[channel],
                modulation,
            ));
            connect(&mut patch, input, output, InputKind::Mod);
            output
        })
        .collect::<Vec<_>>();
    composition_outputs(
        "Delay Group",
        patch,
        [("Modulation", InputKind::Mod, input)],
        outputs,
    )
}

fn reverb_feedback_group(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let delayed = REVERB_GROUP_CHANNEL_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let diffused = input(&mut patch, InputKind::Diff, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let outputs = delayed.map(|delayed| {
        let path = patch.insert(reverb_feedback_path(room, damp));
        connect(&mut patch, delayed, path, InputKind::In);
        connect(&mut patch, reflection, path, InputKind::Feedback);
        connect(&mut patch, diffused, path, InputKind::Diff);
        connect(&mut patch, room_input, path, InputKind::Room);
        connect(&mut patch, damp_input, path, InputKind::Damp);
        path
    });
    composition_outputs(
        "Feedback Group",
        patch,
        REVERB_GROUP_CHANNEL_KINDS
            .into_iter()
            .enumerate()
            .map(|(channel, kind)| (REVERB_CHANNEL_LABELS[channel], kind, delayed[channel]))
            .chain([
                ("Reflection", InputKind::Feedback, reflection),
                ("Diffused", InputKind::Diff, diffused),
                ("Room", InputKind::Room, room_input),
                ("Damping", InputKind::Damp, damp_input),
            ]),
        outputs,
    )
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
    let oscillator = patch.insert(sine(Hertz::new(rate).unwrap(), Unit::ONE, false));
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

fn reverb_feedback_path(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let delayed = input(&mut patch, InputKind::In, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let diffused = input(&mut patch, InputKind::Diff, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let mixed = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, delayed, mixed, InputKind::A);
    connect(&mut patch, reflection, mixed, InputKind::B);
    let damping = patch.insert(Module::Damp { coefficient: damp });
    connect(&mut patch, mixed, damping, InputKind::In);
    connect(&mut patch, damp_input, damping, InputKind::Damp);
    let decay = patch.insert(reverb_room_decay(room));
    connect(&mut patch, room_input, decay, InputKind::Room);
    let decayed = binary(&mut patch, BinaryOp::Multiply, 0.0, room.value());
    connect(&mut patch, damping, decayed, InputKind::A);
    connect(&mut patch, decay, decayed, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, diffused, output, InputKind::A);
    connect(&mut patch, decayed, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Feedback Path",
        patch,
        [
            ("Delayed", InputKind::In, delayed),
            ("Reflection", InputKind::Feedback, reflection),
            ("Diffused", InputKind::Diff, diffused),
            ("Room", InputKind::Room, room_input),
            ("Damping", InputKind::Damp, damp_input),
        ],
    )
}

fn reverb_room_decay(room: Unit) -> Module {
    let mut patch = Patch::new();
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let room_scale = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.69);
    connect(&mut patch, room_input, room_scale, InputKind::A);
    let decay = binary(&mut patch, BinaryOp::Add, 0.3, 0.0);
    connect(&mut patch, room_scale, decay, InputKind::B);
    set_output(&mut patch, decay);
    composition("Room Decay", patch, [("Room", InputKind::Room, room_input)])
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

fn connect_output(
    patch: &mut Patch,
    from: crate::patch::ModuleId,
    output: u16,
    to: crate::patch::ModuleId,
    input: InputKind,
) {
    let input = patch.input_port(to, input).unwrap();
    let from = patch.output_port(from, output).unwrap();
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
            (sine(frequency, Unit::ONE, false), [0.0, 1.0, 0.0, -1.0]),
            (square(frequency, Unit::ONE, false), [1.0, 1.0, -1.0, -1.0]),
            (triangle(frequency, Unit::ONE, false), [-1.0, 0.0, 1.0, 0.0]),
            (saw(frequency, Unit::ONE, false), [-1.0, -0.5, 0.0, 0.5]),
            (
                reverse_saw(frequency, Unit::ONE, false),
                [1.0, 0.5, 0.0, -0.5],
            ),
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
    fn flanger_is_a_composed_variable_delay_graph() {
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
