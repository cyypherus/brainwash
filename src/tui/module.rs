use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubPatchId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn rotate(self) -> Self {
        match self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
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

impl WaveType {
    pub fn name(&self) -> &'static str {
        match self {
            WaveType::Sin => "sin",
            WaveType::Squ => "square",
            WaveType::Tri => "tri",
            WaveType::Saw => "saw",
            WaveType::RSaw => "rsaw",
            WaveType::Noise => "noise",
        }
    }

    pub fn next(self) -> Self {
        match self {
            WaveType::Sin => WaveType::Squ,
            WaveType::Squ => WaveType::Tri,
            WaveType::Tri => WaveType::Saw,
            WaveType::Saw => WaveType::RSaw,
            WaveType::RSaw => WaveType::Noise,
            WaveType::Noise => WaveType::Sin,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            WaveType::Sin => WaveType::Noise,
            WaveType::Squ => WaveType::Sin,
            WaveType::Tri => WaveType::Squ,
            WaveType::Saw => WaveType::Tri,
            WaveType::RSaw => WaveType::Saw,
            WaveType::Noise => WaveType::RSaw,
        }
    }

    pub fn to_index(self) -> u8 {
        match self {
            WaveType::Sin => 0,
            WaveType::Squ => 1,
            WaveType::Tri => 2,
            WaveType::Saw => 3,
            WaveType::RSaw => 4,
            WaveType::Noise => 5,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx {
            0 => WaveType::Sin,
            1 => WaveType::Squ,
            2 => WaveType::Tri,
            3 => WaveType::Saw,
            4 => WaveType::RSaw,
            _ => WaveType::Noise,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    Freq,
    Gate,
    Osc,
    Rise,
    Fall,
    Ramp,
    Adsr,
    Envelope,
    Lpf,
    Hpf,
    Delay,
    Reverb,
    Distortion,
    Flanger,
    Mul,
    Add,
    Gain,
    Gt,
    Lt,
    Switch,
    Probe,
    Output,
    LSplit,
    TSplit,
    RJoin,
    DJoin,
    TurnRD,
    TurnDR,
    SubIn,
    SubOut,
    SubPatch(SubPatchId),
}

impl ModuleKind {
    pub fn name(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "Freq",
            ModuleKind::Gate => "Gate",
            ModuleKind::Osc => "Osc",
            ModuleKind::Rise => "Rise",
            ModuleKind::Fall => "Fall",
            ModuleKind::Ramp => "Ramp",
            ModuleKind::Adsr => "ADSR",
            ModuleKind::Envelope => "Env",
            ModuleKind::Lpf => "LPF",
            ModuleKind::Hpf => "HPF",
            ModuleKind::Delay => "Delay",
            ModuleKind::Reverb => "Verb",
            ModuleKind::Distortion => "Dist",
            ModuleKind::Flanger => "Flang",
            ModuleKind::Mul => "Mul",
            ModuleKind::Add => "Add",
            ModuleKind::Gain => "Gain",
            ModuleKind::Gt => "Gt",
            ModuleKind::Lt => "Lt",
            ModuleKind::Switch => "Switch",
            ModuleKind::Probe => "Probe",
            ModuleKind::Output => "Out",
            ModuleKind::LSplit => "LSplit ◁",
            ModuleKind::TSplit => "USplit △",
            ModuleKind::RJoin => "RJoin ▶",
            ModuleKind::DJoin => "DJoin ▼",
            ModuleKind::TurnRD => "Turn ┐",
            ModuleKind::TurnDR => "Turn └",
            ModuleKind::SubIn => "SubIn",
            ModuleKind::SubOut => "SubOut",
            ModuleKind::SubPatch(_) => "Sub",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "FRQ",
            ModuleKind::Gate => "GAT",
            ModuleKind::Osc => "OSC",
            ModuleKind::Rise => "RIS",
            ModuleKind::Fall => "FAL",
            ModuleKind::Ramp => "RMP",
            ModuleKind::Adsr => "ADS",
            ModuleKind::Envelope => "ENV",
            ModuleKind::Lpf => "LPF",
            ModuleKind::Hpf => "HPF",
            ModuleKind::Delay => "DLY",
            ModuleKind::Reverb => "VRB",
            ModuleKind::Distortion => "DST",
            ModuleKind::Flanger => "FLG",
            ModuleKind::Mul => "MUL",
            ModuleKind::Add => "ADD",
            ModuleKind::Gain => "GAN",
            ModuleKind::Gt => " > ",
            ModuleKind::Lt => " < ",
            ModuleKind::Switch => "SWT",
            ModuleKind::Probe => "PRB",
            ModuleKind::Output => "OUT",
            ModuleKind::LSplit => " ◁ ",
            ModuleKind::TSplit => " △ ",
            ModuleKind::RJoin => " ▶ ",
            ModuleKind::DJoin => " ▼ ",
            ModuleKind::TurnRD => " ┐ ",
            ModuleKind::TurnDR => " └ ",
            ModuleKind::SubIn => "SIN",
            ModuleKind::SubOut => "SOT",
            ModuleKind::SubPatch(_) => "SUB",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "Note frequency from track",
            ModuleKind::Gate => "Note gate - on / off",
            ModuleKind::Osc => "Oscillator - makes noise!",
            ModuleKind::Rise => "Ramps 0->1 while gate high",
            ModuleKind::Fall => "Ramps 0->1 while gate low",
            ModuleKind::Ramp => "Smoothly ramps to target value",
            ModuleKind::Adsr => "Attack/decay/sustain/release",
            ModuleKind::Envelope => "Custom envelope from points",
            ModuleKind::Lpf => "Low-pass filter",
            ModuleKind::Hpf => "High-pass filter",
            ModuleKind::Delay => "Sample delay line",
            ModuleKind::Reverb => "Freeverb reverb effect",
            ModuleKind::Distortion => "Soft-clip distortion",
            ModuleKind::Flanger => "Flanger/chorus effect",
            ModuleKind::Mul => "Multiply A * B",
            ModuleKind::Add => "Add A + B",
            ModuleKind::Gain => "Scale input by gain",
            ModuleKind::Gt => "1 if A > B, else 0",
            ModuleKind::Lt => "1 if A < B, else 0",
            ModuleKind::Switch => "Output A if Sel<=0.5, else B",
            ModuleKind::Probe => "Display signal value",
            ModuleKind::Output => "Final audio output",
            ModuleKind::LSplit => "In from left, out down+right",
            ModuleKind::TSplit => "In from top, out down+right",
            ModuleKind::RJoin => "In from left+top, out right",
            ModuleKind::DJoin => "In from left+top, out down",
            ModuleKind::TurnRD => "In from left, out down",
            ModuleKind::TurnDR => "In from top, out right",
            ModuleKind::SubIn => "Subpatch input port",
            ModuleKind::SubOut => "Subpatch output port",
            ModuleKind::SubPatch(_) => "Subpatch instance",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ModuleKind::Freq | ModuleKind::Gate => Color::Rgb(100, 200, 100),
            ModuleKind::Osc => Color::Rgb(100, 150, 255),
            ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope => Color::Rgb(255, 200, 100),
            ModuleKind::Lpf
            | ModuleKind::Hpf
            | ModuleKind::Delay
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Flanger => Color::Rgb(200, 100, 255),
            ModuleKind::Mul
            | ModuleKind::Add
            | ModuleKind::Gain
            | ModuleKind::Gt
            | ModuleKind::Lt
            | ModuleKind::Switch
            | ModuleKind::Probe => Color::Rgb(100, 220, 220),
            ModuleKind::Output => Color::Rgb(255, 100, 100),
            ModuleKind::LSplit
            | ModuleKind::TSplit
            | ModuleKind::RJoin
            | ModuleKind::DJoin
            | ModuleKind::TurnRD
            | ModuleKind::TurnDR => Color::Rgb(180, 180, 180),
            ModuleKind::SubIn | ModuleKind::SubOut => Color::Rgb(255, 180, 100),
            ModuleKind::SubPatch(_) => Color::Rgb(255, 150, 50),
        }
    }

    pub fn port_count(&self) -> usize {
        match self {
            ModuleKind::LSplit
            | ModuleKind::TSplit
            | ModuleKind::TurnRD
            | ModuleKind::TurnDR => 1,
            ModuleKind::RJoin | ModuleKind::DJoin => 2,
            ModuleKind::SubIn => 0,
            ModuleKind::SubOut => 1,
            _ => self
                .param_defs()
                .iter()
                .filter(|d| d.kind.is_port())
                .count(),
        }
    }

    pub fn port_to_param_idx(&self, port_idx: usize) -> Option<usize> {
        self.param_defs()
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind.is_port())
            .nth(port_idx)
            .map(|(i, _)| i)
    }

    pub fn output_count(&self) -> usize {
        match self {
            ModuleKind::Output | ModuleKind::SubOut => 0,
            ModuleKind::LSplit | ModuleKind::TSplit => 2,
            _ => 1,
        }
    }

    pub fn is_routing(&self) -> bool {
        matches!(
            self,
            ModuleKind::LSplit
                | ModuleKind::TSplit
                | ModuleKind::RJoin
                | ModuleKind::DJoin
                | ModuleKind::TurnRD
                | ModuleKind::TurnDR
        )
    }

    pub fn category(&self) -> ModuleCategory {
        match self {
            ModuleKind::Freq | ModuleKind::Gate => ModuleCategory::Track,
            ModuleKind::Osc => ModuleCategory::Generator,
            ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope => ModuleCategory::Envelope,
            ModuleKind::Lpf
            | ModuleKind::Hpf
            | ModuleKind::Delay
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Flanger => ModuleCategory::Effect,
            ModuleKind::Mul
            | ModuleKind::Add
            | ModuleKind::Gain
            | ModuleKind::Gt
            | ModuleKind::Lt
            | ModuleKind::Switch
            | ModuleKind::Probe => ModuleCategory::Math,
            ModuleKind::Output => ModuleCategory::Output,
            ModuleKind::LSplit
            | ModuleKind::TSplit
            | ModuleKind::RJoin
            | ModuleKind::DJoin
            | ModuleKind::TurnRD
            | ModuleKind::TurnDR => ModuleCategory::Routing,
            ModuleKind::SubIn | ModuleKind::SubOut | ModuleKind::SubPatch(_) => {
                ModuleCategory::Subpatch
            }
        }
    }

    pub fn all() -> &'static [ModuleKind] {
        &[
            ModuleKind::Freq,
            ModuleKind::Gate,
            ModuleKind::Osc,
            ModuleKind::Rise,
            ModuleKind::Fall,
            ModuleKind::Ramp,
            ModuleKind::Adsr,
            ModuleKind::Envelope,
            ModuleKind::Lpf,
            ModuleKind::Hpf,
            ModuleKind::Delay,
            ModuleKind::Reverb,
            ModuleKind::Distortion,
            ModuleKind::Flanger,
            ModuleKind::Mul,
            ModuleKind::Add,
            ModuleKind::Gain,
            ModuleKind::Gt,
            ModuleKind::Lt,
            ModuleKind::Switch,
            ModuleKind::Probe,
            ModuleKind::Output,
            ModuleKind::TurnRD,
            ModuleKind::TurnDR,
            ModuleKind::LSplit,
            ModuleKind::TSplit,
            ModuleKind::RJoin,
            ModuleKind::DJoin,
            ModuleKind::SubIn,
            ModuleKind::SubOut,
            ModuleKind::SubPatch(SubPatchId(0)),
        ]
    }

    pub fn by_category(cat: ModuleCategory) -> Vec<ModuleKind> {
        Self::all()
            .iter()
            .filter(|k| k.category() == cat)
            .copied()
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleCategory {
    Track,
    Generator,
    Envelope,
    Effect,
    Math,
    Routing,
    Subpatch,
    Output,
}

impl ModuleCategory {
    pub fn name(&self) -> &'static str {
        match self {
            ModuleCategory::Track => "Track",
            ModuleCategory::Generator => "Generators",
            ModuleCategory::Envelope => "Envelopes",
            ModuleCategory::Effect => "Effects",
            ModuleCategory::Math => "Math",
            ModuleCategory::Routing => "Routing",
            ModuleCategory::Subpatch => "Subpatch",
            ModuleCategory::Output => "Output",
        }
    }

    pub fn all() -> &'static [ModuleCategory] {
        &[
            ModuleCategory::Track,
            ModuleCategory::Generator,
            ModuleCategory::Envelope,
            ModuleCategory::Effect,
            ModuleCategory::Math,
            ModuleCategory::Routing,
            ModuleCategory::Subpatch,
            ModuleCategory::Output,
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub id: ModuleId,
    pub kind: ModuleKind,
    pub orientation: Orientation,
    pub params: ModuleParams,
}

#[derive(Clone, Copy, Debug)]
pub struct PortInfo {
    pub label: char,
    pub connected: bool,
}

#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub width: u8,
    pub height: u8,
    pub name: &'static str,
    pub color: Color,
    pub input_edge: Edge,
    pub output_edge: Edge,
    pub input_ports: Vec<PortInfo>,
    pub output_ports: Vec<PortInfo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Left,
    Bottom,
    Right,
    None,
}

impl Module {
    pub fn new(id: ModuleId, kind: ModuleKind) -> Self {
        Self {
            id,
            kind,
            orientation: Orientation::default(),
            params: ModuleParams::default_for(kind),
        }
    }

    pub fn rotate(&mut self) {
        self.orientation = self.orientation.rotate();
    }

    pub fn render_info(&self) -> RenderInfo {
        let input_count = self.input_port_count() as usize;
        let output_count = self.output_port_count() as usize;

        let (input_edge, output_edge) = if self.kind.is_routing() {
            match self.kind {
                ModuleKind::LSplit | ModuleKind::TurnRD => (Edge::Left, Edge::Bottom),
                ModuleKind::TSplit => (Edge::Top, Edge::Bottom),
                ModuleKind::RJoin => (Edge::Left, Edge::Right),
                ModuleKind::DJoin => (Edge::Top, Edge::Bottom),
                ModuleKind::TurnDR => (Edge::Top, Edge::Right),
                _ => (Edge::None, Edge::None),
            }
        } else {
            match self.orientation {
                Orientation::Horizontal => (Edge::Left, Edge::Right),
                Orientation::Vertical => (Edge::Top, Edge::Bottom),
            }
        };

        let input_edge = if input_count == 0 { Edge::None } else { input_edge };
        let output_edge = if output_count == 0 { Edge::None } else { output_edge };

        let defs = self.kind.param_defs();
        let port_params: Vec<_> = defs.iter().enumerate().filter(|(_, d)| d.kind.is_port()).collect();

        let input_ports: Vec<PortInfo> = (0..input_count)
            .map(|i| {
                if let Some(&(param_idx, def)) = port_params.get(i) {
                    let connected = match def.kind {
                        ParamKind::Input => true,
                        ParamKind::Float { .. } => self.params.is_connected(param_idx),
                        _ => true,
                    };
                    PortInfo {
                        label: def.name.chars().next().unwrap_or(' '),
                        connected,
                    }
                } else {
                    PortInfo { label: ' ', connected: true }
                }
            })
            .collect();

        let output_ports: Vec<PortInfo> = (0..output_count)
            .map(|_| PortInfo { label: ' ', connected: true })
            .collect();

        RenderInfo {
            width: self.width(),
            height: self.height(),
            name: self.display_name(),
            color: self.kind.color(),
            input_edge,
            output_edge,
            input_ports,
            output_ports,
        }
    }

    pub fn input_port_count(&self) -> u8 {
        if let ModuleParams::SubPatch { inputs, .. } = self.params {
            return inputs.max(1);
        }
        self.kind.port_count() as u8
    }

    pub fn output_port_count(&self) -> u8 {
        if let ModuleParams::SubPatch { outputs, .. } = self.params {
            return outputs.max(1);
        }
        self.kind.output_count() as u8
    }

    pub fn width(&self) -> u8 {
        if self.kind.is_routing() {
            return 1;
        }
        match self.orientation {
            Orientation::Horizontal => self.output_port_count().max(1),
            Orientation::Vertical => self.input_port_count().max(1),
        }
    }

    pub fn height(&self) -> u8 {
        if self.kind.is_routing() {
            return 1;
        }
        match self.orientation {
            Orientation::Horizontal => self.input_port_count().max(1),
            Orientation::Vertical => self.output_port_count().max(1),
        }
    }

    pub fn has_input_top(&self) -> bool {
        if self.input_port_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TurnRD | ModuleKind::SubIn => false,
            ModuleKind::TSplit | ModuleKind::TurnDR => true,
            ModuleKind::RJoin | ModuleKind::DJoin => true,
            _ => self.orientation == Orientation::Vertical,
        }
    }

    pub fn has_input_left(&self) -> bool {
        if self.input_port_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TurnRD => true,
            ModuleKind::TSplit | ModuleKind::TurnDR | ModuleKind::SubIn => false,
            ModuleKind::RJoin | ModuleKind::DJoin => true,
            _ => self.orientation == Orientation::Horizontal,
        }
    }

    pub fn has_output_bottom(&self) -> bool {
        if self.output_port_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::TurnRD => true,
            ModuleKind::RJoin | ModuleKind::TurnDR | ModuleKind::SubOut => false,
            ModuleKind::DJoin => true,
            _ => self.orientation == Orientation::Vertical,
        }
    }

    pub fn has_output_right(&self) -> bool {
        if self.output_port_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::TurnDR => true,
            ModuleKind::RJoin => true,
            ModuleKind::DJoin | ModuleKind::TurnRD | ModuleKind::SubOut => false,
            _ => self.orientation == Orientation::Horizontal,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match &self.params {
            ModuleParams::Osc { wave, .. } => match wave {
                WaveType::Sin => "SIN",
                WaveType::Squ => "SQU",
                WaveType::Tri => "TRI",
                WaveType::Saw => "SAW",
                WaveType::RSaw => "RSW",
                WaveType::Noise => "NSE",
            },
            _ => self.kind.short_name(),
        }
    }

    pub fn is_port_open(&self, port_idx: usize) -> bool {
        if self.kind.is_routing() {
            return port_idx < self.kind.port_count();
        }

        if let ModuleParams::SubPatch { inputs, outputs } = self.params {
            return port_idx < inputs.max(outputs) as usize;
        }

        let Some(param_idx) = self.kind.port_to_param_idx(port_idx) else {
            return false;
        };

        let defs = self.kind.param_defs();
        if let Some(def) = defs.get(param_idx) {
            match def.kind {
                ParamKind::Input => true,
                ParamKind::Float { .. } => self.params.is_connected(param_idx),
                ParamKind::Enum | ParamKind::Toggle => false,
            }
        } else {
            false
        }
    }
}

