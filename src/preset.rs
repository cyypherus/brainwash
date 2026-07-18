use crate::effect::{Distortion, Drive};
use crate::osc::Wave;
use crate::patch::{BinaryOp, Composition, CompressorRatio, Gain, InputKind, Module, Patch};
use crate::sample::{Sample, Unit};
use crate::time::{Duration, Hertz, Seconds};

pub fn oscillator(wave: Wave, frequency: Hertz, gain: Unit, unipolar: bool) -> Module {
    let mut patch = Patch::new();
    let frequency_input = patch.insert(Module::Input {
        kind: InputKind::Freq,
        default: Sample::new(frequency.value()).unwrap(),
    });
    let gain_input = patch.insert(Module::Input {
        kind: InputKind::Gain,
        default: Sample::new(gain.value()).unwrap(),
    });
    let oscillator = patch.insert(Module::Osc { wave, frequency });
    connect(&mut patch, frequency_input, oscillator, InputKind::Freq);
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
            "Oscillator",
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

pub fn distortion(kind: Distortion, drive: Drive, asymmetry: Sample) -> Module {
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
    let shaped = patch.insert(Module::Waveshaper(kind));
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
    let absolute = patch.insert(Module::Absolute);
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
    let lfo = patch.insert(oscillator(Wave::Sine, rate, Unit::ONE, true));
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
    let drives = [0, 4].map(|start| {
        let drive = patch.insert(reverb_drive_bank(
            &delay_samples[start..start + 4],
            &depths[start..start + 4],
            &rates[start..start + 4],
            room,
            damp,
            modulation,
        ));
        connect(&mut patch, modulation_input, drive, InputKind::Mod);
        connect(&mut patch, damp_input, drive, InputKind::Damp);
        connect(&mut patch, room_input, drive, InputKind::Room);
        connect(&mut patch, diffused, drive, InputKind::Diff);
        drive
    });
    let mut delays = Vec::new();
    for delay_samples in delay_samples.iter().take(8) {
        let delay = patch.insert(Module::Delay {
            time: Duration::Seconds(Seconds::new((*delay_samples + 32) as f32 / 29_761.0).unwrap()),
            feedback: Unit::ZERO,
        });
        delays.push(delay);
    }
    let delayed = delays
        .iter()
        .map(|delay| patch.insert_delay_tap(*delay, Unit::ONE).unwrap())
        .collect::<Vec<_>>();

    let reflection = patch.insert(reverb_reflection());
    for (channel, kind) in REVERB_CHANNEL_KINDS.into_iter().enumerate() {
        connect(&mut patch, delayed[channel], reflection, kind);
    }
    for channel in 0..8 {
        let drive = drives[channel / 4];
        connect(
            &mut patch,
            delayed[channel],
            drive,
            REVERB_FEEDBACK_KINDS[channel % 4],
        );
        connect_output(
            &mut patch,
            drive,
            (channel % 4 * 2) as u16,
            delays[channel],
            InputKind::Time,
        );
        connect_output(
            &mut patch,
            drive,
            (channel % 4 * 2 + 1) as u16,
            delays[channel],
            InputKind::In,
        );
    }
    connect(&mut patch, reflection, drives[0], InputKind::Rate);
    connect(&mut patch, reflection, drives[1], InputKind::Rate);
    let output = patch.insert(reverb_output_mix());
    for (channel, kind) in REVERB_CHANNEL_KINDS.into_iter().enumerate() {
        connect(&mut patch, delayed[channel], output, kind);
    }
    connect(&mut patch, reflection, output, InputKind::Rate);
    set_output(&mut patch, output);
    composition(
        "FDN Tank",
        patch,
        [
            ("Modulation", InputKind::Mod, modulation_input),
            ("Input", InputKind::In, diffused),
            ("Room", InputKind::Room, room_input),
            ("Damping", InputKind::Damp, damp_input),
        ],
    )
}

const REVERB_CHANNEL_KINDS: [InputKind; 8] = [
    InputKind::In,
    InputKind::A,
    InputKind::B,
    InputKind::Diff,
    InputKind::Feedback,
    InputKind::Room,
    InputKind::Damp,
    InputKind::Mod,
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
const REVERB_FEEDBACK_KINDS: [InputKind; 4] =
    [InputKind::In, InputKind::A, InputKind::B, InputKind::Gain];

fn reverb_modulation_bank(
    samples: &[usize],
    depths: &[f32],
    rates: &[f32],
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
        "Delay Modulation Bank",
        patch,
        [("Modulation", InputKind::Mod, input)],
        outputs,
    )
}

fn reverb_drive_bank(
    samples: &[usize],
    depths: &[f32],
    rates: &[f32],
    room: Unit,
    damp: Unit,
    modulation: Unit,
) -> Module {
    let mut patch = Patch::new();
    let delayed = REVERB_FEEDBACK_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let diffused = input(&mut patch, InputKind::Diff, 0.0);
    let reflection = input(&mut patch, InputKind::Rate, 0.0);
    let modulation_bank = patch.insert(reverb_modulation_bank(samples, depths, rates, modulation));
    connect(
        &mut patch,
        modulation_input,
        modulation_bank,
        InputKind::Mod,
    );
    let feedback_bank = patch.insert(reverb_feedback_bank(room, damp));
    for channel in 0..4 {
        connect(
            &mut patch,
            delayed[channel],
            feedback_bank,
            REVERB_FEEDBACK_KINDS[channel],
        );
    }
    connect(&mut patch, reflection, feedback_bank, InputKind::Feedback);
    connect(&mut patch, diffused, feedback_bank, InputKind::Diff);
    connect(&mut patch, room_input, feedback_bank, InputKind::Room);
    connect(&mut patch, damp_input, feedback_bank, InputKind::Damp);
    let outputs = (0..4)
        .flat_map(|channel| {
            [
                (
                    format!("Time {}", channel + 1),
                    patch.output_port(modulation_bank, channel).unwrap(),
                ),
                (
                    format!("Signal {}", channel + 1),
                    patch.output_port(feedback_bank, channel).unwrap(),
                ),
            ]
        })
        .collect::<Vec<_>>();
    patch.output(outputs[0].1).unwrap();
    Module::Composition(Box::new(
        Composition::new(
            "FDN Drive",
            patch,
            [
                (
                    REVERB_CHANNEL_LABELS[0].to_string(),
                    InputKind::In,
                    delayed[0],
                ),
                (
                    REVERB_CHANNEL_LABELS[1].to_string(),
                    InputKind::A,
                    delayed[1],
                ),
                (
                    REVERB_CHANNEL_LABELS[2].to_string(),
                    InputKind::B,
                    delayed[2],
                ),
                (
                    REVERB_CHANNEL_LABELS[3].to_string(),
                    InputKind::Gain,
                    delayed[3],
                ),
                ("Reflection".to_string(), InputKind::Rate, reflection),
                ("Diffused".to_string(), InputKind::Diff, diffused),
                ("Room".to_string(), InputKind::Room, room_input),
                ("Damping".to_string(), InputKind::Damp, damp_input),
                ("Modulation".to_string(), InputKind::Mod, modulation_input),
            ],
            outputs,
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

fn reverb_feedback_bank(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let delayed = REVERB_FEEDBACK_KINDS
        .iter()
        .map(|kind| input(&mut patch, *kind, 0.0))
        .collect::<Vec<_>>();
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let diffused = input(&mut patch, InputKind::Diff, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let outputs = delayed
        .iter()
        .map(|delayed| {
            let output = patch.insert(reverb_feedback(room, damp));
            connect(&mut patch, *delayed, output, InputKind::In);
            connect(&mut patch, reflection, output, InputKind::B);
            connect(&mut patch, diffused, output, InputKind::A);
            connect(&mut patch, room_input, output, InputKind::Room);
            connect(&mut patch, damp_input, output, InputKind::Damp);
            output
        })
        .collect::<Vec<_>>();
    composition_outputs(
        "Feedback Bank",
        patch,
        REVERB_FEEDBACK_KINDS
            .iter()
            .enumerate()
            .map(|(index, kind)| (REVERB_CHANNEL_LABELS[index], *kind, delayed[index]))
            .chain([
                ("Reflection", InputKind::Feedback, reflection),
                ("Diffused", InputKind::Diff, diffused),
                ("Room", InputKind::Room, room_input),
                ("Damping", InputKind::Damp, damp_input),
            ]),
        outputs,
    )
}

fn reverb_output_mix() -> Module {
    let mut patch = Patch::new();
    let delayed = REVERB_CHANNEL_KINDS.map(|kind| input(&mut patch, kind, 0.0));
    let reflection = input(&mut patch, InputKind::Rate, 0.0);
    let left = patch.insert(reverb_output_side([1.0, 1.0, -1.0, 1.0]));
    let right = patch.insert(reverb_output_side([1.0, -1.0, 1.0, 1.0]));
    for (channel, kind) in
        [0, 2, 4, 6]
            .into_iter()
            .zip([InputKind::In, InputKind::A, InputKind::B, InputKind::Diff])
    {
        connect(&mut patch, delayed[channel], left, kind);
    }
    for (channel, kind) in
        [1, 3, 5, 7]
            .into_iter()
            .zip([InputKind::In, InputKind::A, InputKind::B, InputKind::Diff])
    {
        connect(&mut patch, delayed[channel], right, kind);
    }
    connect(&mut patch, reflection, left, InputKind::Feedback);
    connect(&mut patch, reflection, right, InputKind::Feedback);
    let mono = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, left, mono, InputKind::A);
    connect(&mut patch, right, mono, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.25);
    connect(&mut patch, mono, output, InputKind::A);
    composition_outputs(
        "Output Mix",
        patch,
        REVERB_CHANNEL_KINDS
            .into_iter()
            .enumerate()
            .map(|(index, kind)| (REVERB_CHANNEL_LABELS[index], kind, delayed[index]))
            .chain([("Reflection", InputKind::Rate, reflection)]),
        [output],
    )
}

fn reverb_delay_time(samples: usize, depth: f32, rate: f32, modulation: Unit) -> Module {
    let mut patch = Patch::new();
    let modulation_input = input(&mut patch, InputKind::Mod, modulation.value());
    let oscillator = patch.insert(Module::Osc {
        wave: Wave::Sine,
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

fn reverb_feedback(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let delayed = input(&mut patch, InputKind::In, 0.0);
    let reflection = input(&mut patch, InputKind::B, 0.0);
    let diffused = input(&mut patch, InputKind::A, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let decayed = patch.insert(reverb_decay(room, damp));
    connect(&mut patch, delayed, decayed, InputKind::In);
    connect(&mut patch, reflection, decayed, InputKind::Feedback);
    connect(&mut patch, room_input, decayed, InputKind::Room);
    connect(&mut patch, damp_input, decayed, InputKind::Damp);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, diffused, output, InputKind::A);
    connect(&mut patch, decayed, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Feedback Channel",
        patch,
        [
            ("Delayed", InputKind::In, delayed),
            ("Reflection", InputKind::B, reflection),
            ("Diffused", InputKind::A, diffused),
            ("Room", InputKind::Room, room_input),
            ("Damping", InputKind::Damp, damp_input),
        ],
    )
}

fn reverb_decay(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let delayed = input(&mut patch, InputKind::In, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
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
    set_output(&mut patch, decayed);
    composition(
        "Feedback Decay",
        patch,
        [
            ("Delayed", InputKind::In, delayed),
            ("Reflection", InputKind::Feedback, reflection),
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

fn reverb_output_side(coefficients: [f32; 4]) -> Module {
    let mut patch = Patch::new();
    let first = input(&mut patch, InputKind::In, 0.0);
    let second = input(&mut patch, InputKind::A, 0.0);
    let third = input(&mut patch, InputKind::B, 0.0);
    let fourth = input(&mut patch, InputKind::Diff, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let left = patch.insert(reverb_output_pair(coefficients[0], coefficients[1], false));
    connect(&mut patch, first, left, InputKind::In);
    connect(&mut patch, second, left, InputKind::A);
    connect(&mut patch, reflection, left, InputKind::Feedback);
    let right = patch.insert(reverb_output_pair(coefficients[2], coefficients[3], true));
    connect(&mut patch, third, right, InputKind::B);
    connect(&mut patch, fourth, right, InputKind::Diff);
    connect(&mut patch, reflection, right, InputKind::Feedback);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, left, output, InputKind::A);
    connect(&mut patch, right, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Output Side",
        patch,
        [
            ("First", InputKind::In, first),
            ("Second", InputKind::A, second),
            ("Third", InputKind::B, third),
            ("Fourth", InputKind::Diff, fourth),
            ("Reflection", InputKind::Feedback, reflection),
        ],
    )
}

fn reverb_output_pair(
    first_coefficient: f32,
    second_coefficient: f32,
    second_pair: bool,
) -> Module {
    let mut patch = Patch::new();
    let first_kind = if second_pair {
        InputKind::B
    } else {
        InputKind::In
    };
    let second_kind = if second_pair {
        InputKind::Diff
    } else {
        InputKind::A
    };
    let first_input = input(&mut patch, first_kind, 0.0);
    let second_input = input(&mut patch, second_kind, 0.0);
    let reflection = input(&mut patch, InputKind::Feedback, 0.0);
    let first = reverb_output_tap(&mut patch, first_input, reflection, first_coefficient);
    let second = reverb_output_tap(&mut patch, second_input, reflection, second_coefficient);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, first, output, InputKind::A);
    connect(&mut patch, second, output, InputKind::B);
    set_output(&mut patch, output);
    composition(
        "Output Pair",
        patch,
        [
            ("First", first_kind, first_input),
            ("Second", second_kind, second_input),
            ("Reflection", InputKind::Feedback, reflection),
        ],
    )
}

fn reverb_output_tap(
    patch: &mut Patch,
    delayed: crate::patch::ModuleId,
    reflection: crate::patch::ModuleId,
    coefficient: f32,
) -> crate::patch::ModuleId {
    let mixed = binary(patch, BinaryOp::Add, 0.0, 0.0);
    connect(patch, delayed, mixed, InputKind::A);
    connect(patch, reflection, mixed, InputKind::B);
    if coefficient == 1.0 {
        mixed
    } else {
        let inverted = binary(patch, BinaryOp::Multiply, 0.0, coefficient);
        connect(patch, mixed, inverted, InputKind::A);
        inverted
    }
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
}
