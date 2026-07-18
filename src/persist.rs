use crate::effect::Distortion;
use crate::osc::Wave;
use crate::patch::{self, BinaryOp, Composition, EnvPoint, InputKind, Module, Patch};
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
    output: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileComposition {
    name: String,
    patch: FilePatch,
    inputs: Vec<FileCompositionInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileModuleEntry {
    id: u32,
    module: FileModule,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileConnection {
    from: u32,
    to: u32,
    input: InputKind,
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
    Absolute,
    Pass,
    Osc {
        wave: FileWave,
        frequency: f32,
    },
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
    Lowpass {
        cutoff: f32,
    },
    Highpass {
        cutoff: f32,
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
    VariableDelay {
        max_time: FileDuration,
    },
    Waveshaper(FileDistortion),
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
    GreaterThan,
    LessThan,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum FileWave {
    Sine,
    Square,
    Triangle,
    Saw,
    Noise,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum FileDistortion {
    Clip,
    Tanh,
    Fold,
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
                    from: connection.from.0,
                    to: connection.input.module.0,
                    input: connection.input.input,
                })
                .collect(),
            output: patch.output_id().ok_or(SaveError::Value)?.0,
        })
    }

    fn into_patch(self) -> Result<Patch, LoadError> {
        self.into_patch_with_ids().map(|(patch, _)| patch)
    }

    fn into_patch_with_ids(self) -> Result<(Patch, HashMap<u32, patch::ModuleId>), LoadError> {
        let mut patch = Patch::new();
        let mut ids = HashMap::new();
        for entry in self.modules {
            let old_id = entry.id;
            let new_id = patch.insert(entry.module.into_module()?);
            if ids.insert(old_id, new_id).is_some() {
                return Err(LoadError::Value);
            }
        }
        for connection in self.connections {
            let from = *ids.get(&connection.from).ok_or(LoadError::Value)?;
            let to = *ids.get(&connection.to).ok_or(LoadError::Value)?;
            let input = patch
                .input_port(to, connection.input)
                .map_err(|_| LoadError::Value)?;
            patch
                .connect_input(from, input)
                .map_err(|_| LoadError::Value)?;
        }
        let output = *ids.get(&self.output).ok_or(LoadError::Value)?;
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
        Composition::new(self.name, patch, inputs).map_err(|_| LoadError::Value)
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
            Module::Absolute => FileModule::Absolute,
            Module::Pass => FileModule::Pass,
            Module::Osc { wave, frequency } => FileModule::Osc {
                wave: FileWave::from_wave(wave),
                frequency: frequency.value(),
            },
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
            Module::Lowpass { cutoff } => FileModule::Lowpass {
                cutoff: cutoff.value(),
            },
            Module::Highpass { cutoff } => FileModule::Highpass {
                cutoff: cutoff.value(),
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
            Module::VariableDelay { max_time } => FileModule::VariableDelay {
                max_time: FileDuration::from_duration(max_time),
            },
            Module::Waveshaper(kind) => {
                FileModule::Waveshaper(FileDistortion::from_distortion(kind))
            }
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
            FileModule::Absolute => Module::Absolute,
            FileModule::Pass => Module::Pass,
            FileModule::Osc { wave, frequency } => Module::Osc {
                wave: wave.into_wave(),
                frequency: Hertz::new(frequency).ok_or(LoadError::Value)?,
            },
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
            FileModule::Lowpass { cutoff } => Module::Lowpass {
                cutoff: Hertz::new(cutoff).ok_or(LoadError::Value)?,
            },
            FileModule::Highpass { cutoff } => Module::Highpass {
                cutoff: Hertz::new(cutoff).ok_or(LoadError::Value)?,
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
            FileModule::VariableDelay { max_time } => Module::VariableDelay {
                max_time: max_time.into_duration()?,
            },
            FileModule::Waveshaper(kind) => Module::Waveshaper(kind.into_distortion()),
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
                    Composition::new(name, patch, inputs).map_err(|_| LoadError::Value)?,
                ))
            }
        })
    }
}

impl FileWave {
    fn from_wave(wave: Wave) -> Self {
        match wave {
            Wave::Sine => FileWave::Sine,
            Wave::Square => FileWave::Square,
            Wave::Triangle => FileWave::Triangle,
            Wave::Saw => FileWave::Saw,
            Wave::Noise => FileWave::Noise,
        }
    }

    fn into_wave(self) -> Wave {
        match self {
            FileWave::Sine => Wave::Sine,
            FileWave::Square => Wave::Square,
            FileWave::Triangle => Wave::Triangle,
            FileWave::Saw => Wave::Saw,
            FileWave::Noise => Wave::Noise,
        }
    }
}

impl FileDistortion {
    fn from_distortion(kind: Distortion) -> Self {
        match kind {
            Distortion::Clip => FileDistortion::Clip,
            Distortion::Tanh => FileDistortion::Tanh,
            Distortion::Fold => FileDistortion::Fold,
        }
    }

    fn into_distortion(self) -> Distortion {
        match self {
            FileDistortion::Clip => Distortion::Clip,
            FileDistortion::Tanh => Distortion::Tanh,
            FileDistortion::Fold => Distortion::Fold,
        }
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
        patch.connect_input(source, input).unwrap();
        patch.output(compressor).unwrap();

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
        patch.output(input).unwrap();
        let composition = Composition::new(
            "User Gain",
            patch,
            [("Signal".to_string(), InputKind::In, input)],
        )
        .unwrap();

        let encoded = composition_to_string(&composition).unwrap();
        let decoded = composition_from_str(&encoded).unwrap();

        assert_eq!(decoded.name(), "User Gain");
        assert_eq!(decoded.inputs()[0].label(), "Signal");
        assert_eq!(composition_to_string(&decoded).unwrap(), encoded);
    }

    #[test]
    fn module_round_trips_without_a_patch_wrapper() {
        let module = Module::Osc {
            wave: Wave::Triangle,
            frequency: Hertz::new(220.0).unwrap(),
        };

        let encoded = module_to_string(&module).unwrap();
        let decoded = module_from_str(&encoded).unwrap();

        assert_eq!(decoded, module);
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
}
