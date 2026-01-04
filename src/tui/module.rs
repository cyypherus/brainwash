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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DistType {
    #[default]
    Tube,
    Tape,
    Fuzz,
    Fold,
    Clip,
}

impl DistType {
    pub fn name(&self) -> &'static str {
        match self {
            DistType::Tube => "tube",
            DistType::Tape => "tape",
            DistType::Fuzz => "fuzz",
            DistType::Fold => "fold",
            DistType::Clip => "clip",
        }
    }

    pub fn next(self) -> Self {
        match self {
            DistType::Tube => DistType::Tape,
            DistType::Tape => DistType::Fuzz,
            DistType::Fuzz => DistType::Fold,
            DistType::Fold => DistType::Clip,
            DistType::Clip => DistType::Tube,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            DistType::Tube => DistType::Clip,
            DistType::Tape => DistType::Tube,
            DistType::Fuzz => DistType::Tape,
            DistType::Fold => DistType::Fuzz,
            DistType::Clip => DistType::Fold,
        }
    }

    pub fn to_index(self) -> u8 {
        match self {
            DistType::Tube => 0,
            DistType::Tape => 1,
            DistType::Fuzz => 2,
            DistType::Fold => 3,
            DistType::Clip => 4,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx {
            0 => DistType::Tube,
            1 => DistType::Tape,
            2 => DistType::Fuzz,
            3 => DistType::Fold,
            _ => DistType::Clip,
        }
    }

    pub fn to_dsp(self) -> crate::distortion::DistortionType {
        match self {
            DistType::Tube => crate::distortion::DistortionType::Tube,
            DistType::Tape => crate::distortion::DistortionType::Tape,
            DistType::Fuzz => crate::distortion::DistortionType::Fuzz,
            DistType::Fold => crate::distortion::DistortionType::Fold,
            DistType::Clip => crate::distortion::DistortionType::Clip,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    Freq,
    Gate,
    Degree,
    DegreeGate,
    Osc,
    Rise,
    Fall,
    Ramp,
    Adsr,
    Envelope,
    Lpf,
    Hpf,
    Comb,
    Allpass,
    Delay,
    Reverb,
    Distortion,
    Flanger,
    Mul,
    Add,
    Gt,
    Lt,
    Switch,
    Rng,
    Sample,
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
    DelayTap(ModuleId),
}

impl ModuleKind {
    pub fn name(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "Freq",
            ModuleKind::Gate => "Gate",
            ModuleKind::Degree => "Deg",
            ModuleKind::DegreeGate => "DegG",
            ModuleKind::Osc => "Osc",
            ModuleKind::Rise => "Rise",
            ModuleKind::Fall => "Fall",
            ModuleKind::Ramp => "Ramp",
            ModuleKind::Adsr => "ADSR",
            ModuleKind::Envelope => "Env",
            ModuleKind::Lpf => "LPF",
            ModuleKind::Hpf => "HPF",
            ModuleKind::Comb => "Comb",
            ModuleKind::Allpass => "Allpass",
            ModuleKind::Delay => "Delay",
            ModuleKind::Reverb => "Verb",
            ModuleKind::Distortion => "Dist",
            ModuleKind::Flanger => "Flang",
            ModuleKind::Mul => "Mul",
            ModuleKind::Add => "Add",
            ModuleKind::Gt => "Gt",
            ModuleKind::Lt => "Lt",
            ModuleKind::Switch => "Switch",
            ModuleKind::Rng => "Rng",
            ModuleKind::Sample => "Sample",
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
            ModuleKind::DelayTap(_) => "Tap",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "FRQ",
            ModuleKind::Gate => "GAT",
            ModuleKind::Degree => "DEG",
            ModuleKind::DegreeGate => "DGG",
            ModuleKind::Osc => "OSC",
            ModuleKind::Rise => "RIS",
            ModuleKind::Fall => "FAL",
            ModuleKind::Ramp => "RMP",
            ModuleKind::Adsr => "ADS",
            ModuleKind::Envelope => "ENV",
            ModuleKind::Lpf => "LPF",
            ModuleKind::Hpf => "HPF",
            ModuleKind::Comb => "CMB",
            ModuleKind::Allpass => "APF",
            ModuleKind::Delay => "DLY",
            ModuleKind::Reverb => "VRB",
            ModuleKind::Distortion => "DST",
            ModuleKind::Flanger => "FLG",
            ModuleKind::Mul => "MUL",
            ModuleKind::Add => "ADD",
            ModuleKind::Gt => " > ",
            ModuleKind::Lt => " < ",
            ModuleKind::Switch => "SWT",
            ModuleKind::Rng => "RNG",
            ModuleKind::Sample => "SMP",
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
            ModuleKind::DelayTap(_) => "TAP",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "Note frequency from track",
            ModuleKind::Gate => "Note gate - on / off",
            ModuleKind::Degree => "Scale degree from track",
            ModuleKind::DegreeGate => "Gate when degree matches",
            ModuleKind::Osc => "Oscillator - makes noise!",
            ModuleKind::Rise => "Ramps 0->1 while gate high",
            ModuleKind::Fall => "Ramps 0->1 while gate low",
            ModuleKind::Ramp => "Smoothly ramps to target value",
            ModuleKind::Adsr => "Attack/decay/sustain/release",
            ModuleKind::Envelope => "Custom envelope from points",
            ModuleKind::Lpf => "Low-pass filter",
            ModuleKind::Hpf => "High-pass filter",
            ModuleKind::Comb => "Comb filter (resonant delay)",
            ModuleKind::Allpass => "Allpass filter (phase shift)",
            ModuleKind::Delay => "Sample delay line",
            ModuleKind::Reverb => "FDN reverb with modulation",
            ModuleKind::Distortion => "Soft-clip distortion",
            ModuleKind::Flanger => "Flanger/chorus effect",
            ModuleKind::Mul => "Multiply A * B",
            ModuleKind::Add => "Add A + B",
            ModuleKind::Gt => "1 if A > B, else 0",
            ModuleKind::Lt => "1 if A < B, else 0",
            ModuleKind::Switch => "Output A if Sel<=0.5, else B",
            ModuleKind::Rng => "Random 0-1 on gate rising edge",
            ModuleKind::Sample => "Play WAV file by position 0-1",
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
            ModuleKind::DelayTap(_) => "Read from delay (feedback)",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree | ModuleKind::DegreeGate => {
                Color::Rgb(100, 200, 100)
            }
            ModuleKind::Osc | ModuleKind::Sample => Color::Rgb(100, 150, 255),
            ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope => Color::Rgb(255, 200, 100),
            ModuleKind::Lpf | ModuleKind::Hpf | ModuleKind::Comb | ModuleKind::Allpass => {
                Color::Rgb(150, 200, 255)
            }
            ModuleKind::Delay
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Flanger => Color::Rgb(200, 100, 255),
            ModuleKind::Mul
            | ModuleKind::Add
            | ModuleKind::Gt
            | ModuleKind::Lt
            | ModuleKind::Switch
            | ModuleKind::Rng
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
            ModuleKind::DelayTap(_) => Color::Rgb(200, 100, 255),
        }
    }

    pub fn port_count(&self) -> usize {
        match self {
            ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::TurnRD | ModuleKind::TurnDR => 1,
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
            ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree | ModuleKind::DegreeGate => {
                ModuleCategory::Track
            }
            ModuleKind::Osc | ModuleKind::Sample => ModuleCategory::Generator,
            ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope => ModuleCategory::Envelope,
            ModuleKind::Lpf | ModuleKind::Hpf | ModuleKind::Comb | ModuleKind::Allpass => {
                ModuleCategory::Filter
            }
            ModuleKind::Delay
            | ModuleKind::DelayTap(_)
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Flanger => ModuleCategory::Effect,
            ModuleKind::Mul
            | ModuleKind::Add
            | ModuleKind::Gt
            | ModuleKind::Lt
            | ModuleKind::Switch
            | ModuleKind::Rng
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
            ModuleKind::Degree,
            ModuleKind::DegreeGate,
            ModuleKind::Osc,
            ModuleKind::Rise,
            ModuleKind::Fall,
            ModuleKind::Ramp,
            ModuleKind::Adsr,
            ModuleKind::Envelope,
            ModuleKind::Lpf,
            ModuleKind::Hpf,
            ModuleKind::Comb,
            ModuleKind::Allpass,
            ModuleKind::Delay,
            ModuleKind::DelayTap(ModuleId(0)),
            ModuleKind::Reverb,
            ModuleKind::Distortion,
            ModuleKind::Flanger,
            ModuleKind::Mul,
            ModuleKind::Add,
            ModuleKind::Gt,
            ModuleKind::Lt,
            ModuleKind::Switch,
            ModuleKind::Rng,
            ModuleKind::Sample,
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
    Filter,
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
            ModuleCategory::Filter => "Filters",
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
            ModuleCategory::Filter,
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
    pub input_edges: Vec<Edge>,
    pub output_edges: Vec<Edge>,
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

        let (input_edges, output_edges): (Vec<Edge>, Vec<Edge>) = if self.kind.is_routing() {
            match self.kind {
                ModuleKind::LSplit => (vec![Edge::Left], vec![Edge::Bottom, Edge::Right]),
                ModuleKind::TSplit => (vec![Edge::Top], vec![Edge::Bottom, Edge::Right]),
                ModuleKind::RJoin => (vec![Edge::Left, Edge::Top], vec![Edge::Right]),
                ModuleKind::DJoin => (vec![Edge::Left, Edge::Top], vec![Edge::Bottom]),
                ModuleKind::TurnRD => (vec![Edge::Left], vec![Edge::Bottom]),
                ModuleKind::TurnDR => (vec![Edge::Top], vec![Edge::Right]),
                _ => (vec![], vec![]),
            }
        } else {
            match self.orientation {
                Orientation::Horizontal => (vec![Edge::Left], vec![Edge::Right]),
                Orientation::Vertical => (vec![Edge::Top], vec![Edge::Bottom]),
            }
        };

        let input_edges = if input_count == 0 {
            vec![]
        } else {
            input_edges
        };
        let output_edges = if output_count == 0 {
            vec![]
        } else {
            output_edges
        };

        let defs = self.kind.param_defs();
        let port_params: Vec<_> = defs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind.is_port())
            .collect();

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
                    PortInfo {
                        label: ' ',
                        connected: true,
                    }
                }
            })
            .collect();

        let output_ports: Vec<PortInfo> = (0..output_count)
            .map(|_| PortInfo {
                label: ' ',
                connected: true,
            })
            .collect();

        RenderInfo {
            width: self.width(),
            height: self.height(),
            name: self.display_name(),
            color: self.kind.color(),
            input_edges,
            output_edges,
            input_ports,
            output_ports,
        }
    }

    pub fn input_port_count(&self) -> u8 {
        if let ModuleParams::SubPatch { inputs, .. } = self.params {
            return inputs;
        }
        self.kind.port_count() as u8
    }

    pub fn output_port_count(&self) -> u8 {
        if let ModuleParams::SubPatch { outputs, .. } = self.params {
            return outputs;
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
                ParamKind::Float { .. } | ParamKind::Time => self.params.is_connected(param_idx),
                ParamKind::Enum | ParamKind::Toggle | ParamKind::Int { .. } => false,
            }
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimeUnit {
    Seconds,
    Samples,
    Bars,
    Hz,
}

impl TimeUnit {
    pub fn next(self) -> Self {
        match self {
            TimeUnit::Seconds => TimeUnit::Samples,
            TimeUnit::Samples => TimeUnit::Bars,
            TimeUnit::Bars => TimeUnit::Hz,
            TimeUnit::Hz => TimeUnit::Seconds,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TimeUnit::Seconds => TimeUnit::Hz,
            TimeUnit::Samples => TimeUnit::Seconds,
            TimeUnit::Bars => TimeUnit::Samples,
            TimeUnit::Hz => TimeUnit::Bars,
        }
    }

    pub fn suffix(&self) -> &'static str {
        match self {
            TimeUnit::Seconds => "s",
            TimeUnit::Samples => "smp",
            TimeUnit::Bars => "bar",
            TimeUnit::Hz => "hz",
        }
    }
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

impl Default for TimeValue {
    fn default() -> Self {
        Self {
            unit: TimeUnit::Seconds,
            seconds: 0.1,
            samples: 4410.0,
            bar_num: 1,
            bar_denom: 4,
            hz: 10.0,
        }
    }
}

impl TimeValue {
    pub fn from_seconds(s: f32) -> Self {
        Self {
            unit: TimeUnit::Seconds,
            seconds: s,
            samples: s * 44100.0,
            bar_num: 1,
            bar_denom: 4,
            hz: 1.0 / s,
        }
    }

    pub fn from_samples(s: f32) -> Self {
        Self {
            unit: TimeUnit::Samples,
            seconds: s / 44100.0,
            samples: s,
            bar_num: 1,
            bar_denom: 4,
            hz: 44100.0 / s,
        }
    }

    pub fn from_hz(h: f32) -> Self {
        Self {
            unit: TimeUnit::Hz,
            seconds: 1.0 / h,
            samples: 44100.0 / h,
            bar_num: 1,
            bar_denom: 4,
            hz: h,
        }
    }

    pub fn to_hz(&self, bpm: f32, bars: f32) -> f32 {
        1.0 / self.to_seconds(bpm, bars)
    }

    pub fn to_samples(&self, sample_rate: f32, bpm: f32, bars: f32) -> f32 {
        match self.unit {
            TimeUnit::Seconds => self.seconds * sample_rate,
            TimeUnit::Samples => self.samples,
            TimeUnit::Bars => {
                let bar_fraction = self.bar_num as f32 / self.bar_denom as f32;
                let seconds_per_bar = (60.0 / bpm) * 4.0 * bars;
                bar_fraction * seconds_per_bar * sample_rate
            }
            TimeUnit::Hz => sample_rate / self.hz,
        }
    }

    pub fn to_seconds(&self, bpm: f32, bars: f32) -> f32 {
        match self.unit {
            TimeUnit::Seconds => self.seconds,
            TimeUnit::Samples => self.samples / 44100.0,
            TimeUnit::Bars => {
                let bar_fraction = self.bar_num as f32 / self.bar_denom as f32;
                let seconds_per_bar = (60.0 / bpm) * 4.0 * bars;
                bar_fraction * seconds_per_bar
            }
            TimeUnit::Hz => 1.0 / self.hz,
        }
    }

    pub fn display(&self) -> String {
        match self.unit {
            TimeUnit::Seconds => {
                if self.seconds >= 1.0 {
                    format!("{:.2}s", self.seconds)
                } else {
                    format!("{:.0}ms", self.seconds * 1000.0)
                }
            }
            TimeUnit::Samples => format!("{:.0}smp", self.samples),
            TimeUnit::Bars => {
                if self.bar_num == 1 {
                    format!("1/{}", self.bar_denom)
                } else {
                    format!("{}/{}", self.bar_num, self.bar_denom)
                }
            }
            TimeUnit::Hz => {
                if self.hz >= 1.0 {
                    format!("{:.1}hz", self.hz)
                } else {
                    format!("{:.3}hz", self.hz)
                }
            }
        }
    }

    pub fn adjust(&mut self, up: bool, fast: bool) {
        let mult = if fast { 10.0 } else { 1.0 };
        match self.unit {
            TimeUnit::Seconds => {
                let step = if self.seconds < 0.1 {
                    0.001
                } else if self.seconds < 1.0 {
                    0.01
                } else {
                    0.1
                } * mult;
                if up {
                    self.seconds = (self.seconds + step).min(60.0);
                } else {
                    self.seconds = (self.seconds - step).max(0.001);
                }
            }
            TimeUnit::Samples => {
                let step = if self.samples < 1000.0 {
                    10.0
                } else if self.samples < 10000.0 {
                    100.0
                } else {
                    1000.0
                } * mult;
                if up {
                    self.samples = (self.samples + step).min(44100.0 * 60.0);
                } else {
                    self.samples = (self.samples - step).max(1.0);
                }
            }
            TimeUnit::Bars => {
                let max_num = if self.bar_denom == 1 {
                    16
                } else {
                    self.bar_denom
                };
                if up {
                    if self.bar_num < max_num {
                        self.bar_num += 1;
                    } else if self.bar_denom > 1 {
                        self.bar_denom /= 2;
                        self.bar_num = 1;
                    }
                } else {
                    if self.bar_num > 1 {
                        self.bar_num -= 1;
                    } else if self.bar_denom < 64 {
                        self.bar_denom *= 2;
                        self.bar_num = self.bar_denom;
                    }
                }
            }
            TimeUnit::Hz => {
                let step = if self.hz < 1.0 {
                    0.01
                } else if self.hz < 10.0 {
                    0.1
                } else if self.hz < 100.0 {
                    1.0
                } else {
                    10.0
                } * mult;
                if up {
                    self.hz = (self.hz + step).min(20000.0);
                } else {
                    self.hz = (self.hz - step).max(0.01);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum ParamKind {
    Float { min: f32, max: f32, step: f32 },
    Int { min: i32, max: i32 },
    Time,
    Input,
    Enum,
    Toggle,
}

impl ParamKind {
    pub fn is_port(&self) -> bool {
        matches!(
            self,
            ParamKind::Input | ParamKind::Float { .. } | ParamKind::Time
        )
    }
}

#[derive(Clone, Copy)]
pub struct ParamDef {
    pub name: &'static str,
    pub kind: ParamKind,
    pub desc: Option<&'static str>,
}

impl ParamDef {
    pub const fn new(name: &'static str, kind: ParamKind) -> Self {
        Self {
            name,
            kind,
            desc: None,
        }
    }

    pub const fn with_desc(name: &'static str, kind: ParamKind, desc: &'static str) -> Self {
        Self {
            name,
            kind,
            desc: Some(desc),
        }
    }
}

impl ModuleKind {
    pub fn param_defs(&self) -> &'static [ParamDef] {
        match self {
            ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree => &[],
            ModuleKind::DegreeGate => &[ParamDef {
                name: "Deg",
                kind: ParamKind::Int { min: 0, max: 12 },
                desc: None,
            }],
            ModuleKind::Osc => &[
                ParamDef {
                    name: "Wave",
                    kind: ParamKind::Enum,
                    desc: None,
                },
                ParamDef {
                    name: "Freq",
                    kind: ParamKind::Time,
                    desc: None,
                },
                ParamDef {
                    name: "Shift",
                    kind: ParamKind::Float {
                        min: -24.0,
                        max: 24.0,
                        step: 1.0,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Gain",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Uni",
                    kind: ParamKind::Toggle,
                    desc: None,
                },
            ],
            ModuleKind::Rise | ModuleKind::Fall => &[
                ParamDef {
                    name: "Gate",
                    kind: ParamKind::Input,
                    desc: None,
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Time,
                    desc: None,
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
                    desc: None,
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Time,
                    desc: None,
                },
            ],
            ModuleKind::Adsr => &[
                ParamDef {
                    name: "Rise",
                    kind: ParamKind::Input,
                    desc: None,
                },
                ParamDef {
                    name: "Fall",
                    kind: ParamKind::Input,
                    desc: None,
                },
                ParamDef {
                    name: "Atk",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Sus",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Envelope => &[ParamDef {
                name: "Phase",
                kind: ParamKind::Float {
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                },
                desc: None,
            }],
            ModuleKind::Lpf => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Freq",
                    kind: ParamKind::Float {
                        min: 0.001,
                        max: 0.99,
                        step: 0.01,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Q",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                    desc: None,
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
                    desc: None,
                },
                ParamDef {
                    name: "Freq",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Q",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Comb => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Time,
                    desc: Some("Delay time (sets pitch)"),
                },
                ParamDef {
                    name: "Fdbk",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 0.99,
                        step: 0.01,
                    },
                    desc: Some("Feedback (resonance)"),
                },
                ParamDef {
                    name: "Damp",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                    },
                    desc: Some("HF damping in feedback"),
                },
            ],
            ModuleKind::Allpass => &[
                ParamDef {
                    name: "In",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.01,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Time,
                    desc: Some("Delay time"),
                },
                ParamDef {
                    name: "Fdbk",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 0.99,
                        step: 0.01,
                    },
                    desc: Some("Coefficient (diffusion)"),
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
                    desc: None,
                },
                ParamDef {
                    name: "Time",
                    kind: ParamKind::Time,
                    desc: None,
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
                    desc: None,
                },
                ParamDef {
                    name: "Room",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: Some("Decay time / tail length"),
                },
                ParamDef {
                    name: "Damp",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: Some("HF decay (air absorption)"),
                },
                ParamDef {
                    name: "Mod",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: Some("Delay wobble (kills ringing)"),
                },
                ParamDef {
                    name: "Diff",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: Some("Echo smear (density)"),
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
                    desc: None,
                },
                ParamDef {
                    name: "Type",
                    kind: ParamKind::Enum,
                    desc: None,
                },
                ParamDef {
                    name: "Drive",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 20.0,
                        step: 0.1,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Asym",
                    kind: ParamKind::Float {
                        min: -1.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: None,
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
                    desc: None,
                },
                ParamDef {
                    name: "Rate",
                    kind: ParamKind::Float {
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Depth",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 1.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "Fdbk",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 0.95,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Mul => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 100.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 100.0,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Add => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],

            ModuleKind::Gt => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Lt => &[
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.05,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Switch => &[
                ParamDef {
                    name: "Sel",
                    kind: ParamKind::Input,
                    desc: None,
                },
                ParamDef {
                    name: "A",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                    desc: None,
                },
                ParamDef {
                    name: "B",
                    kind: ParamKind::Float {
                        min: -1000.0,
                        max: 1000.0,
                        step: 0.1,
                    },
                    desc: None,
                },
            ],
            ModuleKind::Rng => &[ParamDef {
                name: "Gate",
                kind: ParamKind::Input,
                desc: None,
            }],
            ModuleKind::Sample => &[
                ParamDef {
                    name: "File",
                    kind: ParamKind::Enum,
                    desc: None,
                },
                ParamDef {
                    name: "Pos",
                    kind: ParamKind::Input,
                    desc: None,
                },
            ],
            ModuleKind::Probe => &[ParamDef {
                name: "In",
                kind: ParamKind::Float {
                    min: -1.0,
                    max: 1.0,
                    step: 0.01,
                },
                desc: None,
            }],
            ModuleKind::Output => &[ParamDef {
                name: "In",
                kind: ParamKind::Float {
                    min: -1.0,
                    max: 1.0,
                    step: 0.01,
                },
                desc: None,
            }],
            ModuleKind::SubIn => &[],
            ModuleKind::SubOut => &[ParamDef {
                name: "In",
                kind: ParamKind::Input,
                desc: None,
            }],
            ModuleKind::DelayTap(_) => &[
                ParamDef {
                    name: "Src",
                    kind: ParamKind::Enum,
                    desc: None,
                },
                ParamDef {
                    name: "Gain",
                    kind: ParamKind::Float {
                        min: 0.0,
                        max: 0.7,
                        step: 0.05,
                    },
                    desc: None,
                },
            ],
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
    DegreeGate {
        degree: i32,
    },
    Osc {
        wave: WaveType,
        freq: TimeValue,
        shift: f32,
        gain: f32,
        uni: bool,
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
    Reverb {
        room: f32,
        damp: f32,
        mod_depth: f32,
        diffusion: f32,
        connected: u8,
    },
    Distortion {
        dist_type: DistType,
        drive: f32,
        asymmetry: f32,
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
        connected: u8,
    },
    SubPatch {
        inputs: u8,
        outputs: u8,
    },
    DelayTap {
        gain: f32,
    },
}

impl ModuleParams {
    pub fn default_for(kind: ModuleKind) -> Self {
        match kind {
            ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree => ModuleParams::None,
            ModuleKind::DegreeGate => ModuleParams::DegreeGate { degree: 0 },
            ModuleKind::Osc => ModuleParams::Osc {
                wave: WaveType::Sin,
                freq: TimeValue::from_hz(440.0),
                shift: 0.0,
                gain: 1.0,
                uni: false,
                connected: 0xFF,
            },
            ModuleKind::Rise => ModuleParams::Rise {
                time: TimeValue::from_seconds(0.1),
                connected: 0xFF,
            },
            ModuleKind::Fall => ModuleParams::Fall {
                time: TimeValue::from_seconds(0.1),
                connected: 0xFF,
            },
            ModuleKind::Ramp => ModuleParams::Ramp {
                value: 0.0,
                time: TimeValue::from_seconds(0.1),
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
            ModuleKind::Comb => ModuleParams::Comb {
                time: TimeValue::from_samples(441.0),
                feedback: 0.8,
                damp: 0.2,
                connected: 0xFF,
            },
            ModuleKind::Allpass => ModuleParams::Allpass {
                time: TimeValue::from_samples(441.0),
                feedback: 0.5,
                connected: 0xFF,
            },
            ModuleKind::Delay => ModuleParams::Delay {
                time: TimeValue::from_samples(4410.0),
                connected: 0xFF,
            },
            ModuleKind::Reverb => ModuleParams::Reverb {
                room: 0.5,
                damp: 0.3,
                mod_depth: 0.5,
                diffusion: 0.75,
                connected: 0xFF,
            },
            ModuleKind::Distortion => ModuleParams::Distortion {
                dist_type: DistType::Tube,
                drive: 2.0,
                asymmetry: 0.0,
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
            ModuleKind::Rng => ModuleParams::None,
            ModuleKind::Sample => ModuleParams::Sample {
                file_idx: 0,
                file_name: String::new(),
                samples: std::sync::Arc::new(Vec::new()),
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
                inputs: 0,
                outputs: 0,
            },
            ModuleKind::DelayTap(_) => ModuleParams::DelayTap { gain: 0.3 },
        }
    }

    pub fn connected(&self) -> u8 {
        match self {
            ModuleParams::None | ModuleParams::DegreeGate { .. } => 0xFF,
            ModuleParams::Osc { connected, .. } => *connected,
            ModuleParams::Rise { connected, .. } => *connected,
            ModuleParams::Fall { connected, .. } => *connected,
            ModuleParams::Ramp { connected, .. } => *connected,
            ModuleParams::Adsr { connected, .. } => *connected,
            ModuleParams::Envelope { connected, .. } => *connected,
            ModuleParams::Filter { connected, .. } => *connected,
            ModuleParams::Comb { connected, .. } => *connected,
            ModuleParams::Allpass { connected, .. } => *connected,
            ModuleParams::Delay { connected, .. } => *connected,
            ModuleParams::Reverb { connected, .. } => *connected,
            ModuleParams::Distortion { connected, .. } => *connected,
            ModuleParams::Flanger { connected, .. } => *connected,
            ModuleParams::Mul { connected, .. } => *connected,
            ModuleParams::Add { connected, .. } => *connected,

            ModuleParams::Gt { connected, .. } => *connected,
            ModuleParams::Lt { connected, .. } => *connected,
            ModuleParams::Switch { connected, .. } => *connected,
            ModuleParams::Sample { connected, .. } => *connected,
            ModuleParams::Probe { connected, .. } => *connected,
            ModuleParams::Output { connected, .. } => *connected,
            ModuleParams::SubPatch { .. } => 0xFF,
            ModuleParams::DelayTap { .. } => 0xFF,
        }
    }

    pub fn connected_mut(&mut self) -> Option<&mut u8> {
        match self {
            ModuleParams::None
            | ModuleParams::SubPatch { .. }
            | ModuleParams::DegreeGate { .. } => None,
            ModuleParams::Osc { connected, .. } => Some(connected),
            ModuleParams::Rise { connected, .. } => Some(connected),
            ModuleParams::Fall { connected, .. } => Some(connected),
            ModuleParams::Ramp { connected, .. } => Some(connected),
            ModuleParams::Adsr { connected, .. } => Some(connected),
            ModuleParams::Envelope { connected, .. } => Some(connected),
            ModuleParams::Filter { connected, .. } => Some(connected),
            ModuleParams::Comb { connected, .. } => Some(connected),
            ModuleParams::Allpass { connected, .. } => Some(connected),
            ModuleParams::Delay { connected, .. } => Some(connected),
            ModuleParams::Reverb { connected, .. } => Some(connected),
            ModuleParams::Distortion { connected, .. } => Some(connected),
            ModuleParams::Flanger { connected, .. } => Some(connected),
            ModuleParams::Mul { connected, .. } => Some(connected),
            ModuleParams::Add { connected, .. } => Some(connected),

            ModuleParams::Gt { connected, .. } => Some(connected),
            ModuleParams::Lt { connected, .. } => Some(connected),
            ModuleParams::Switch { connected, .. } => Some(connected),
            ModuleParams::Sample { connected, .. } => Some(connected),
            ModuleParams::Probe { connected, .. } => Some(connected),
            ModuleParams::Output { connected, .. } => Some(connected),
            ModuleParams::DelayTap { .. } => None,
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
            ModuleParams::Osc { shift, gain, .. } => match idx {
                2 => Some(*shift),
                3 => Some(*gain),
                _ => None,
            },
            ModuleParams::Rise { .. } | ModuleParams::Fall { .. } => None,
            ModuleParams::Ramp { value, .. } => match idx {
                0 => Some(*value),
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
            ModuleParams::Delay { .. } => None,
            ModuleParams::Reverb {
                room,
                damp,
                mod_depth,
                diffusion,
                ..
            } => match idx {
                1 => Some(*room),
                2 => Some(*damp),
                3 => Some(*mod_depth),
                4 => Some(*diffusion),
                _ => None,
            },
            ModuleParams::Distortion {
                drive, asymmetry, ..
            } => match idx {
                2 => Some(*drive),
                3 => Some(*asymmetry),
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
            ModuleParams::DelayTap { gain } => match idx {
                1 => Some(*gain),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn set_float(&mut self, idx: usize, val: f32) {
        match self {
            ModuleParams::Osc { shift, gain, .. } => match idx {
                2 => *shift = val,
                3 => *gain = val,
                _ => {}
            },
            ModuleParams::Rise { .. } | ModuleParams::Fall { .. } => {}
            ModuleParams::Ramp { value, .. } => match idx {
                0 => *value = val,
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
            ModuleParams::Delay { .. } => {}
            ModuleParams::Reverb {
                room,
                damp,
                mod_depth,
                diffusion,
                ..
            } => match idx {
                1 => *room = val,
                2 => *damp = val,
                3 => *mod_depth = val,
                4 => *diffusion = val,
                _ => {}
            },
            ModuleParams::Distortion {
                drive, asymmetry, ..
            } => match idx {
                2 => *drive = val,
                3 => *asymmetry = val,
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
            ModuleParams::DelayTap { gain } => match idx {
                1 => *gain = val,
                _ => {}
            },
            _ => {}
        }
    }

    pub fn get_time(&self, idx: usize) -> Option<&TimeValue> {
        match self {
            ModuleParams::Osc { freq, .. } => match idx {
                1 => Some(freq),
                _ => None,
            },
            ModuleParams::Rise { time, .. } | ModuleParams::Fall { time, .. } => match idx {
                1 => Some(time),
                _ => None,
            },
            ModuleParams::Ramp { time, .. } => match idx {
                1 => Some(time),
                _ => None,
            },
            ModuleParams::Delay { time, .. } => match idx {
                1 => Some(time),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_time_mut(&mut self, idx: usize) -> Option<&mut TimeValue> {
        match self {
            ModuleParams::Osc { freq, .. } => match idx {
                1 => Some(freq),
                _ => None,
            },
            ModuleParams::Rise { time, .. } | ModuleParams::Fall { time, .. } => match idx {
                1 => Some(time),
                _ => None,
            },
            ModuleParams::Ramp { time, .. } => match idx {
                1 => Some(time),
                _ => None,
            },
            ModuleParams::Delay { time, .. } => match idx {
                1 => Some(time),
                _ => None,
            },
            _ => None,
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

    pub fn cycle_enum_next(&mut self, idx: usize) {
        match self {
            ModuleParams::Osc { wave, .. } if idx == 0 => *wave = wave.next(),
            ModuleParams::Distortion { dist_type, .. } if idx == 1 => *dist_type = dist_type.next(),
            _ => {}
        }
    }

    pub fn cycle_enum_prev(&mut self, idx: usize) {
        match self {
            ModuleParams::Osc { wave, .. } if idx == 0 => *wave = wave.prev(),
            ModuleParams::Distortion { dist_type, .. } if idx == 1 => *dist_type = dist_type.prev(),
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

    pub fn get_int(&self, idx: usize) -> Option<i32> {
        match self {
            ModuleParams::DegreeGate { degree } => match idx {
                0 => Some(*degree),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn set_int(&mut self, idx: usize, val: i32) {
        match self {
            ModuleParams::DegreeGate { degree } => match idx {
                0 => *degree = val,
                _ => {}
            },
            _ => {}
        }
    }

    pub fn has_enum(&self) -> bool {
        matches!(self, ModuleParams::Osc { .. })
    }

    pub fn enum_display(&self, idx: usize) -> Option<&'static str> {
        match self {
            ModuleParams::Osc { wave, .. } if idx == 0 => Some(wave.name()),
            ModuleParams::Distortion { dist_type, .. } if idx == 1 => Some(dist_type.name()),
            _ => None,
        }
    }
}
