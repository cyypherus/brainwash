use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Freq,
    Gate,
    Osc,
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
    Output,
    LSplit,
    TSplit,
    RJoin,
    DJoin,
    TurnRD,
    TurnDR,
}

impl ModuleKind {
    pub fn name(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "Freq",
            ModuleKind::Gate => "Gate",
            ModuleKind::Osc => "Osc",
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
            ModuleKind::Output => "Out",
            ModuleKind::LSplit => "LSplit ◁",
            ModuleKind::TSplit => "USplit △",
            ModuleKind::RJoin => "RJoin ▶",
            ModuleKind::DJoin => "DJoin ▼",
            ModuleKind::TurnRD => "Turn ┐",
            ModuleKind::TurnDR => "Turn └",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            ModuleKind::Freq => "FRQ",
            ModuleKind::Gate => "GAT",
            ModuleKind::Osc => "OSC",
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
            ModuleKind::Output => "OUT",
            ModuleKind::LSplit => " ◁ ",
            ModuleKind::TSplit => " △ ",
            ModuleKind::RJoin => " ▶ ",
            ModuleKind::DJoin => " ▼ ",
            ModuleKind::TurnRD => " ┐ ",
            ModuleKind::TurnDR => " └ ",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ModuleKind::Freq | ModuleKind::Gate => Color::Rgb(100, 200, 100),
            ModuleKind::Osc => Color::Rgb(100, 150, 255),
            ModuleKind::Adsr | ModuleKind::Envelope => Color::Rgb(255, 200, 100),
            ModuleKind::Lpf
            | ModuleKind::Hpf
            | ModuleKind::Delay
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Flanger => Color::Rgb(200, 100, 255),
            ModuleKind::Mul
            | ModuleKind::Add => Color::Rgb(100, 220, 220),
            ModuleKind::Output => Color::Rgb(255, 100, 100),
            ModuleKind::LSplit
            | ModuleKind::TSplit
            | ModuleKind::RJoin
            | ModuleKind::DJoin
            | ModuleKind::TurnRD
            | ModuleKind::TurnDR => Color::Rgb(180, 180, 180),
        }
    }

    pub fn port_count(&self) -> usize {
        match self {
            ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::TurnRD | ModuleKind::TurnDR => 1,
            ModuleKind::RJoin | ModuleKind::DJoin => 2,
            _ => self.param_defs().iter().filter(|p| !matches!(p.kind, ParamKind::Enum { .. })).count(),
        }
    }

    pub fn output_count(&self) -> usize {
        match self {
            ModuleKind::Output => 0,
            ModuleKind::LSplit | ModuleKind::TSplit => 2,
            _ => 1,
        }
    }

    pub fn is_routing(&self) -> bool {
        matches!(self, ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::RJoin | ModuleKind::DJoin | ModuleKind::TurnRD | ModuleKind::TurnDR)
    }

    pub fn category(&self) -> ModuleCategory {
        match self {
            ModuleKind::Freq | ModuleKind::Gate => ModuleCategory::Track,
            ModuleKind::Osc => ModuleCategory::Generator,
            ModuleKind::Adsr | ModuleKind::Envelope => ModuleCategory::Envelope,
            ModuleKind::Lpf
            | ModuleKind::Hpf
            | ModuleKind::Delay
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Flanger => ModuleCategory::Effect,
            ModuleKind::Mul
            | ModuleKind::Add => ModuleCategory::Math,
            ModuleKind::Output => ModuleCategory::Output,
            ModuleKind::LSplit
            | ModuleKind::TSplit
            | ModuleKind::RJoin
            | ModuleKind::DJoin
            | ModuleKind::TurnRD
            | ModuleKind::TurnDR => ModuleCategory::Routing,
        }
    }

    pub fn all() -> &'static [ModuleKind] {
        &[
            ModuleKind::Freq,
            ModuleKind::Gate,
            ModuleKind::Osc,
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
            ModuleKind::Output,
            ModuleKind::LSplit,
            ModuleKind::TSplit,
            ModuleKind::RJoin,
            ModuleKind::DJoin,
            ModuleKind::TurnRD,
            ModuleKind::TurnDR,
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
            ModuleCategory::Output,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub id: ModuleId,
    pub kind: ModuleKind,
    pub orientation: Orientation,
    pub params: ModuleParams,
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

    pub fn width(&self) -> u8 {
        if self.kind.is_routing() {
            return 1;
        }
        match self.orientation {
            Orientation::Horizontal => 1,
            Orientation::Vertical => self.kind.port_count().max(1) as u8,
        }
    }

    pub fn height(&self) -> u8 {
        if self.kind.is_routing() {
            return 1;
        }
        match self.orientation {
            Orientation::Horizontal => self.kind.port_count().max(1) as u8,
            Orientation::Vertical => 1,
        }
    }

    pub fn has_input_top(&self) -> bool {
        if self.kind.port_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TurnRD => false,
            ModuleKind::TSplit | ModuleKind::TurnDR => true,
            ModuleKind::RJoin | ModuleKind::DJoin => true,
            _ => self.orientation == Orientation::Vertical,
        }
    }

    pub fn has_input_left(&self) -> bool {
        if self.kind.port_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TurnRD => true,
            ModuleKind::TSplit | ModuleKind::TurnDR => false,
            ModuleKind::RJoin | ModuleKind::DJoin => true,
            _ => self.orientation == Orientation::Horizontal,
        }
    }

    pub fn has_output_bottom(&self) -> bool {
        if self.kind.output_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::TurnRD => true,
            ModuleKind::RJoin | ModuleKind::TurnDR => false,
            ModuleKind::DJoin => true,
            _ => self.orientation == Orientation::Vertical,
        }
    }

    pub fn has_output_right(&self) -> bool {
        if self.kind.output_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::TurnDR => true,
            ModuleKind::RJoin => true,
            ModuleKind::DJoin | ModuleKind::TurnRD => false,
            _ => self.orientation == Orientation::Horizontal,
        }
    }

    pub fn is_port_open(&self, port_idx: usize) -> bool {
        if self.kind.is_routing() {
            return port_idx < self.kind.port_count();
        }
        
        let defs = self.kind.param_defs();
        let port_defs: Vec<_> = defs.iter().enumerate()
            .filter(|(_, d)| !matches!(d.kind, ParamKind::Enum { .. }))
            .collect();
        
        if let Some((param_idx, def)) = port_defs.get(port_idx) {
            match def.kind {
                ParamKind::Input => true,
                ParamKind::Float { .. } => self.params.is_connected(*param_idx),
                ParamKind::Enum { .. } => false,
            }
        } else {
            false
        }
    }
}

