use crate::patch::{self, BinaryOp, CompressorRatio, EnvPoint, Gain, Module, Patch};
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FilePatch {
    modules: Vec<FileModuleEntry>,
    connections: Vec<FileConnection>,
    output: u32,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum FileModule {
    Freq,
    Gate,
    Degree,
    DegreeGate {
        target: i32,
    },
    Constant(f32),
    Pass,
    Osc {
        wave: FileWave,
        frequency: f32,
        #[serde(default)]
        shift: f32,
        #[serde(default = "one")]
        gain: f32,
        #[serde(default)]
        unipolar: bool,
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
    Adsr {
        attack: FileDuration,
        decay: FileDuration,
        sustain: f32,
        release: FileDuration,
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
    DelayTap {
        gain: f32,
    },
    Reverb {
        room: f32,
        damp: f32,
        mod_depth: f32,
        diffusion: f32,
    },
    Distortion {
        kind: FileDistortion,
        drive: f32,
    },
    Compressor {
        threshold: f32,
        ratio: f32,
        attack: FileDuration,
        release: FileDuration,
        makeup: f32,
    },
    Flanger {
        rate: f32,
        depth: f32,
        feedback: f32,
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

fn one() -> f32 {
    1.0
}

impl FilePatch {
    fn from_patch(patch: &Patch) -> Result<Self, SaveError> {
        Ok(Self {
            modules: patch
                .modules()
                .iter()
                .map(|(id, module)| FileModuleEntry {
                    id: id.0,
                    module: FileModule::from_module(module),
                })
                .collect(),
            connections: patch
                .connections()
                .iter()
                .map(|connection| FileConnection {
                    from: connection.from.0,
                    to: connection.input.module.0,
                })
                .collect(),
            output: patch.output_id().ok_or(SaveError::Value)?.0,
        })
    }

    fn into_patch(self) -> Result<Patch, LoadError> {
        let mut patch = Patch::new();
        let mut ids = HashMap::new();
        for entry in self.modules {
            let old_id = entry.id;
            let new_id = patch.insert(entry.module.into_module()?);
            ids.insert(old_id, new_id);
        }
        for connection in self.connections {
            let from = *ids.get(&connection.from).ok_or(LoadError::Value)?;
            let to = *ids.get(&connection.to).ok_or(LoadError::Value)?;
            patch.connect(from, to).map_err(|_| LoadError::Value)?;
        }
        let output = *ids.get(&self.output).ok_or(LoadError::Value)?;
        patch.output(output).map_err(|_| LoadError::Value)?;
        Ok(patch)
    }
}

impl FileModule {
    fn from_module(module: &Module) -> Self {
        match module.clone() {
            Module::Freq => FileModule::Freq,
            Module::Gate => FileModule::Gate,
            Module::Degree => FileModule::Degree,
            Module::DegreeGate { target } => FileModule::DegreeGate { target },
            Module::Constant(value) => FileModule::Constant(value.value()),
            Module::Pass => FileModule::Pass,
            Module::Osc {
                wave,
                frequency,
                shift,
                gain,
                unipolar,
            } => FileModule::Osc {
                wave: FileWave::from_wave(wave),
                frequency: frequency.value(),
                shift: shift.value(),
                gain: gain.value(),
                unipolar,
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
            Module::Adsr {
                attack_ratio,
                sustain,
            } => FileModule::Adsr {
                attack: FileDuration::Seconds(attack_ratio.value()),
                decay: FileDuration::Seconds(1.0 - attack_ratio.value()),
                sustain: sustain.value(),
                release: FileDuration::Seconds(1.0),
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
            Module::DelayTap { gain } => FileModule::DelayTap { gain: gain.value() },
            Module::Reverb {
                room,
                damp,
                mod_depth,
                diffusion,
            } => FileModule::Reverb {
                room: room.value(),
                damp: damp.value(),
                mod_depth: mod_depth.value(),
                diffusion: diffusion.value(),
            },
            Module::Distortion { kind, drive } => FileModule::Distortion {
                kind: FileDistortion::from_distortion(kind),
                drive: drive.value(),
            },
            Module::Compressor {
                threshold,
                ratio,
                attack,
                release,
                makeup,
            } => FileModule::Compressor {
                threshold: threshold.value(),
                ratio: ratio.value(),
                attack: FileDuration::from_duration(attack),
                release: FileDuration::from_duration(release),
                makeup: makeup.value(),
            },
            Module::Flanger {
                rate,
                depth,
                feedback,
            } => FileModule::Flanger {
                rate: rate.value(),
                depth: depth.value(),
                feedback: feedback.value(),
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
        }
    }

    fn into_module(self) -> Result<Module, LoadError> {
        Ok(match self {
            FileModule::Freq => Module::Freq,
            FileModule::Gate => Module::Gate,
            FileModule::Degree => Module::Degree,
            FileModule::DegreeGate { target } => Module::DegreeGate { target },
            FileModule::Constant(value) => {
                Module::Constant(Sample::new(value).ok_or(LoadError::Value)?)
            }
            FileModule::Pass => Module::Pass,
            FileModule::Osc {
                wave,
                frequency,
                shift,
                gain,
                unipolar,
            } => Module::Osc {
                wave: wave.into_wave(),
                frequency: Hertz::new(frequency).ok_or(LoadError::Value)?,
                shift: Sample::new(shift).ok_or(LoadError::Value)?,
                gain: Unit::new(gain).ok_or(LoadError::Value)?,
                unipolar,
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
            FileModule::Adsr {
                attack,
                decay: _,
                sustain,
                release: _,
            } => Module::Adsr {
                attack_ratio: duration_unit(attack.into_duration()?),
                sustain: Unit::new(sustain).ok_or(LoadError::Value)?,
            },
            FileModule::Envelope { points } => Module::Envelope {
                points: Arc::new(
                    points
                        .into_iter()
                        .map(|point| {
                            Ok(EnvPoint {
                                time: Unit::new(point.time).ok_or(LoadError::Value)?,
                                value: Unit::new(point.value).ok_or(LoadError::Value)?,
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
            FileModule::DelayTap { gain } => Module::DelayTap {
                gain: Unit::new(gain).ok_or(LoadError::Value)?,
            },
            FileModule::Reverb {
                room,
                damp,
                mod_depth,
                diffusion,
            } => Module::Reverb {
                room: Unit::new(room).ok_or(LoadError::Value)?,
                damp: Unit::new(damp).ok_or(LoadError::Value)?,
                mod_depth: Unit::new(mod_depth).ok_or(LoadError::Value)?,
                diffusion: Unit::new(diffusion).ok_or(LoadError::Value)?,
            },
            FileModule::Distortion { kind, drive } => Module::Distortion {
                kind: kind.into_distortion(),
                drive: patch::Drive::new(drive).ok_or(LoadError::Value)?,
            },
            FileModule::Compressor {
                threshold,
                ratio,
                attack,
                release,
                makeup,
            } => Module::Compressor {
                threshold: Unit::new(threshold).ok_or(LoadError::Value)?,
                ratio: CompressorRatio::new(ratio).ok_or(LoadError::Value)?,
                attack: attack.into_duration()?,
                release: release.into_duration()?,
                makeup: Gain::new(makeup).ok_or(LoadError::Value)?,
            },
            FileModule::Flanger {
                rate,
                depth,
                feedback,
            } => Module::Flanger {
                rate: Hertz::new(rate).ok_or(LoadError::Value)?,
                depth: Unit::new(depth).ok_or(LoadError::Value)?,
                feedback: Unit::new(feedback).ok_or(LoadError::Value)?,
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
        })
    }
}

impl FileWave {
    fn from_wave(wave: patch::Wave) -> Self {
        match wave {
            patch::Wave::Sine => FileWave::Sine,
            patch::Wave::Square => FileWave::Square,
            patch::Wave::Triangle => FileWave::Triangle,
            patch::Wave::Saw => FileWave::Saw,
            patch::Wave::Noise => FileWave::Noise,
        }
    }

    fn into_wave(self) -> patch::Wave {
        match self {
            FileWave::Sine => patch::Wave::Sine,
            FileWave::Square => patch::Wave::Square,
            FileWave::Triangle => patch::Wave::Triangle,
            FileWave::Saw => patch::Wave::Saw,
            FileWave::Noise => patch::Wave::Noise,
        }
    }
}

impl FileDistortion {
    fn from_distortion(kind: patch::Distortion) -> Self {
        match kind {
            patch::Distortion::Clip => FileDistortion::Clip,
            patch::Distortion::Tanh => FileDistortion::Tanh,
            patch::Distortion::Fold => FileDistortion::Fold,
        }
    }

    fn into_distortion(self) -> patch::Distortion {
        match self {
            FileDistortion::Clip => patch::Distortion::Clip,
            FileDistortion::Tanh => patch::Distortion::Tanh,
            FileDistortion::Fold => patch::Distortion::Fold,
        }
    }
}

impl FileBinaryOp {
    fn from_binary_op(op: BinaryOp) -> Self {
        match op {
            BinaryOp::Multiply => FileBinaryOp::Multiply,
            BinaryOp::Add => FileBinaryOp::Add,
            BinaryOp::GreaterThan => FileBinaryOp::GreaterThan,
            BinaryOp::LessThan => FileBinaryOp::LessThan,
        }
    }

    fn into_binary_op(self) -> BinaryOp {
        match self {
            FileBinaryOp::Multiply => BinaryOp::Multiply,
            FileBinaryOp::Add => BinaryOp::Add,
            FileBinaryOp::GreaterThan => BinaryOp::GreaterThan,
            FileBinaryOp::LessThan => BinaryOp::LessThan,
        }
    }
}

fn duration_unit(duration: Duration) -> Unit {
    let value = match duration {
        Duration::Seconds(seconds) => seconds.value(),
        Duration::Samples(samples) => samples.value() as f32,
    };
    Unit::new(value.clamp(0.0, 1.0)).unwrap()
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
