use crate::patch::{
    self, BinaryOp, Composition, EnvPoint, InputKind, Module, Patch, SaturationCurve, UnaryOp,
    Waveform,
};
use crate::sample::{Sample, Unit};
use crate::time::{Duration, Hertz, Samples, Seconds};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    Ron(ron::error::SpannedError),
    Value,
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Ron(ron::Error),
    Value,
}

pub fn to_string(patch: &Patch) -> Result<String, SaveError> {
    ron::to_string(&FilePatch::from_patch(patch)?).map_err(SaveError::Ron)
}

pub fn from_str(input: &str) -> Result<Patch, LoadError> {
    let file: FilePatch = ron::from_str(input).map_err(LoadError::Ron)?;
    file.into_patch()
}

pub fn load(path: &Path) -> Result<Patch, LoadError> {
    let input = fs::read_to_string(path).map_err(LoadError::Io)?;
    from_str(&input)
}

pub fn save(path: &Path, patch: &Patch) -> Result<(), SaveError> {
    let output = to_string(patch)?;
    fs::write(path, output).map_err(SaveError::Io)
}

pub fn composition_to_string(composition: &Composition) -> Result<String, SaveError> {
    ron::to_string(&FileComposition::from_composition(composition)?).map_err(SaveError::Ron)
}

pub fn composition_from_str(input: &str) -> Result<Composition, LoadError> {
    let file: FileComposition = ron::from_str(input).map_err(LoadError::Ron)?;
    file.into_composition()
}

pub fn load_composition(path: &Path) -> Result<Composition, LoadError> {
    let input = fs::read_to_string(path).map_err(LoadError::Io)?;
    composition_from_str(&input)
}

pub fn save_composition(path: &Path, composition: &Composition) -> Result<(), SaveError> {
    let output = composition_to_string(composition)?;
    fs::write(path, output).map_err(SaveError::Io)
}

pub fn module_to_string(module: &Module) -> Result<String, SaveError> {
    ron::to_string(&FileModule::from_module(module)?).map_err(SaveError::Ron)
}