pub enum ParamKind {
    Float { min: f32, max: f32, step: f32 },
    Enum { options: &'static [&'static str] },
    Input,
}

pub struct ParamDef {
    pub name: &'static str,
    pub kind: ParamKind,
}

const WAVE_OPTIONS: &[&str] = &["sin", "square", "tri", "saw", "rsaw", "noise"];

impl ModuleKind {
    pub fn param_defs(&self) -> &'static [ParamDef] {
        match self {
            ModuleKind::Freq | ModuleKind::Gate => &[],
            ModuleKind::Osc => &[
                ParamDef { name: "Wave", kind: ParamKind::Enum { options: WAVE_OPTIONS } },
                ParamDef { name: "Freq", kind: ParamKind::Float { min: 20.0, max: 20000.0, step: 1.0 } },
                ParamDef { name: "Shift", kind: ParamKind::Float { min: -24.0, max: 24.0, step: 1.0 } },
                ParamDef { name: "Gain", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.05 } },
            ],
            ModuleKind::Adsr => &[
                ParamDef { name: "Gate", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 1.0 } },
                ParamDef { name: "Atk", kind: ParamKind::Float { min: 0.001, max: 2.0, step: 0.01 } },
                ParamDef { name: "Dec", kind: ParamKind::Float { min: 0.001, max: 2.0, step: 0.01 } },
                ParamDef { name: "Sus", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.05 } },
                ParamDef { name: "Rel", kind: ParamKind::Float { min: 0.001, max: 4.0, step: 0.01 } },
            ],
            ModuleKind::Envelope => &[
                ParamDef { name: "Phase", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.01 } },
            ],
            ModuleKind::Lpf | ModuleKind::Hpf => &[
                ParamDef { name: "In", kind: ParamKind::Float { min: -1.0, max: 1.0, step: 0.01 } },
                ParamDef { name: "Freq", kind: ParamKind::Float { min: 0.001, max: 0.99, step: 0.01 } },
                ParamDef { name: "Q", kind: ParamKind::Float { min: 0.1, max: 10.0, step: 0.1 } },
            ],
            ModuleKind::Delay => &[
                ParamDef { name: "In", kind: ParamKind::Float { min: -1.0, max: 1.0, step: 0.01 } },
                ParamDef { name: "Samp", kind: ParamKind::Float { min: 0.0, max: 44100.0, step: 100.0 } },
            ],
            ModuleKind::Reverb => &[
                ParamDef { name: "In", kind: ParamKind::Float { min: -1.0, max: 1.0, step: 0.01 } },
                ParamDef { name: "Room", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.05 } },
                ParamDef { name: "Damp", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.05 } },
            ],
            ModuleKind::Distortion => &[
                ParamDef { name: "In", kind: ParamKind::Float { min: -1.0, max: 1.0, step: 0.01 } },
                ParamDef { name: "Drive", kind: ParamKind::Float { min: 0.1, max: 0.5, step: 0.05 } },
                ParamDef { name: "Gain", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.05 } },
            ],
            ModuleKind::Flanger => &[
                ParamDef { name: "In", kind: ParamKind::Float { min: -1.0, max: 1.0, step: 0.01 } },
                ParamDef { name: "Rate", kind: ParamKind::Float { min: 0.1, max: 10.0, step: 0.1 } },
                ParamDef { name: "Depth", kind: ParamKind::Float { min: 0.0, max: 1.0, step: 0.05 } },
                ParamDef { name: "Fdbk", kind: ParamKind::Float { min: 0.0, max: 0.95, step: 0.05 } },
            ],
            ModuleKind::Mul => &[
                ParamDef { name: "A", kind: ParamKind::Float { min: 0.0, max: 100.0, step: 0.1 } },
                ParamDef { name: "B", kind: ParamKind::Float { min: 0.0, max: 100.0, step: 0.1 } },
            ],
            ModuleKind::Add => &[
                ParamDef { name: "A", kind: ParamKind::Float { min: -1000.0, max: 1000.0, step: 1.0 } },
                ParamDef { name: "B", kind: ParamKind::Float { min: -1000.0, max: 1000.0, step: 1.0 } },
            ],
            ModuleKind::Output => &[
                ParamDef { name: "In", kind: ParamKind::Float { min: -1.0, max: 1.0, step: 0.01 } },
            ],
            _ => &[],
        }
    }

}

