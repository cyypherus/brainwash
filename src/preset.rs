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

pub fn reverb(room: Unit, damp: Unit, mix: Unit, diffusion: Unit) -> Module {
    let mut patch = Patch::new();
    let input = patch.insert(Module::Input {
        kind: InputKind::In,
        default: Sample::ZERO,
    });
    let room_input = patch.insert(Module::Input {
        kind: InputKind::Room,
        default: Sample::new(room.value()).unwrap(),
    });
    let damp_input = patch.insert(Module::Input {
        kind: InputKind::Damp,
        default: Sample::new(damp.value()).unwrap(),
    });
    let mix_input = patch.insert(Module::Input {
        kind: InputKind::Mix,
        default: Sample::new(mix.value()).unwrap(),
    });
    let diffusion_input = patch.insert(Module::Input {
        kind: InputKind::Diff,
        default: Sample::new(diffusion.value()).unwrap(),
    });
    let diffused = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::new(diffusion.value()).unwrap(),
    });
    connect(&mut patch, input, diffused, InputKind::A);
    connect(&mut patch, diffusion_input, diffused, InputKind::B);

    let mut wet = None;
    for samples in [149, 211, 263, 293] {
        let comb = patch.insert(Module::Comb {
            time: Duration::Seconds(Seconds::new(samples as f32 / 44_100.0).unwrap()),
            feedback: room,
            damp,
        });
        connect(&mut patch, diffused, comb, InputKind::In);
        connect(&mut patch, room_input, comb, InputKind::Feedback);
        connect(&mut patch, damp_input, comb, InputKind::Damp);
        wet = Some(if let Some(sum) = wet {
            let add = patch.insert(Module::Binary {
                op: BinaryOp::Add,
                a: Sample::ZERO,
                b: Sample::ZERO,
            });
            connect(&mut patch, sum, add, InputKind::A);
            connect(&mut patch, comb, add, InputKind::B);
            add
        } else {
            comb
        });
    }

    let wet_scale = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::new(0.25).unwrap(),
    });
    connect(&mut patch, wet.unwrap(), wet_scale, InputKind::A);
    let wet_mix = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::new(mix.value()).unwrap(),
    });
    connect(&mut patch, wet_scale, wet_mix, InputKind::A);
    connect(&mut patch, mix_input, wet_mix, InputKind::B);
    let negative_mix = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::new(-0.5).unwrap(),
    });
    connect(&mut patch, mix_input, negative_mix, InputKind::A);
    let dry_amount = patch.insert(Module::Binary {
        op: BinaryOp::Add,
        a: Sample::new(1.0).unwrap(),
        b: Sample::ZERO,
    });
    connect(&mut patch, negative_mix, dry_amount, InputKind::B);
    let dry_mix = patch.insert(Module::Binary {
        op: BinaryOp::Multiply,
        a: Sample::ZERO,
        b: Sample::ZERO,
    });
    connect(&mut patch, input, dry_mix, InputKind::A);
    connect(&mut patch, dry_amount, dry_mix, InputKind::B);
    let output = patch.insert(Module::Binary {
        op: BinaryOp::Add,
        a: Sample::ZERO,
        b: Sample::ZERO,
    });
    connect(&mut patch, dry_mix, output, InputKind::A);
    connect(&mut patch, wet_mix, output, InputKind::B);
    patch.output(output).unwrap();

    Module::Composition(Box::new(
        Composition::new(
            "Reverb",
            patch,
            [
                ("Input".to_string(), InputKind::In, input),
                ("Room".to_string(), InputKind::Room, room_input),
                ("Damping".to_string(), InputKind::Damp, damp_input),
                ("Mix".to_string(), InputKind::Mix, mix_input),
                ("Diffusion".to_string(), InputKind::Diff, diffusion_input),
            ],
        )
        .unwrap(),
    ))
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
    use crate::time::SampleRate;

    struct ReferenceReverb {
        delays: Vec<Vec<f32>>,
        indices: Vec<usize>,
        room: f32,
        damp: f32,
        mix: f32,
        diffusion: f32,
        store: Vec<f32>,
    }

    impl ReferenceReverb {
        fn new(rate: u32, room: f32, damp: f32, mix: f32, diffusion: f32) -> Self {
            let scale = rate as f32 / 44_100.0;
            let sizes =
                [149, 211, 263, 293].map(|size| ((size as f32 * scale).round() as usize).max(1));
            Self {
                delays: sizes.map(|size| vec![0.0; size]).into(),
                indices: vec![0; sizes.len()],
                room,
                damp,
                mix,
                diffusion,
                store: vec![0.0; sizes.len()],
            }
        }

        fn next(&mut self, input: f32) -> f32 {
            let mut sum = 0.0;
            for index in 0..self.delays.len() {
                let position = self.indices[index];
                let delayed = self.delays[index][position];
                self.store[index] = delayed * (1.0 - self.damp) + self.store[index] * self.damp;
                self.delays[index][position] =
                    input * self.diffusion + self.store[index] * self.room;
                self.indices[index] = (position + 1) % self.delays[index].len();
                sum += delayed;
            }
            input * (1.0 - self.mix * 0.5) + sum * 0.25 * self.mix
        }
    }

    #[test]
    fn composed_reverb_matches_removed_reverb_node() {
        let room = 0.7;
        let damp = 0.2;
        let mix = 0.6;
        let diffusion = 0.8;
        for rate in [44_100, 48_000] {
            let mut patch = Patch::new();
            let input = patch.insert(Module::Gate);
            let reverb = patch.insert(super::reverb(
                Unit::new(room).unwrap(),
                Unit::new(damp).unwrap(),
                Unit::new(mix).unwrap(),
                Unit::new(diffusion).unwrap(),
            ));
            connect(&mut patch, input, reverb, InputKind::In);
            patch.output(reverb).unwrap();
            let sample_rate = SampleRate::new(rate).unwrap();
            let mut composed = CompiledPatch::new(&patch, sample_rate).unwrap();
            let mut reference = ReferenceReverb::new(rate, room, damp, mix, diffusion);
            for frame in 0..2_000 {
                let signal = if frame == 0 { 1.0 } else { 0.0 };
                let actual = composed
                    .next_with_controls(PatchControls {
                        gate: signal,
                        ..PatchControls::default()
                    })
                    .left()
                    .value();
                let expected = reference.next(signal);
                assert!(
                    (actual - expected).abs() < 0.000_001,
                    "rate {rate} frame {frame}: {actual} != {expected}"
                );
            }
        }
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