pub fn module_from_str(input: &str) -> Result<Module, LoadError> {
    let module: FileModule = ron::from_str(input).map_err(LoadError::Ron)?;
    module.into_module()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FilePatch {
    modules: Vec<FileModuleEntry>,
    connections: Vec<FileConnection>,
    output: FileOutput,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileComposition {
    name: String,
    patch: FilePatch,
    inputs: Vec<FileCompositionInput>,
    outputs: Vec<FileCompositionOutput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileModuleEntry {
    id: u32,
    module: FileModule,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileConnection {
    from: u32,
    output: u16,
    to: u32,
    input: InputKind,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct FileOutput {
    module: u32,
    output: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileCompositionOutput {
    label: String,
    port: FileOutput,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum FileModule {
    Input {
        kind: InputKind,
        default: f32,
    },
    Freq,
    Gate,
    Degree,
    Expression,
    Constant(f32),
    Unary {
        op: FileUnaryOp,
        input: f32,
    },
    Damp {
        input: f32,
        coefficient: f32,
    },
    Phase {
        frequency: f32,
    },
    Oscillator {
        waveform: Waveform,
        frequency: f32,
    },
    Saturation {
        curve: SaturationCurve,
        input: f32,
        drive: f32,
        asymmetry: f32,
    },
    Noise,
    Rise {
        gate: f32,
        time: FileDuration,
    },
    Fall {
        gate: f32,
        time: FileDuration,
    },
    Ramp {
        value: f32,
        time: FileDuration,
    },
    Envelope {
        phase: f32,
        points: Vec<FileEnvPoint>,
    },
    Filter {
        input: f32,
        cutoff: f32,
        resonance: f32,
    },
    Comb {
        input: f32,
        time: FileDuration,
        feedback: f32,
        damp: f32,
    },
    Allpass {
        input: f32,
        time: FileDuration,
        feedback: f32,
    },
    Delay {
        input: f32,
        time: FileDuration,
        feedback: f32,
    },
    DelayTap {
        delay: u32,
        gain: f32,
    },
    Slew {
        input: f32,
        rise: f32,
        fall: f32,
    },
    Binary {
        op: FileBinaryOp,
        a: f32,
        b: f32,
    },
    Switch {
        select: f32,
        a: f32,
        b: f32,
    },
    Random {
        gate: f32,
    },
    Sample {
        position: f32,
        samples: Vec<f32>,
    },
    Probe {
        input: f32,
    },
    Composition {
        name: String,
        patch: Box<FilePatch>,
        inputs: Vec<FileCompositionInput>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileCompositionInput {
    label: String,
    kind: InputKind,
    module: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct FileEnvPoint {
    time: f32,
    value: f32,
    curve: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum FileBinaryOp {
    Multiply,
    Add,
    Subtract,
    Divide,
    Power,
    Remainder,
    Minimum,
    Maximum,
    GreaterThan,
    LessThan,
    Equal,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum FileUnaryOp {
    Absolute,
    Sine,
    HyperbolicTangent,
    Arctangent,
    Exponential,
    Sign,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum FileDuration {
    Samples(u64),
    Seconds(f32),
}

impl FilePatch {
    fn from_patch(patch: &Patch) -> Result<Self, SaveError> {
        Ok(Self {
            modules: patch
                .modules()
                .iter()
                .map(|(id, module)| {
                    Ok(FileModuleEntry {
                        id: id.0,
                        module: FileModule::from_module(module)?,
                    })
                })
                .collect::<Result<Vec<_>, SaveError>>()?,
            connections: patch
                .connections()
                .iter()
                .map(|connection| FileConnection {
                    from: connection.from.module.0,
                    output: connection.from.output,
                    to: connection.input.module.0,
                    input: connection.input.input,
                })
                .collect(),
            output: {
                let output = patch.output_id().ok_or(SaveError::Value)?;
                FileOutput {
                    module: output.module.0,
                    output: output.output,
                }
            },
        })
    }

    fn into_patch(self) -> Result<Patch, LoadError> {
        self.into_patch_with_ids().map(|(patch, _)| patch)
    }

    fn into_patch_with_ids(self) -> Result<(Patch, HashMap<u32, patch::ModuleId>), LoadError> {
        let mut patch = Patch::new();
        let mut ids = HashMap::new();
        let mut taps = Vec::new();
        for entry in self.modules {
            let old_id = entry.id;
            if matches!(entry.module, FileModule::DelayTap { .. }) {
                taps.push(entry);
                continue;
            }
            let new_id = patch.insert(entry.module.into_module()?);
            if ids.insert(old_id, new_id).is_some() {
                return Err(LoadError::Value);
            }
        }
        for entry in taps {
            let FileModule::DelayTap { delay, gain } = entry.module else {
                unreachable!()
            };
            let delay = *ids.get(&delay).ok_or(LoadError::Value)?;
            let gain = Unit::new(gain).ok_or(LoadError::Value)?;
            let new_id = patch
                .insert_delay_tap(delay, gain)
                .map_err(|_| LoadError::Value)?;
            if ids.insert(entry.id, new_id).is_some() {
                return Err(LoadError::Value);
            }
        }
        for connection in self.connections {
            let from = *ids.get(&connection.from).ok_or(LoadError::Value)?;
            let from = patch
                .output_port(from, connection.output)
                .map_err(|_| LoadError::Value)?;
            let to = *ids.get(&connection.to).ok_or(LoadError::Value)?;
            let input = patch
                .input_port(to, connection.input)
                .map_err(|_| LoadError::Value)?;
            patch
                .connect_input(from, input)
                .map_err(|_| LoadError::Value)?;
        }
        let output = *ids.get(&self.output.module).ok_or(LoadError::Value)?;
        let output = patch
            .output_port(output, self.output.output)
            .map_err(|_| LoadError::Value)?;
        patch.output(output).map_err(|_| LoadError::Value)?;
        Ok((patch, ids))
    }
}

impl FileComposition {
    fn from_composition(composition: &Composition) -> Result<Self, SaveError> {
        Ok(Self {
            name: composition.name().to_string(),
            patch: FilePatch::from_patch(composition.patch())?,
            inputs: composition
                .inputs()
                .iter()
                .map(|input| FileCompositionInput {
                    label: input.label().to_string(),
                    kind: input.kind(),
                    module: input.module().0,
                })
                .collect(),
            outputs: composition
                .outputs()
                .iter()
                .map(|output| FileCompositionOutput {
                    label: output.label().to_string(),
                    port: FileOutput {
                        module: output.port().module().0,
                        output: output.port().index(),
                    },
                })
                .collect(),
        })
    }

    fn into_composition(self) -> Result<Composition, LoadError> {
        let (patch, ids) = self.patch.into_patch_with_ids()?;
        let inputs = self
            .inputs
            .into_iter()
            .map(|input| {
                Ok((
                    input.label,
                    input.kind,
                    *ids.get(&input.module).ok_or(LoadError::Value)?,
                ))
            })
            .collect::<Result<Vec<_>, LoadError>>()?;
        let outputs = self
            .outputs
            .into_iter()
            .map(|output| {
                let module = *ids.get(&output.port.module).ok_or(LoadError::Value)?;
                let port = patch
                    .output_port(module, output.port.output)
                    .map_err(|_| LoadError::Value)?;
                Ok((output.label, port))
            })
            .collect::<Result<Vec<_>, LoadError>>()?;
        Composition::new(self.name, patch, inputs, outputs).map_err(|_| LoadError::Value)
    }
}

impl FileModule {
    fn from_module(module: &Module) -> Result<Self, SaveError> {
        Ok(match module.clone() {
            Module::Input { kind, default } => FileModule::Input {
                kind,
                default: default.value(),
            },
            Module::Freq => FileModule::Freq,
            Module::Gate => FileModule::Gate,
            Module::Degree => FileModule::Degree,
            Module::Expression => FileModule::Expression,
            Module::Constant(value) => FileModule::Constant(value.value()),
            Module::Unary { op, input } => FileModule::Unary {
                op: match op {
                    UnaryOp::Absolute => FileUnaryOp::Absolute,
                    UnaryOp::Sine => FileUnaryOp::Sine,
                    UnaryOp::HyperbolicTangent => FileUnaryOp::HyperbolicTangent,
                    UnaryOp::Arctangent => FileUnaryOp::Arctangent,
                    UnaryOp::Exponential => FileUnaryOp::Exponential,
                    UnaryOp::Sign => FileUnaryOp::Sign,
                },
                input: input.value(),
            },
            Module::Damp { input, coefficient } => FileModule::Damp {
                input: input.value(),
                coefficient: coefficient.value(),
            },
            Module::Phase { frequency } => FileModule::Phase {
                frequency: frequency.value(),
            },
            Module::Oscillator {
                waveform,
                frequency,
            } => FileModule::Oscillator {
                waveform,
                frequency: frequency.value(),
            },
            Module::Saturation {
                curve,
                input,
                drive,
                asymmetry,
            } => FileModule::Saturation {
                curve,
                input: input.value(),
                drive: drive.value(),
                asymmetry: asymmetry.value(),
            },
            Module::Noise => FileModule::Noise,
            Module::Rise { gate, time } => FileModule::Rise {
                gate: gate.value(),
                time: FileDuration::from_duration(time),
            },
            Module::Fall { gate, time } => FileModule::Fall {
                gate: gate.value(),
                time: FileDuration::from_duration(time),
            },
            Module::Ramp { value, time } => FileModule::Ramp {
                value: value.value(),
                time: FileDuration::from_duration(time),
            },
            Module::Envelope { phase, points } => FileModule::Envelope {
                phase: phase.value(),
                points: points
                    .iter()
                    .map(|point| FileEnvPoint {
                        time: point.time.value(),
                        value: point.value.value(),
                        curve: point.curve,
                    })
                    .collect(),
            },
            Module::Filter {
                input,
                cutoff,
                resonance,
            } => FileModule::Filter {
                input: input.value(),
                cutoff: cutoff.value(),
                resonance: resonance.value(),
            },
            Module::Comb {
                input,
                time,
                feedback,
                damp,
            } => FileModule::Comb {
                input: input.value(),
                time: FileDuration::from_duration(time),
                feedback: feedback.value(),
                damp: damp.value(),
            },
            Module::Allpass {
                input,
                time,
                feedback,
            } => FileModule::Allpass {
                input: input.value(),
                time: FileDuration::from_duration(time),
                feedback: feedback.value(),
            },
            Module::Delay {
                input,
                time,
                feedback,
            } => FileModule::Delay {
                input: input.value(),
                time: FileDuration::from_duration(time),
                feedback: feedback.value(),
            },
            Module::DelayTap(tap) => FileModule::DelayTap {
                delay: tap.delay().0,
                gain: tap.gain().value(),
            },
            Module::Slew { input, rise, fall } => FileModule::Slew {
                input: input.value(),
                rise: rise.value(),
                fall: fall.value(),
            },
            Module::Binary { op, a, b } => FileModule::Binary {
                op: FileBinaryOp::from_binary_op(op),
                a: a.value(),
                b: b.value(),
            },
            Module::Switch { select, a, b } => FileModule::Switch {
                select: select.value(),
                a: a.value(),
                b: b.value(),
            },
            Module::Random { gate } => FileModule::Random { gate: gate.value() },
            Module::Sample { position, samples } => FileModule::Sample {
                position: position.value(),
                samples: samples.iter().map(|sample| sample.value()).collect(),
            },
            Module::Probe { input } => FileModule::Probe {
                input: input.value(),
            },
            Module::Composition(composition) => FileModule::Composition {
                name: composition.name().to_string(),
                patch: Box::new(FilePatch::from_patch(composition.patch())?),
                inputs: composition
                    .inputs()
                    .iter()
                    .map(|input| FileCompositionInput {
                        label: input.label().to_string(),
                        kind: input.kind(),
                        module: input.module().0,
                    })
                    .collect(),
            },
        })
    }

    fn into_module(self) -> Result<Module, LoadError> {
        Ok(match self {
            FileModule::Input { kind, default } => Module::Input {
                kind,
                default: Sample::new(default).ok_or(LoadError::Value)?,
            },
            FileModule::Freq => Module::Freq,
            FileModule::Gate => Module::Gate,
            FileModule::Degree => Module::Degree,
            FileModule::Expression => Module::Expression,
            FileModule::Constant(value) => {
                Module::Constant(Sample::new(value).ok_or(LoadError::Value)?)
            }
            FileModule::Unary { op, input } => Module::Unary {
                op: match op {
                    FileUnaryOp::Absolute => UnaryOp::Absolute,
                    FileUnaryOp::Sine => UnaryOp::Sine,
                    FileUnaryOp::HyperbolicTangent => UnaryOp::HyperbolicTangent,
                    FileUnaryOp::Arctangent => UnaryOp::Arctangent,
                    FileUnaryOp::Exponential => UnaryOp::Exponential,
                    FileUnaryOp::Sign => UnaryOp::Sign,
                },
                input: Sample::new(input).ok_or(LoadError::Value)?,
            },
            FileModule::Damp { input, coefficient } => Module::Damp {
                input: Sample::new(input).ok_or(LoadError::Value)?,
                coefficient: Unit::new(coefficient).ok_or(LoadError::Value)?,
            },
            FileModule::Phase { frequency } => Module::Phase {
                frequency: Hertz::new(frequency).ok_or(LoadError::Value)?,
            },
            FileModule::Oscillator {
                waveform,
                frequency,
            } => Module::Oscillator {
                waveform,
                frequency: Hertz::new(frequency).ok_or(LoadError::Value)?,
            },
            FileModule::Saturation {
                curve,
                input,
                drive,
                asymmetry,
            } => Module::Saturation {
                curve,
                input: Sample::new(input).ok_or(LoadError::Value)?,
                drive: Sample::new(drive).ok_or(LoadError::Value)?,
                asymmetry: Sample::new(asymmetry).ok_or(LoadError::Value)?,
            },
            FileModule::Noise => Module::Noise,
            FileModule::Rise { gate, time } => Module::Rise {
                gate: Sample::new(gate).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
            },
            FileModule::Fall { gate, time } => Module::Fall {
                gate: Sample::new(gate).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
            },
            FileModule::Ramp { value, time } => Module::Ramp {
                value: Sample::new(value).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
            },
            FileModule::Envelope { phase, points } => Module::Envelope {
                phase: Sample::new(phase).ok_or(LoadError::Value)?,
                points: Arc::new(
                    points
                        .into_iter()
                        .map(|point| {
                            Ok(EnvPoint {
                                time: Unit::new(point.time).ok_or(LoadError::Value)?,
                                value: Sample::new(point.value).ok_or(LoadError::Value)?,
                                curve: point.curve,
                            })
                        })
                        .collect::<Result<Vec<_>, LoadError>>()?,
                ),
            },
            FileModule::Filter {
                input,
                cutoff,
                resonance,
            } => Module::Filter {
                input: Sample::new(input).ok_or(LoadError::Value)?,
                cutoff: Hertz::new(cutoff).ok_or(LoadError::Value)?,
                resonance: crate::patch::Resonance::new(resonance).ok_or(LoadError::Value)?,
            },
            FileModule::Comb {
                input,
                time,
                feedback,
                damp,
            } => Module::Comb {
                input: Sample::new(input).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
                damp: Unit::new(damp).ok_or(LoadError::Value)?,
            },
            FileModule::Allpass {
                input,
                time,
                feedback,
            } => Module::Allpass {
                input: Sample::new(input).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
            },
            FileModule::Delay {
                input,
                time,
                feedback,
            } => Module::Delay {
                input: Sample::new(input).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
            },
            FileModule::DelayTap { .. } => return Err(LoadError::Value),
            FileModule::Slew { input, rise, fall } => Module::Slew {
                input: Sample::new(input).ok_or(LoadError::Value)?,
                rise: Seconds::new(rise).ok_or(LoadError::Value)?,
                fall: Seconds::new(fall).ok_or(LoadError::Value)?,
            },
            FileModule::Binary { op, a, b } => Module::Binary {
                op: op.into_binary_op(),
                a: Sample::new(a).ok_or(LoadError::Value)?,
                b: Sample::new(b).ok_or(LoadError::Value)?,
            },
            FileModule::Switch { select, a, b } => Module::Switch {
                select: Sample::new(select).ok_or(LoadError::Value)?,
                a: Sample::new(a).ok_or(LoadError::Value)?,
                b: Sample::new(b).ok_or(LoadError::Value)?,
            },
            FileModule::Random { gate } => Module::Random {
                gate: Sample::new(gate).ok_or(LoadError::Value)?,
            },
            FileModule::Sample { position, samples } => Module::Sample {
                position: Sample::new(position).ok_or(LoadError::Value)?,
                samples: Arc::new(
                    samples
                        .into_iter()
                        .map(|sample| Sample::new(sample).ok_or(LoadError::Value))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            },
            FileModule::Probe { input } => Module::Probe {
                input: Sample::new(input).ok_or(LoadError::Value)?,
            },
            FileModule::Composition {
                name,
                patch,
                inputs,
            } => {
                let (patch, ids) = patch.into_patch_with_ids()?;
                let output = patch.output_id().ok_or(LoadError::Value)?;
                let inputs = inputs
                    .into_iter()
                    .map(|input| {
                        Ok((
                            input.label,
                            input.kind,
                            *ids.get(&input.module).ok_or(LoadError::Value)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, LoadError>>()?;
                Module::Composition(Box::new(
                    Composition::new(name, patch, inputs, [("Output".to_string(), output)])
                        .map_err(|_| LoadError::Value)?,
                ))
            }
        })
    }
}

impl FileBinaryOp {
    fn from_binary_op(op: BinaryOp) -> Self {
        match op {
            BinaryOp::Multiply => FileBinaryOp::Multiply,
            BinaryOp::Add => FileBinaryOp::Add,
            BinaryOp::Subtract => FileBinaryOp::Subtract,
            BinaryOp::Divide => FileBinaryOp::Divide,
            BinaryOp::Power => FileBinaryOp::Power,
            BinaryOp::Remainder => FileBinaryOp::Remainder,
            BinaryOp::Minimum => FileBinaryOp::Minimum,
            BinaryOp::Maximum => FileBinaryOp::Maximum,
            BinaryOp::GreaterThan => FileBinaryOp::GreaterThan,
            BinaryOp::LessThan => FileBinaryOp::LessThan,
            BinaryOp::Equal => FileBinaryOp::Equal,
        }
    }

    fn into_binary_op(self) -> BinaryOp {
        match self {
            FileBinaryOp::Multiply => BinaryOp::Multiply,
            FileBinaryOp::Add => BinaryOp::Add,
            FileBinaryOp::Subtract => BinaryOp::Subtract,
            FileBinaryOp::Divide => BinaryOp::Divide,
            FileBinaryOp::Power => BinaryOp::Power,
            FileBinaryOp::Remainder => BinaryOp::Remainder,
            FileBinaryOp::Minimum => BinaryOp::Minimum,
            FileBinaryOp::Maximum => BinaryOp::Maximum,
            FileBinaryOp::GreaterThan => BinaryOp::GreaterThan,
            FileBinaryOp::LessThan => BinaryOp::LessThan,
            FileBinaryOp::Equal => BinaryOp::Equal,
        }
    }
}

impl FileDuration {
    fn from_duration(duration: Duration) -> Self {
        match duration {
            Duration::Samples(samples) => FileDuration::Samples(samples.value()),
            Duration::Seconds(seconds) => FileDuration::Seconds(seconds.value()),
        }
    }

    fn into_duration(self) -> Result<Duration, LoadError> {
        Ok(match self {
            FileDuration::Samples(samples) => Duration::Samples(Samples::new(samples)),
            FileDuration::Seconds(seconds) => {
                Duration::Seconds(Seconds::new(seconds).ok_or(LoadError::Value)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::CompiledPatch;
    use crate::patch::{CompressorRatio, Gain};
    use crate::time::SampleRate;

    #[test]
    fn multi_input_composition_round_trips() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(Sample::new(0.5).unwrap()));
        let compressor = patch.insert(crate::preset::compressor(
            Unit::new(0.5).unwrap(),
            CompressorRatio::new(2.0).unwrap(),
            Seconds::new(0.001).unwrap(),
            Seconds::new(0.001).unwrap(),
            Gain::new(1.0).unwrap(),
        ));
        let input = patch.input_port(compressor, InputKind::In).unwrap();
        patch
            .connect_input(patch.output_port(source, 0).unwrap(), input)
            .unwrap();
        patch
            .output(patch.output_port(compressor, 0).unwrap())
            .unwrap();

        let encoded = to_string(&patch).unwrap();
        let decoded = from_str(&encoded).unwrap();
        let mut compiled = CompiledPatch::new(&decoded, SampleRate::new(44_100).unwrap()).unwrap();

        assert!(compiled.next().left().value().is_finite());
        assert_eq!(to_string(&decoded).unwrap(), encoded);
    }

    #[test]
    fn composition_file_preserves_name_and_labels() {
        let mut patch = Patch::new();
        let input = patch.insert(Module::Input {
            kind: InputKind::In,
            default: Sample::ZERO,
        });
        let output = patch.output_port(input, 0).unwrap();
        patch.output(output).unwrap();
        let composition = Composition::new(
            "User Gain",
            patch,
            [("Signal".to_string(), InputKind::In, input)],
            [("Output".to_string(), output)],
        )
        .unwrap();

        let encoded = composition_to_string(&composition).unwrap();
        let decoded = composition_from_str(&encoded).unwrap();

        assert_eq!(decoded.name(), "User Gain");
        assert_eq!(decoded.inputs()[0].label(), "Signal");
        assert_eq!(composition_to_string(&decoded).unwrap(), encoded);
    }

    #[test]
    fn composition_file_preserves_output_order_and_sources() {
        let mut patch = Patch::new();
        let first = patch.insert(Module::Constant(Sample::new(0.25).unwrap()));
        let second = patch.insert(Module::Constant(Sample::new(0.75).unwrap()));
        let first_output = patch.output_port(first, 0).unwrap();
        let second_output = patch.output_port(second, 0).unwrap();
        patch.output(first_output).unwrap();
        let composition = Composition::new(
            "Pair",
            patch,
            Vec::<(String, InputKind, crate::patch::ModuleId)>::new(),
            [
                ("First".to_string(), first_output),
                ("Second".to_string(), second_output),
            ],
        )
        .unwrap();

        let decoded = composition_from_str(&composition_to_string(&composition).unwrap()).unwrap();

        assert_eq!(
            decoded
                .outputs()
                .iter()
                .map(|output| output.label())
                .collect::<Vec<_>>(),
            ["First", "Second"]
        );
        assert_ne!(decoded.outputs()[0].port(), decoded.outputs()[1].port());
    }

    #[test]
    fn variant_parameters_round_trip_and_reject_unknown_variants() {
        for waveform in [
            Waveform::Sine,
            Waveform::Square,
            Waveform::Triangle,
            Waveform::Saw,
            Waveform::ReverseSaw,
        ] {
            let module = Module::Oscillator {
                waveform,
                frequency: Hertz::new(440.0).unwrap(),
            };
            assert_eq!(
                module_from_str(&module_to_string(&module).unwrap()).unwrap(),
                module
            );
        }
        for curve in [
            SaturationCurve::Tube,
            SaturationCurve::Tape,
            SaturationCurve::Fuzz,
            SaturationCurve::Fold,
            SaturationCurve::Clip,
        ] {
            let module = Module::Saturation {
                curve,
                input: Sample::new(-0.3).unwrap(),
                drive: Sample::new(2.0).unwrap(),
                asymmetry: Sample::new(0.1).unwrap(),
            };
            assert_eq!(
                module_from_str(&module_to_string(&module).unwrap()).unwrap(),
                module
            );
        }
        assert!(module_from_str("Oscillator(waveform:Unknown,frequency:440.0)").is_err());
        assert!(
            module_from_str("Saturation(curve:Unknown,input:0.0,drive:2.0,asymmetry:0.0)").is_err()
        );
    }

    #[test]
    fn module_round_trips_without_a_patch_wrapper() {
        for module in [
            Module::Phase {
                frequency: Hertz::new(220.0).unwrap(),
            },
            Module::Noise,
            crate::preset::tape(),
            Module::Filter {
                input: Sample::ZERO,
                cutoff: Hertz::new(1_000.0).unwrap(),
                resonance: crate::patch::Resonance::new(8.0).unwrap(),
            },
        ] {
            let encoded = module_to_string(&module).unwrap();
            let decoded = module_from_str(&encoded).unwrap();
            assert_eq!(decoded, module);
            if matches!(module, Module::Filter { .. }) {
                assert!(encoded.contains("Filter"));
                assert!(!encoded.contains("Lowpass"));
                assert!(!encoded.contains("Highpass"));
            }
        }
    }

    #[test]
    fn bipolar_envelope_round_trips() {
        let module = Module::Envelope {
            phase: Sample::ZERO,
            points: Arc::new(vec![EnvPoint::new(
                Unit::new(0.5).unwrap(),
                Sample::new(-0.75).unwrap(),
                true,
            )]),
        };

        let encoded = module_to_string(&module).unwrap();
        let decoded = module_from_str(&encoded).unwrap();

        assert_eq!(decoded, module);
    }

    #[test]
    fn delay_tap_round_trip_preserves_delay_identity() {
        let mut patch = Patch::new();
        let delay = patch.insert(Module::Delay {
            input: Sample::ZERO,
            time: Duration::Samples(Samples::new(2)),
            feedback: Unit::ZERO,
        });
        let tap = patch
            .insert_delay_tap(delay, Unit::new(0.5).unwrap())
            .unwrap();
        patch.output(patch.output_port(tap, 0).unwrap()).unwrap();

        let decoded = from_str(&to_string(&patch).unwrap()).unwrap();
        let (_, Module::DelayTap(tap)) = decoded
            .module_entries()
            .find(|(_, module)| matches!(module, Module::DelayTap(_)))
            .unwrap()
        else {
            unreachable!()
        };

        assert!(matches!(
            decoded.module_by_id(tap.delay()),
            Some(Module::Delay { .. })
        ));
        assert_eq!(tap.gain(), Unit::new(0.5).unwrap());
    }
}