#[derive(Clone, Debug, Default)]
pub struct ModuleParams {
    pub floats: [f32; 8],
    pub connected: u8,
}

impl ModuleParams {
    pub fn is_connected(&self, idx: usize) -> bool {
        (self.connected & (1 << idx)) != 0
    }

    pub fn set_connected(&mut self, idx: usize, val: bool) {
        if val {
            self.connected |= 1 << idx;
        } else {
            self.connected &= !(1 << idx);
        }
    }

    pub fn toggle_connected(&mut self, idx: usize) {
        self.connected ^= 1 << idx;
    }
}

impl ModuleParams {
    pub fn default_for(kind: ModuleKind) -> Self {
        let mut params = Self { connected: 0xFF, ..Default::default() };
        
        match kind {
            ModuleKind::Osc => {
                params.floats[1] = 440.0;
                params.floats[3] = 1.0;
            }
            ModuleKind::Adsr => {
                params.floats[1] = 0.01;
                params.floats[2] = 0.1;
                params.floats[3] = 0.7;
                params.floats[4] = 0.3;
            }
            ModuleKind::Lpf | ModuleKind::Hpf => {
                params.floats[1] = 0.5;
                params.floats[2] = 0.707;
            }
            ModuleKind::Reverb => {
                params.floats[1] = 0.5;
                params.floats[2] = 0.5;
            }
            ModuleKind::Distortion => {
                params.floats[1] = 0.3;
                params.floats[2] = 1.0;
            }
            ModuleKind::Flanger => {
                params.floats[1] = 0.5;
                params.floats[2] = 0.5;
                params.floats[3] = 0.3;
            }
            ModuleKind::Mul => {
                params.floats[0] = 1.0;
                params.floats[1] = 1.0;
            }
            _ => {}
        }
        params
    }
}
