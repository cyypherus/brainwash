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
    patch.output(output).unwrap();
    Module::Composition(Box::new(
        Composition::new(
            "Oscillator",
            patch,
            [
                ("Frequency".to_string(), InputKind::Freq, frequency_input),
                ("Gain".to_string(), InputKind::Gain, gain_input),
            ],
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
    patch.output(output).unwrap();
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
    patch.output(output).unwrap();
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
    patch.output(shaped).unwrap();
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
    patch.output(apply_makeup).unwrap();

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
    patch.output(output).unwrap();
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
    patch.output(output).unwrap();
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
    patch.output(tank).unwrap();
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
        let scaled = binary(&mut patch, BinaryOp::Multiply, 0.0, coefficient);
        connect(&mut patch, diffusion_input, scaled, InputKind::A);
        let allpass = patch.insert(Module::Allpass {
            time: Duration::Seconds(Seconds::new(samples as f32 / 29_761.0).unwrap()),
            feedback: Unit::new(coefficient * diffusion.value()).unwrap(),
        });
        connect(&mut patch, diffused, allpass, InputKind::In);
        connect(&mut patch, scaled, allpass, InputKind::Feedback);
        diffused = allpass;
    }
    patch.output(diffused).unwrap();
    composition(
        "Input Diffuser",
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
    let mut delays = Vec::new();
    for channel in 0..8 {
        let delay_time = patch.insert(reverb_delay_time(
            delay_samples[channel],
            depths[channel],
            rates[channel],
            modulation,
        ));
        connect(&mut patch, modulation_input, delay_time, InputKind::Mod);
        let delay = patch.insert(Module::Delay {
            time: Duration::Seconds(
                Seconds::new((delay_samples[channel] + 32) as f32 / 29_761.0).unwrap(),
            ),
            feedback: Unit::ZERO,
        });
        connect(&mut patch, delay_time, delay, InputKind::Time);
        delays.push(delay);
    }
    let delayed = delays
        .iter()
        .map(|delay| patch.insert_delay_tap(*delay, Unit::ONE).unwrap())
        .collect::<Vec<_>>();

    let sum = delayed
        .iter()
        .copied()
        .reduce(|a, b| {
            let add = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
            connect(&mut patch, a, add, InputKind::A);
            connect(&mut patch, b, add, InputKind::B);
            add
        })
        .unwrap();
    let reflection = binary(&mut patch, BinaryOp::Multiply, 0.0, -0.25);
    connect(&mut patch, sum, reflection, InputKind::A);
    let mut tank_outputs = Vec::new();
    for channel in 0..8 {
        let mixed = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
        connect(&mut patch, delayed[channel], mixed, InputKind::A);
        connect(&mut patch, reflection, mixed, InputKind::B);
        tank_outputs.push(mixed);
        let tank_input = patch.insert(reverb_feedback(room, damp));
        connect(&mut patch, mixed, tank_input, InputKind::In);
        connect(&mut patch, diffused, tank_input, InputKind::A);
        connect(&mut patch, room_input, tank_input, InputKind::Room);
        connect(&mut patch, damp_input, tank_input, InputKind::Damp);
        connect(&mut patch, tank_input, delays[channel], InputKind::In);
    }

    let coefficients = [1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
    let mut output_channels = Vec::new();
    for (delay, coefficient) in tank_outputs.into_iter().zip(coefficients) {
        let scaled = binary(&mut patch, BinaryOp::Multiply, 0.0, coefficient * 0.25);
        connect(&mut patch, delay, scaled, InputKind::A);
        output_channels.push(scaled);
    }
    let output = output_channels
        .into_iter()
        .reduce(|a, b| {
            let add = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
            connect(&mut patch, a, add, InputKind::A);
            connect(&mut patch, b, add, InputKind::B);
            add
        })
        .unwrap();
    patch.output(output).unwrap();
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
    patch.output(time).unwrap();
    composition("Delay Modulation", patch, [("Modulation", InputKind::Mod, modulation_input)])
}

fn reverb_feedback(room: Unit, damp: Unit) -> Module {
    let mut patch = Patch::new();
    let mixed = input(&mut patch, InputKind::In, 0.0);
    let diffused = input(&mut patch, InputKind::A, 0.0);
    let room_input = input(&mut patch, InputKind::Room, room.value());
    let damp_input = input(&mut patch, InputKind::Damp, damp.value());
    let damping = patch.insert(Module::Damp { coefficient: damp });
    connect(&mut patch, mixed, damping, InputKind::In);
    connect(&mut patch, damp_input, damping, InputKind::Damp);
    let room_scale = binary(&mut patch, BinaryOp::Multiply, 0.0, 0.69);
    connect(&mut patch, room_input, room_scale, InputKind::A);
    let decay = binary(&mut patch, BinaryOp::Add, 0.3, 0.0);
    connect(&mut patch, room_scale, decay, InputKind::B);
    let decayed = binary(&mut patch, BinaryOp::Multiply, 0.0, room.value());
    connect(&mut patch, damping, decayed, InputKind::A);
    connect(&mut patch, decay, decayed, InputKind::B);
    let output = binary(&mut patch, BinaryOp::Add, 0.0, 0.0);
    connect(&mut patch, diffused, output, InputKind::A);
    connect(&mut patch, decayed, output, InputKind::B);
    patch.output(output).unwrap();
    composition(
        "Feedback Channel",
        patch,
        [
            ("Mixed", InputKind::In, mixed),
            ("Diffused", InputKind::A, diffused),
            ("Room", InputKind::Room, room_input),
            ("Damping", InputKind::Damp, damp_input),
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
    patch.connect_input(from, input).unwrap();
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
        patch.output(reverb).unwrap();
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
    Module::Composition(Box::new(
        Composition::new(
            name,
            patch,
            inputs
                .into_iter()
                .map(|(label, kind, module)| (label.to_string(), kind, module)),
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
        patch.output(envelope).unwrap();
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
        patch.output(effect).unwrap();
        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();

        assert!(compiled.next().left().value().is_finite());
    }
}