pub enum ParamKind {
    Float { min: f32, max: f32, step: f32 },
    Input,
    Enum,
    Toggle,
}

impl ParamKind {
    pub fn is_port(&self) -> bool {
        matches!(self, ParamKind::Input | ParamKind::Float { .. })
    }
}

pub struct ParamDef {
    pub name: &'static str,
    pub kind: ParamKind,
}

impl ModuleKind {
    pub fn param_defs(&self) -> &'static [ParamDef] {
        match self {
            ModuleKind::Freq | ModuleKind::Gate => &[],
            ModuleKind::Osc => &[
                ParamDef {
                    name: "Wave",
                    kind: ParamKind::Enum,
                },
                ParamDef {
                    name: "Freq",
                    kind: ParamKind::Float {
                        min: 20.0,
                        max: 20000.0,
                        step: 1.0,
                    },
                },
                ParamDef {
                    name: "Shift",
                    kind: ParamKind::Float {
                        min: -24.0,
                        max: 24.0,
                        step: 1.0,
                    },
                },
                ParamDef {
                    name: "Gain",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
                ParamDef {
                    name: "Uni",
                    kind: ParamKind::Toggle,
                },
            ],
            ModuleKind::Rise | ModuleKind::Fall => &[
                ParamDef {
                    name: "Gate",
                    kind: ParamKind::Input,
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Float {
                        min: 0.001,
                        max: 10.0,
                        step: 0.01,
                    },
                },
            ],
            ModuleKind::Ramp => &[
                ParamDef {
                    name: "Val",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Float {
                        min: 0.001,
                        max: 10.0,
                        step: 0.01,
                    },
                },
            ],
            ModuleKind::Adsr => &[
                ParamDef {
                    name: "Rise",
                    kind: ParamKind::Input,
                },
                ParamDef {
                    name: "Fall",
                    kind: ParamKind::Input,
                },
                ParamDef {
                    name: "Atk",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
                ParamDef {
                    name: "Sus",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
            ],
            ModuleKind::Envelope => &[ParamDef {
                name: "Phase",
                kind: ParamKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                },
            }],
            ModuleKind::Lpf => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Freq",
                    kind: ParamKind::Float {
                        min: 0.001,
                        max: 0.99,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Q",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                },
            ],
            ModuleKind::Hpf => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Freq",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Q",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                },
            ],
            ModuleKind::Delay => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Samp",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 44100.0,
                        step: 100.0,
                    },
                },
            ],
            ModuleKind::Reverb => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Room",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
                ParamDef {
                    name: "Damp",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
            ],
            ModuleKind::Distortion => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Drive",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 0.5,
                        step: 0.05,
                    },
                },
                ParamDef {
                    name: "Gain",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
            ],
            ModuleKind::Flanger => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Rate",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                },
                ParamDef {
                    name: "Depth",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                },
                ParamDef {
                    name: "Fdbk",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 0.95,
                        step: 0.05,
                    },
                },
            ],
            ModuleKind::Mul => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 100.0,
                        step: 0.1,
                    },
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 100.0,
                        step: 0.1,
                    },
                },
            ],
            ModuleKind::Add => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 1.0,
                    },
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 1.0,
                    },
                },
            ],
            ModuleKind::Gain => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                },
                ParamDef {
                    name: "Gain",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 2.0,
                        step: 0.05,
                    },
                },
            ],
            ModuleKind::Gt => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
            ],
            ModuleKind::Lt => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
            ],
            ModuleKind::Switch => &[
                ParamDef {
                    name: "Sel",
                    kind: ParamKind::Input,
                },
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                },
            ],
            ModuleKind::Probe => &[ParamDef {
                name: "In",
                kind: ParamKind::Float {
                    min: -1.0,
                    max: 1.0,
                    step: 0.01,
                },
            }],
            ModuleKind::Output => &[ParamDef {
                name: "In",
                kind: ParamKind::Float {
                    min: -1.0,
                    max: 1.0,
                    step: 0.01,
                },
            }],
            ModuleKind::SubIn => &[],
            ModuleKind::SubOut => &[ParamDef {
                name: "In",
                kind: ParamKind::Input,
            }],
            _ => &[],
        }
    }
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
    Osc {
        wave: WaveType,
        freq: f32,
        shift: f32,
        gain: f32,
        uni: bool,
        connected: u8,
    },
    Rise {
        time: f32,
        connected: u8,
    },
    Fall {
        time: f32,
        connected: u8,
    },
    Ramp {
        value: f32,
        time: f32,
        connected: u8,
    },
    Adsr {
        attack_ratio: f32,
        sustain: f32,
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
    Delay {
        samples: f32,
        connected: u8,
    },
    Reverb {
        room: f32,
        damp: f32,
        connected: u8,
    },
    Distortion {
        drive: f32,
        gain: f32,
        connected: u8,
    },
    Flanger {
        rate: f32,
        depth: f32,
        feedback: f32,
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
    Gain {
        gain: f32,
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
    Probe {
        connected: u8,
    },
    Output {
        connected: u8,
    },
    SubPatch {
        inputs: u8,
        outputs: u8,
    },
}

impl ModuleParams {
    pub fn default_for(kind: ModuleKind) -> Self {
        match kind {
            ModuleKind::Freq | ModuleKind::Gate => ModuleParams::None,
            ModuleKind::Osc => ModuleParams::Osc {
                wave: WaveType::Sin,
                freq: 440.0,
                shift: 0.0,
                gain: 1.0,
                uni: false,
                connected: 0xFF,
            },
            ModuleKind::Rise => ModuleParams::Rise {
                time: 0.1,
                connected: 0xFF,
            },
            ModuleKind::Fall => ModuleParams::Fall {
                time: 0.1,
                connected: 0xFF,
            },
            ModuleKind::Ramp => ModuleParams::Ramp {
                value: 0.0,
                time: 0.1,
                connected: 0xFF,
            },
            ModuleKind::Adsr => ModuleParams::Adsr {
                attack_ratio: 0.5,
                sustain: 0.7,
                connected: 0xFF,
            },
            ModuleKind::Envelope => ModuleParams::Envelope {
                points: vec![
                    EnvPoint {
                        time: 0.0,
                        value: 0.0,
                        curve: false,
                    },
                    EnvPoint {
                        time: 1.0,
                        value: 1.0,
                        curve: false,
                    },
                ],
                connected: 0xFF,
            },
            ModuleKind::Lpf => ModuleParams::Filter {
                freq: 0.5,
                q: 0.707,
                connected: 0xFF,
            },
            ModuleKind::Hpf => ModuleParams::Filter {
                freq: 0.5,
                q: 0.707,
                connected: 0xFF,
            },
            ModuleKind::Delay => ModuleParams::Delay {
                samples: 0.0,
                connected: 0xFF,
            },
            ModuleKind::Reverb => ModuleParams::Reverb {
                room: 0.5,
                damp: 0.5,
                connected: 0xFF,
            },
            ModuleKind::Distortion => ModuleParams::Distortion {
                drive: 0.3,
                gain: 1.0,
                connected: 0xFF,
            },
            ModuleKind::Flanger => ModuleParams::Flanger {
                rate: 0.5,
                depth: 0.5,
                feedback: 0.3,
                connected: 0xFF,
            },
            ModuleKind::Mul => ModuleParams::Mul {
                a: 1.0,
                b: 1.0,
                connected: 0xFF,
            },
            ModuleKind::Add => ModuleParams::Add {
                a: 0.0,
                b: 0.0,
                connected: 0xFF,
            },
            ModuleKind::Gain => ModuleParams::Gain {
                gain: 1.0,
                connected: 0xFF,
            },
            ModuleKind::Gt => ModuleParams::Gt {
                a: 0.0,
                b: 0.0,
                connected: 0xFF,
            },
            ModuleKind::Lt => ModuleParams::Lt {
                a: 0.0,
                b: 0.0,
                connected: 0xFF,
            },
            ModuleKind::Switch => ModuleParams::Switch {
                a: 0.0,
                b: 0.0,
                connected: 0xFF,
            },
            ModuleKind::Probe => ModuleParams::Probe { connected: 0xFF },
            ModuleKind::Output => ModuleParams::Output { connected: 0xFF },
            ModuleKind::LSplit
            | ModuleKind::TSplit
            | ModuleKind::RJoin
            | ModuleKind::DJoin
            | ModuleKind::TurnRD
            | ModuleKind::TurnDR
            | ModuleKind::SubIn
            | ModuleKind::SubOut => ModuleParams::None,
            ModuleKind::SubPatch(_) => ModuleParams::SubPatch {
                inputs: 1,
                outputs: 1,
            },
        }
    }

    pub fn connected(&self) -> u8 {
        match self {
            ModuleParams::None => 0xFF,
            ModuleParams::Osc { connected, .. } => *connected,
            ModuleParams::Rise { connected, .. } => *connected,
            ModuleParams::Fall { connected, .. } => *connected,
            ModuleParams::Ramp { connected, .. } => *connected,
            ModuleParams::Adsr { connected, .. } => *connected,
            ModuleParams::Envelope { connected, .. } => *connected,
            ModuleParams::Filter { connected, .. } => *connected,
            ModuleParams::Delay { connected, .. } => *connected,
            ModuleParams::Reverb { connected, .. } => *connected,
            ModuleParams::Distortion { connected, .. } => *connected,
            ModuleParams::Flanger { connected, .. } => *connected,
            ModuleParams::Mul { connected, .. } => *connected,
            ModuleParams::Add { connected, .. } => *connected,
            ModuleParams::Gain { connected, .. } => *connected,
            ModuleParams::Gt { connected, .. } => *connected,
            ModuleParams::Lt { connected, .. } => *connected,
            ModuleParams::Switch { connected, .. } => *connected,
            ModuleParams::Probe { connected, .. } => *connected,
            ModuleParams::Output { connected, .. } => *connected,
            ModuleParams::SubPatch { .. } => 0xFF,
        }
    }

    pub fn connected_mut(&mut self) -> Option<&mut u8> {
        match self {
            ModuleParams::None | ModuleParams::SubPatch { .. } => None,
            ModuleParams::Osc { connected, .. } => Some(connected),
            ModuleParams::Rise { connected, .. } => Some(connected),
            ModuleParams::Fall { connected, .. } => Some(connected),
            ModuleParams::Ramp { connected, .. } => Some(connected),
            ModuleParams::Adsr { connected, .. } => Some(connected),
            ModuleParams::Envelope { connected, .. } => Some(connected),
            ModuleParams::Filter { connected, .. } => Some(connected),
            ModuleParams::Delay { connected, .. } => Some(connected),
            ModuleParams::Reverb { connected, .. } => Some(connected),
            ModuleParams::Distortion { connected, .. } => Some(connected),
            ModuleParams::Flanger { connected, .. } => Some(connected),
            ModuleParams::Mul { connected, .. } => Some(connected),
            ModuleParams::Add { connected, .. } => Some(connected),
            ModuleParams::Gain { connected, .. } => Some(connected),
            ModuleParams::Gt { connected, .. } => Some(connected),
            ModuleParams::Lt { connected, .. } => Some(connected),
            ModuleParams::Switch { connected, .. } => Some(connected),
            ModuleParams::Probe { connected, .. } => Some(connected),
            ModuleParams::Output { connected, .. } => Some(connected),
        }
    }

    pub fn is_connected(&self, idx: usize) -> bool {
        (self.connected() & (1 << idx)) != 0
    }

    pub fn set_connected(&mut self, idx: usize, val: bool) {
        if let Some(c) = self.connected_mut() {
            if val {
                *c |= 1 << idx;
            } else {
                *c &= !(1 << idx);
            }
        }
    }

    pub fn toggle_connected(&mut self, idx: usize) {
        if let Some(c) = self.connected_mut() {
            *c ^= 1 << idx;
        }
    }

    pub fn get_float(&self, idx: usize) -> Option<f32> {
        match self {
            ModuleParams::Osc {
                freq, shift, gain, ..
            } => match idx {
                1 => Some(*freq),
                2 => Some(*shift),
                3 => Some(*gain),
                _ => None,
            },
            ModuleParams::Rise { time, .. } | ModuleParams::Fall { time, .. } => match idx {
                1 => Some(*time),
                _ => None,
            },
            ModuleParams::Ramp { value, time, .. } => match idx {
                0 => Some(*value),
                1 => Some(*time),
                _ => None,
            },
            ModuleParams::Adsr {
                attack_ratio,
                sustain,
                ..
            } => match idx {
                2 => Some(*attack_ratio),
                3 => Some(*sustain),
                _ => None,
            },
            ModuleParams::Filter { freq, q, .. } => match idx {
                1 => Some(*freq),
                2 => Some(*q),
                _ => None,
            },
            ModuleParams::Delay { samples, .. } => match idx {
                1 => Some(*samples),
                _ => None,
            },
            ModuleParams::Reverb { room, damp, .. } => match idx {
                1 => Some(*room),
                2 => Some(*damp),
                _ => None,
            },
            ModuleParams::Distortion { drive, gain, .. } => match idx {
                1 => Some(*drive),
                2 => Some(*gain),
                _ => None,
            },
            ModuleParams::Flanger {
                rate,
                depth,
                feedback,
                ..
            } => match idx {
                1 => Some(*rate),
                2 => Some(*depth),
                3 => Some(*feedback),
                _ => None,
            },
            ModuleParams::Mul { a, b, .. } => match idx {
                0 => Some(*a),
                1 => Some(*b),
                _ => None,
            },
            ModuleParams::Add { a, b, .. } => match idx {
                0 => Some(*a),
                1 => Some(*b),
                _ => None,
            },
            ModuleParams::Gain { gain, .. } => match idx {
                1 => Some(*gain),
                _ => None,
            },
            ModuleParams::Gt { a, b, .. } => match idx {
                0 => Some(*a),
                1 => Some(*b),
                _ => None,
            },
            ModuleParams::Lt { a, b, .. } => match idx {
                0 => Some(*a),
                1 => Some(*b),
                _ => None,
            },
            ModuleParams::Switch { a, b, .. } => match idx {
                1 => Some(*a),
                2 => Some(*b),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn set_float(&mut self, idx: usize, val: f32) {
        match self {
            ModuleParams::Osc {
                freq, shift, gain, ..
            } => match idx {
                1 => *freq = val,
                2 => *shift = val,
                3 => *gain = val,
                _ => {}
            },
            ModuleParams::Rise { time, .. } | ModuleParams::Fall { time, .. } => match idx {
                1 => *time = val,
                _ => {}
            },
            ModuleParams::Ramp { value, time, .. } => match idx {
                0 => *value = val,
                1 => *time = val,
                _ => {}
            },
            ModuleParams::Adsr {
                attack_ratio,
                sustain,
                ..
            } => match idx {
                2 => *attack_ratio = val,
                3 => *sustain = val,
                _ => {}
            },
            ModuleParams::Filter { freq, q, .. } => match idx {
                1 => *freq = val,
                2 => *q = val,
                _ => {}
            },
            ModuleParams::Delay { samples, .. } => match idx {
                1 => *samples = val,
                _ => {}
            },
            ModuleParams::Reverb { room, damp, .. } => match idx {
                1 => *room = val,
                2 => *damp = val,
                _ => {}
            },
            ModuleParams::Distortion { drive, gain, .. } => match idx {
                1 => *drive = val,
                2 => *gain = val,
                _ => {}
            },
            ModuleParams::Flanger {
                rate,
                depth,
                feedback,
                ..
            } => match idx {
                1 => *rate = val,
                2 => *depth = val,
                3 => *feedback = val,
                _ => {}
            },
            ModuleParams::Mul { a, b, .. } => match idx {
                0 => *a = val,
                1 => *b = val,
                _ => {}
            },
            ModuleParams::Add { a, b, .. } => match idx {
                0 => *a = val,
                1 => *b = val,
                _ => {}
            },
            ModuleParams::Gain { gain, .. } => match idx {
                1 => *gain = val,
                _ => {}
            },
            ModuleParams::Gt { a, b, .. } => match idx {
                0 => *a = val,
                1 => *b = val,
                _ => {}
            },
            ModuleParams::Lt { a, b, .. } => match idx {
                0 => *a = val,
                1 => *b = val,
                _ => {}
            },
            ModuleParams::Switch { a, b, .. } => match idx {
                1 => *a = val,
                2 => *b = val,
                _ => {}
            },
            _ => {}
        }
    }

    pub fn env_points(&self) -> Option<&Vec<EnvPoint>> {
        match self {
            ModuleParams::Envelope { points, .. } => Some(points),
            _ => None,
        }
    }

    pub fn env_points_mut(&mut self) -> Option<&mut Vec<EnvPoint>> {
        match self {
            ModuleParams::Envelope { points, .. } => Some(points),
            _ => None,
        }
    }

    pub fn cycle_enum_next(&mut self) {
        match self {
            ModuleParams::Osc { wave, .. } => *wave = wave.next(),
            _ => {}
        }
    }

    pub fn cycle_enum_prev(&mut self) {
        match self {
            ModuleParams::Osc { wave, .. } => *wave = wave.prev(),
            _ => {}
        }
    }

    pub fn get_toggle(&self, idx: usize) -> bool {
        match self {
            ModuleParams::Osc { uni, .. } => match idx {
                4 => *uni,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn toggle(&mut self, idx: usize) {
        match self {
            ModuleParams::Osc { uni, .. } => match idx {
                4 => *uni = !*uni,
                _ => {}
            },
            _ => {}
        }
    }

    pub fn has_enum(&self) -> bool {
        matches!(self, ModuleParams::Osc { .. })
    }

    pub fn enum_display(&self) -> Option<&'static str> {
        match self {
            ModuleParams::Osc { wave, .. } => Some(wave.name()),
            _ => None,
        }
    }
}
