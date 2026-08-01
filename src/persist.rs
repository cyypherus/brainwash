use crate::patch::{self, BinaryOp, Composition, EnvPoint, InputKind, Module, Patch, UnaryOp};
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
    DegreeGate {
        target: i32,
    },
    Constant(f32),
    Unary(FileUnaryOp),
    Pass,
    Damp {
        coefficient: f32,
    },
    Phase {
        frequency: f32,
    },
    Noise,
    Rise {
        time: FileDuration,
    },
    Fall {
        time: FileDuration,
    },
    Ramp {
        value: f32,
        time: FileDuration,
    },
    Envelope {
        points: Vec<FileEnvPoint>,
    },
    Filter {
        cutoff: f32,
        resonance: f32,
    },
    Comb {
        time: FileDuration,
        feedback: f32,
        damp: f32,
    },
    Allpass {
        time: FileDuration,
        feedback: f32,
    },
    Delay {
        time: FileDuration,
        feedback: f32,
    },
    DelayTap {
        delay: u32,
        gain: f32,
    },
    VariableDelay {
        max_time: FileDuration,
    },
    Slew {
        rise: f32,
        fall: f32,
    },
    Binary {
        op: FileBinaryOp,
        a: f32,
        b: f32,
    },
    Switch {
        a: f32,
        b: f32,
    },
    Random,
    Sample {
        samples: Vec<f32>,
    },
    Probe,
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
            Module::DegreeGate { target } => FileModule::DegreeGate { target },
            Module::Constant(value) => FileModule::Constant(value.value()),
            Module::Unary(op) => FileModule::Unary(match op {
                UnaryOp::Absolute => FileUnaryOp::Absolute,
                UnaryOp::Sine => FileUnaryOp::Sine,
                UnaryOp::HyperbolicTangent => FileUnaryOp::HyperbolicTangent,
                UnaryOp::Arctangent => FileUnaryOp::Arctangent,
                UnaryOp::Exponential => FileUnaryOp::Exponential,
                UnaryOp::Sign => FileUnaryOp::Sign,
            }),
            Module::Pass => FileModule::Pass,
            Module::Damp { coefficient } => FileModule::Damp {
                coefficient: coefficient.value(),
            },
            Module::Phase { frequency } => FileModule::Phase {
                frequency: frequency.value(),
            },
            Module::Noise => FileModule::Noise,
            Module::Rise { time } => FileModule::Rise {
                time: FileDuration::from_duration(time),
            },
            Module::Fall { time } => FileModule::Fall {
                time: FileDuration::from_duration(time),
            },
            Module::Ramp { value, time } => FileModule::Ramp {
                value: value.value(),
                time: FileDuration::from_duration(time),
            },
            Module::Envelope { points } => FileModule::Envelope {
                points: points
                    .iter()
                    .map(|point| FileEnvPoint {
                        time: point.time.value(),
                        value: point.value.value(),
                        curve: point.curve,
                    })
                    .collect(),
            },
            Module::Filter { cutoff, resonance } => FileModule::Filter {
                cutoff: cutoff.value(),
                resonance: resonance.value(),
            },
            Module::Comb {
                time,
                feedback,
                damp,
            } => FileModule::Comb {
                time: FileDuration::from_duration(time),
                feedback: feedback.value(),
                damp: damp.value(),
            },
            Module::Allpass { time, feedback } => FileModule::Allpass {
                time: FileDuration::from_duration(time),
                feedback: feedback.value(),
            },
            Module::Delay { time, feedback } => FileModule::Delay {
                time: FileDuration::from_duration(time),
                feedback: feedback.value(),
            },
            Module::DelayTap(tap) => FileModule::DelayTap {
                delay: tap.delay().0,
                gain: tap.gain().value(),
            },
            Module::VariableDelay { max_time } => FileModule::VariableDelay {
                max_time: FileDuration::from_duration(max_time),
            },
            Module::Slew { rise, fall } => FileModule::Slew {
                rise: rise.value(),
                fall: fall.value(),
            },
            Module::Binary { op, a, b } => FileModule::Binary {
                op: FileBinaryOp::from_binary_op(op),
                a: a.value(),
                b: b.value(),
            },
            Module::Switch { a, b } => FileModule::Switch {
                a: a.value(),
                b: b.value(),
            },
            Module::Random => FileModule::Random,
            Module::Sample { samples } => FileModule::Sample {
                samples: samples.iter().map(|sample| sample.value()).collect(),
            },
            Module::Probe => FileModule::Probe,
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
            FileModule::DegreeGate { target } => Module::DegreeGate { target },
            FileModule::Constant(value) => {
                Module::Constant(Sample::new(value).ok_or(LoadError::Value)?)
            }
            FileModule::Unary(op) => Module::Unary(match op {
                FileUnaryOp::Absolute => UnaryOp::Absolute,
                FileUnaryOp::Sine => UnaryOp::Sine,
                FileUnaryOp::HyperbolicTangent => UnaryOp::HyperbolicTangent,
                FileUnaryOp::Arctangent => UnaryOp::Arctangent,
                FileUnaryOp::Exponential => UnaryOp::Exponential,
                FileUnaryOp::Sign => UnaryOp::Sign,
            }),
            FileModule::Pass => Module::Pass,
            FileModule::Damp { coefficient } => Module::Damp {
                coefficient: Unit::new(coefficient).ok_or(LoadError::Value)?,
            },
            FileModule::Phase { frequency } => Module::Phase {
                frequency: Hertz::new(frequency).ok_or(LoadError::Value)?,
            },
            FileModule::Noise => Module::Noise,
            FileModule::Rise { time } => Module::Rise {
                time: time.into_duration()?,
            },
            FileModule::Fall { time } => Module::Fall {
                time: time.into_duration()?,
            },
            FileModule::Ramp { value, time } => Module::Ramp {
                value: Sample::new(value).ok_or(LoadError::Value)?,
                time: time.into_duration()?,
            },
            FileModule::Envelope { points } => Module::Envelope {
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
            FileModule::Filter { cutoff, resonance } => Module::Filter {
                cutoff: Hertz::new(cutoff).ok_or(LoadError::Value)?,
                resonance: crate::patch::Resonance::new(resonance).ok_or(LoadError::Value)?,
            },
            FileModule::Comb {
                time,
                feedback,
                damp,
            } => Module::Comb {
                time: time.into_duration()?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
                damp: Unit::new(damp).ok_or(LoadError::Value)?,
            },
            FileModule::Allpass { time, feedback } => Module::Allpass {
                time: time.into_duration()?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
            },
            FileModule::Delay { time, feedback } => Module::Delay {
                time: time.into_duration()?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
            },
            FileModule::DelayTap { .. } => return Err(LoadError::Value),
            FileModule::VariableDelay { max_time } => Module::VariableDelay {
                max_time: max_time.into_duration()?,
            },
            FileModule::Slew { rise, fall } => Module::Slew {
                rise: Seconds::new(rise).ok_or(LoadError::Value)?,
                fall: Seconds::new(fall).ok_or(LoadError::Value)?,
            },
            FileModule::Binary { op, a, b } => Module::Binary {
                op: op.into_binary_op(),
                a: Sample::new(a).ok_or(LoadError::Value)?,
                b: Sample::new(b).ok_or(LoadError::Value)?,
            },
            FileModule::Switch { a, b } => Module::Switch {
                a: Sample::new(a).ok_or(LoadError::Value)?,
                b: Sample::new(b).ok_or(LoadError::Value)?,
            },
            FileModule::Random => Module::Random,
            FileModule::Sample { samples } => Module::Sample {
                samples: Arc::new(
                    samples
                        .into_iter()
                        .map(|sample| Sample::new(sample).ok_or(LoadError::Value))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            },
            FileModule::Probe => Module::Probe,
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
    fn module_round_trips_without_a_patch_wrapper() {
        for module in [
            Module::Phase {
                frequency: Hertz::new(220.0).unwrap(),
            },
            Module::Noise,
            crate::preset::tape(),
            Module::Filter {
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
