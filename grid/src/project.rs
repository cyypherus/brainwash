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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WaveType {
    #[default]
    Sin,
    Squ,
    Tri,
    Saw,
    RSaw,
    Noise,
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
    Osc,
    Rise,
    Fall,
    Ramp,
    Envelope,
    Lpf,
    Hpf,
    Comb,
    Allpass,
    Delay,
    DelayTap(ModuleId),
    Mul,
    Add,
    Gt,
    Lt,
    Switch,
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
    Osc {
        wave: WaveType,
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
    Filter {
        freq: f32,
        q: f32,
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
    Mul {
        a: f32,
        b: f32,
        connected: u8,
    },
    Add {
        a: f32,
        b: f32,
        connected: u8,
    },
    Gt {
        a: f32,
        b: f32,
        connected: u8,
    },
    Lt {
        a: f32,
        b: f32,
        connected: u8,
    },
    Switch {
        a: f32,
        b: f32,
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

pub fn from_str(input: &str) -> Result<Project, ron::error::SpannedError> {
    ron::from_str(input)
}

pub fn load(path: &Path) -> io::Result<Project> {
    let input = fs::read_to_string(path)?;
    from_str(&input).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn save(path: &Path, project: &Project) -> io::Result<()> {
    let output = ron::to_string(project)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, output)
}
