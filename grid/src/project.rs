use crate::{ModuleId, Orientation};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    #[serde(default = "default_bars")]
    pub bars: f32,
    #[serde(default)]
    pub scale_idx: usize,
    #[serde(default)]
    pub modules: Vec<ModuleDef>,
    #[serde(default)]
    pub track: Option<String>,
    #[serde(default)]
    pub compositions: Vec<CompositionDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompositionDef {
    pub id: u32,
    pub name: String,
    pub color: (u8, u8, u8),
    pub modules: Vec<ModuleDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDef {
    pub id: ModuleId,
    pub kind: ModuleKind,
    pub x: u16,
    pub y: u16,
    #[serde(default, skip_serializing_if = "is_right")]
    pub orientation: Orientation,
    pub params: ModuleParams,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompositionId(u32);

impl CompositionId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingModule {
    LSplit,
    TSplit,
    RJoin,
    DJoin,
    TurnRD,
    TurnDR,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositionModule {
    Input,
    Output,
    Composition(CompositionId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StandardModule {
    Primitive,
    Freq,
    Gate,
    Degree,
    DegreeGate,
    Rate,
    Phase,
    Noise,
    Rise,
    Fall,
    Ramp,
    Envelope,
    Comb,
    Allpass,
    Delay,
    DelayTap(ModuleId),
    Rng,
    Sample,
    Probe,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    Routing(RoutingModule),
    Composition(CompositionModule),
    Standard(StandardModule),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimeUnit {
    Seconds,
    Samples,
    Bars,
    Hz,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeValue {
    pub unit: TimeUnit,
    pub seconds: f32,
    pub samples: f32,
    pub bar_num: u8,
    pub bar_denom: u8,
    pub hz: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvPoint {
    pub time: f32,
    pub value: f32,
    pub curve: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ModuleParams {
    None,
    Primitive {
        source: String,
    },
    DegreeGate {
        degree: i32,
    },
    Rate {
        time: TimeValue,
    },
    Phase {
        frequency: f32,
        connected: u8,
    },
    Rise {
        time: TimeValue,
        connected: u8,
    },
    Fall {
        time: TimeValue,
        connected: u8,
    },
    Ramp {
        value: f32,
        time: TimeValue,
        connected: u8,
    },
    Envelope {
        points: Vec<EnvPoint>,
        connected: u8,
    },
    Comb {
        time: TimeValue,
        feedback: f32,
        damp: f32,
        connected: u8,
    },
    Allpass {
        time: TimeValue,
        feedback: f32,
        connected: u8,
    },
    Delay {
        time: TimeValue,
        connected: u8,
    },
    Sample {
        file_idx: usize,
        file_name: String,
        #[serde(skip)]
        samples: std::sync::Arc<Vec<f32>>,
        connected: u8,
    },
    Probe {
        connected: u8,
    },
    Output {
        gain: f32,
        connected: u8,
    },
    CompositionInput {
        label: String,
        kind: brainwash::patch::InputKind,
        value: f32,
        connected: bool,
    },
    CompositionOutput {
        label: String,
    },
    Composition {
        inputs: u8,
        outputs: u8,
        color: (u8, u8, u8),
    },
    DelayTap {
        gain: f32,
    },
}

fn default_bpm() -> f32 {
    120.0
}

fn default_bars() -> f32 {
    1.0
}

fn is_right(orientation: &Orientation) -> bool {
    *orientation == Orientation::Right
}

fn project_params_match(kind: ModuleKind, params: &ModuleParams) -> bool {
    matches!(
        (kind, params),
        (ModuleKind::Routing(_), ModuleParams::None)
            | (
                ModuleKind::Standard(StandardModule::Primitive),
                ModuleParams::Primitive { .. }
            )
            | (
                ModuleKind::Composition(CompositionModule::Input),
                ModuleParams::CompositionInput { .. }
            )
            | (
                ModuleKind::Composition(CompositionModule::Output),
                ModuleParams::CompositionOutput { .. }
            )
            | (
                ModuleKind::Composition(CompositionModule::Composition(_)),
                ModuleParams::Composition { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Freq),
                ModuleParams::None
            )
            | (
                ModuleKind::Standard(StandardModule::Gate),
                ModuleParams::None
            )
            | (
                ModuleKind::Standard(StandardModule::Degree),
                ModuleParams::None
            )
            | (
                ModuleKind::Standard(StandardModule::DegreeGate),
                ModuleParams::DegreeGate { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Rate),
                ModuleParams::Rate { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Phase),
                ModuleParams::Phase { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Noise),
                ModuleParams::None
            )
            | (
                ModuleKind::Standard(StandardModule::Rise),
                ModuleParams::Rise { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Fall),
                ModuleParams::Fall { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Ramp),
                ModuleParams::Ramp { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Envelope),
                ModuleParams::Envelope { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Comb),
                ModuleParams::Comb { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Allpass),
                ModuleParams::Allpass { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Delay),
                ModuleParams::Delay { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::DelayTap(_)),
                ModuleParams::DelayTap { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Rng),
                ModuleParams::None
            )
            | (
                ModuleKind::Standard(StandardModule::Sample),
                ModuleParams::Sample { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Probe),
                ModuleParams::Probe { .. }
            )
            | (
                ModuleKind::Standard(StandardModule::Output),
                ModuleParams::Output { .. }
            )
    )
}

fn validate_project_params(params: &ModuleParams) -> Result<(), String> {
    match params {
        ModuleParams::Rate { time } => validate_project_time(*time),
        ModuleParams::Phase { frequency, .. } => validate_finite(*frequency),
        ModuleParams::Rise { time, .. }
        | ModuleParams::Fall { time, .. }
        | ModuleParams::Delay { time, .. } => validate_project_time(*time),
        ModuleParams::Ramp { value, time, .. } => {
            validate_finite(*value)?;
            validate_project_time(*time)
        }
        ModuleParams::Envelope { points, .. } => {
            for point in points {
                validate_finite(point.time)?;
                validate_finite(point.value)?;
            }
            Ok(())
        }
        ModuleParams::Comb {
            time,
            feedback,
            damp,
            ..
        } => {
            validate_project_time(*time)?;
            validate_finite(*feedback)?;
            validate_finite(*damp)
        }
        ModuleParams::Allpass { time, feedback, .. } => {
            validate_project_time(*time)?;
            validate_finite(*feedback)
        }
        ModuleParams::Output { gain, .. } | ModuleParams::DelayTap { gain } => {
            validate_finite(*gain)
        }
        ModuleParams::None
        | ModuleParams::Primitive { .. }
        | ModuleParams::DegreeGate { .. }
        | ModuleParams::Sample { .. }
        | ModuleParams::Probe { .. }
        | ModuleParams::Composition { .. } => Ok(()),
        ModuleParams::CompositionInput { label, value, .. } => {
            validate_finite(*value)?;
            if label.trim().is_empty() {
                Err("empty composition port label".to_string())
            } else {
                Ok(())
            }
        }
        ModuleParams::CompositionOutput { label } => {
            if label.trim().is_empty() {
                Err("empty composition port label".to_string())
            } else {
                Ok(())
            }
        }
    }
}

fn validate_project_time(value: TimeValue) -> Result<(), String> {
    validate_finite(value.seconds)?;
    validate_finite(value.samples)?;
    validate_finite(value.hz)?;
    if value.bar_denom == 0 {
        Err("invalid bar denominator".to_string())
    } else {
        Ok(())
    }
}

fn validate_finite(value: f32) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("non-finite project value".to_string())
    }
}

fn validate_module(module: &ModuleDef) -> Result<(), String> {
    if !project_params_match(module.kind, &module.params) {
        return Err(format!(
            "module {} has mismatched params",
            module.id.value()
        ));
    }
    validate_project_params(&module.params)
}

fn validate_project(project: &Project) -> Result<(), String> {
    validate_finite(project.bpm)?;
    validate_finite(project.bars)?;
    for module in &project.modules {
        validate_module(module)?;
    }
    for composition in &project.compositions {
        for module in &composition.modules {
            validate_module(module)?;
        }
    }
    Ok(())
}

fn from_str(input: &str) -> io::Result<Project> {
    let project =
        ron::from_str(input).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    validate_project(&project)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(project)
}

pub fn load(path: &Path) -> io::Result<Project> {
    let input = fs::read_to_string(path)?;
    from_str(&input)
}

pub fn save(path: &Path, project: &Project) -> io::Result<()> {
    validate_project(project).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let output = ron::to_string(project)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_rejects_module_kind_and_params_disagreement() {
        let error = from_str(
            r#"(
                modules: [(
                    id: 1,
                    kind: Standard(Output),
                    x: 0,
                    y: 0,
                    params: None,
                )],
            )"#,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn project_rejects_invalid_inactive_time_value() {
        let error = from_str(
            r#"(
                modules: [(
                    id: 1,
                    kind: Standard(Rise),
                    x: 0,
                    y: 0,
                    params: Rise(
                        time: (
                            unit: Seconds,
                            seconds: 0.1,
                            samples: 100.0,
                            bar_num: 1,
                            bar_denom: 0,
                            hz: 10.0,
                        ),
                        connected: 0,
                    ),
                )],
            )"#,
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
