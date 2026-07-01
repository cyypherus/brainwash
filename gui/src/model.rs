use crate::audio::{AudioHandle, MeterRoute, ProbeRoute, VoiceMode};
use brainwash::compile::{CompileError, CompiledPatch};
use brainwash::patch::{
    BinaryOp, ConnectError, Distortion as AudioDistortion, Drive, EnvPoint as AudioEnvPoint,
    Module as AudioModule, ModuleId as AudioModuleId, Patch, Wave,
};
use brainwash::project::{
    self, DistType as ProjectDistType, ModuleDef as ProjectModuleDef,
    ModuleKind as ProjectModuleKind, ModuleParams as ProjectModuleParams,
    Orientation as ProjectOrientation, Project, RoutingModule as ProjectRoutingModule,
    StandardModule as ProjectStandardModule, SubpatchDef as ProjectSubpatchDef,
    SubpatchModule as ProjectSubpatchModule, TimeUnit as ProjectTimeUnit,
    TimeValue as ProjectTimeValue, WaveType as ProjectWaveType,
};
use brainwash::sample::Unit;
use brainwash::scale::{
    amaj, amin, asharpmaj, asharpmin, bmaj, bmin, chromatic, cmaj, cmin, csharpmaj, csharpmin,
    dmaj, dmin, dsharpmaj, dsharpmin, emaj, emin, fmaj, fmin, fsharpmaj, fsharpmin, gmaj, gmin,
    gsharpmaj, gsharpmin,
};
use brainwash::time::{Duration, Hertz, SampleRate, Samples, Seconds};
use brainwash::{Scale, track::Track};
use haven::{ButtonState, TextState};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

const GRID_VIEW_MARGIN: u16 = 2;
const INSTRUMENT_COUNT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GridViewSize {
    columns: u16,
    rows: u16,
}

impl GridViewSize {
    pub(crate) fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns: columns.max(1),
            rows: rows.max(1),
        }
    }

    pub(crate) fn columns(self) -> u16 {
        self.columns
    }

    pub(crate) fn rows(self) -> u16 {
        self.rows
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connection {
    from: ModuleId,
    to: ModuleId,
    from_cell: GridPos,
    to_cell: GridPos,
    orientation: Orientation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuiAction {
    Left,
    Down,
    Up,
    Right,
    LeftFast,
    DownFast,
    UpFast,
    RightFast,
    OpenPalette,
    TogglePlay,
    ToggleMeters,
    Quit,
    Save,
    SaveAs,
    Load,
    Export,
    TrackSettings,
    TrackEdit,
    Search,
    EditSubpatch,
    ExitSubpatch,
    Palette(ModuleCategory),
    PaletteLeft,
    PaletteRight,
    PaletteUp,
    PaletteDown,
    Confirm,
    Cancel,
    Delete,
    Edit,
    Move,
    Copy,
    Rotate,
    Select,
    Undo,
    Redo,
    Instrument(usize),
    ValueDown,
    ValueUp,
    ValueDownFast,
    ValueUpFast,
    TogglePort,
    CycleUnit,
    CycleStep,
    TypeValue,
    InputChar(char),
    Backspace,
    DeleteChar,
    TextStart,
    TextEnd,
    AddPoint,
    DeletePoint,
    ToggleCurve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GridPointerPhase {
    Start,
    Drag,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnvPointerPhase {
    Start,
    Drag,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    QuitConfirm,
    Palette,
    Move {
        module: ModuleId,
        origin: GridPos,
    },
    Copy {
        source: ModuleId,
    },
    Edit {
        module: ModuleId,
        parameter: usize,
    },
    ValueInput {
        module: ModuleId,
        parameter: usize,
    },
    AdsrEdit {
        module: ModuleId,
        parameter: usize,
    },
    EnvEdit {
        module: ModuleId,
        point: usize,
        editing: bool,
    },
    ProbeEdit {
        module: ModuleId,
    },
    SampleView {
        module: ModuleId,
        zoom: u16,
        offset: u16,
    },
    Select {
        anchor: GridPos,
    },
    SelectMove {
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    },
    CopySelection {
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    },
    SavePrompt,
    SaveConfirm,
    ExportPrompt,
    ExportConfirm,
    TrackPrompt,
    TrackSettings {
        parameter: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioPatchError {
    MissingOutput,
    InvalidParameter,
    Connect(ConnectError),
    Compile(CompileError),
}

struct GuiAudioPatch {
    patch: CompiledPatch,
    probes: Vec<ProbeRoute>,
    meters: Vec<MeterRoute>,
    voice_mode: VoiceMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AudioKey {
    Root(ModuleId),
    Subpatch { owner: ModuleId, module: ModuleId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Lowpass,
    Highpass,
    Comb,
    Allpass,
    Delay,
    DelayTap,
    Reverb,
    Distortion,
    Compressor,
    Flanger,
    Probe,
    Multiply,
    Add,
    GreaterThan,
    LessThan,
    Switch,
    Random,
    Sample,
    Output,
    TurnRightDown,
    TurnDownRight,
    LeftSplit,
    TopSplit,
    RightJoin,
    DownJoin,
    SubpatchInput,
    SubpatchOutput,
    Subpatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleCategory {
    Source,
    Shape,
    Filter,
    Effect,
    Logic,
    Routing,
    Subpatch,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    id: ModuleId,
    kind: ModuleKind,
    position: GridPos,
    orientation: Orientation,
    parameters: Vec<ModuleParameter>,
    env_points: Vec<EnvPoint>,
    disabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Right,
    Down,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleParameter {
    name: &'static str,
    value: ParameterValue,
    connected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParameterValue {
    Float {
        value: i32,
        min: i32,
        max: i32,
        step: i32,
    },
    Int {
        value: i32,
        min: i32,
        max: i32,
    },
    Time {
        value: i32,
        unit: TimeUnit,
    },
    Input,
    Enum {
        index: usize,
        options: &'static [&'static str],
    },
    Toggle(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Samples,
    Bars,
    Hertz,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvPoint {
    pub time: i32,
    pub value: i32,
    pub curve: bool,
}

const SCALE_NAMES: &[&str] = &[
    "Chromatic",
    "C maj",
    "C min",
    "C# maj",
    "C# min",
    "D maj",
    "D min",
    "D# maj",
    "D# min",
    "E maj",
    "E min",
    "F maj",
    "F min",
    "F# maj",
    "F# min",
    "G maj",
    "G min",
    "G# maj",
    "G# min",
    "A maj",
    "A min",
    "A# maj",
    "A# min",
    "B maj",
    "B min",
];

const NUM_VOICES: usize = 6;

fn float_parameter(
    name: &'static str,
    min: i32,
    max: i32,
    step: i32,
    value: i32,
) -> ModuleParameter {
    ModuleParameter {
        name,
        value: ParameterValue::Float {
            value,
            min,
            max,
            step,
        },
        connected: true,
    }
}

fn int_parameter(name: &'static str, min: i32, max: i32, value: i32) -> ModuleParameter {
    ModuleParameter {
        name,
        value: ParameterValue::Int { value, min, max },
        connected: false,
    }
}

fn time_parameter(name: &'static str, value: i32, unit: TimeUnit) -> ModuleParameter {
    ModuleParameter {
        name,
        value: ParameterValue::Time { value, unit },
        connected: true,
    }
}

fn input_parameter(name: &'static str) -> ModuleParameter {
    ModuleParameter {
        name,
        value: ParameterValue::Input,
        connected: true,
    }
}

fn enum_parameter(
    name: &'static str,
    options: &'static [&'static str],
    index: usize,
) -> ModuleParameter {
    ModuleParameter {
        name,
        value: ParameterValue::Enum { index, options },
        connected: false,
    }
}

fn toggle_parameter(name: &'static str, value: bool) -> ModuleParameter {
    ModuleParameter {
        name,
        value: ParameterValue::Toggle(value),
        connected: false,
    }
}

#[derive(Debug)]
pub struct GuiState {
    width: u16,
    height: u16,
    grid_view: GridPos,
    grid_view_size: GridViewSize,
    mode: Mode,
    instruments: Vec<Instrument>,
    active_instrument: usize,
    playing: bool,
    show_meters: bool,
    bpm: u16,
    scale_index: usize,
    probe_voice: usize,
    track_edit_requested: bool,
    dirty: bool,
    should_quit: bool,
    step_size: usize,
    probe_len: u32,
    palette_category: ModuleCategory,
    palette_index: usize,
    palette_searching: bool,
    palette_filter: String,
    palette_filter_index: usize,
    next_module_id: u32,
    prompt_text: String,
    prompt_cursor: usize,
    pub(crate) prompt_input: TextState,
    saved_path: Option<String>,
    exported_path: Option<String>,
    load_requested: bool,
    export_loops: u16,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    pub(crate) play_button: ButtonState,
    pub(crate) meters_button: ButtonState,
    pub(crate) load_button: ButtonState,
    pub(crate) save_button: ButtonState,
    pub(crate) export_button: ButtonState,
    pub(crate) track_button: ButtonState,
    pub(crate) cancel_button: ButtonState,
    pub(crate) confirm_button: ButtonState,
    pointer_drag: Option<PointerDrag>,
    held_move: Option<HeldMove>,
    held_selection: Option<HeldSelection>,
    audio: Option<AudioHandle>,
    audio_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Instrument {
    root: PatchSurface,
    track_text: String,
    subpatches: Vec<SubpatchSurface>,
    editing_subpatch: Option<ModuleId>,
    subpatch_stack: Vec<(Option<ModuleId>, GridPos)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PatchSurface {
    cursor: GridPos,
    modules: Vec<Module>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SubpatchSurface {
    owner: ModuleId,
    surface: PatchSurface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    instruments: Vec<Instrument>,
    active_instrument: usize,
    next_module_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HeldMove {
    module: Module,
    origin_surface: Option<ModuleId>,
    origin_stack: Vec<(Option<ModuleId>, GridPos)>,
    origin: GridPos,
    before: Snapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HeldSelection {
    modules: Vec<Module>,
    origin_surface: Option<ModuleId>,
    origin_stack: Vec<(Option<ModuleId>, GridPos)>,
    origin: GridPos,
    before: Snapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerDrag {
    Module {
        module: ModuleId,
        origin: GridPos,
        grab: GridPos,
    },
    Selection {
        anchor: GridPos,
    },
    SelectionMove {
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextInputKind {
    Number,
    Time,
    Free,
}

fn text_input_accepts(kind: TextInputKind, character: char) -> bool {
    match kind {
        TextInputKind::Number => character.is_ascii_digit() || matches!(character, '.' | '-'),
        TextInputKind::Time => character.is_ascii_digit() || matches!(character, '.' | '-' | '/'),
        TextInputKind::Free => true,
    }
}

impl GridPos {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl ModuleId {
    pub fn value(self) -> u32 {
        self.0
    }
}

impl Connection {
    pub fn from(self) -> ModuleId {
        self.from
    }

    pub fn to(self) -> ModuleId {
        self.to
    }

    pub(crate) fn from_cell(self) -> GridPos {
        self.from_cell
    }

    pub(crate) fn to_cell(self) -> GridPos {
        self.to_cell
    }

    pub(crate) fn orientation(self) -> Orientation {
        self.orientation
    }
}

impl ModuleKind {
    pub fn category(self) -> ModuleCategory {
        match self {
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::DegreeGate
            | ModuleKind::Osc
            | ModuleKind::Random
            | ModuleKind::Sample => ModuleCategory::Source,
            ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope => ModuleCategory::Shape,
            ModuleKind::Lowpass | ModuleKind::Highpass => ModuleCategory::Filter,
            ModuleKind::Comb
            | ModuleKind::Allpass
            | ModuleKind::Delay
            | ModuleKind::DelayTap
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Compressor
            | ModuleKind::Flanger => ModuleCategory::Effect,
            ModuleKind::Multiply
            | ModuleKind::Add
            | ModuleKind::GreaterThan
            | ModuleKind::LessThan
            | ModuleKind::Switch
            | ModuleKind::Probe => ModuleCategory::Logic,
            ModuleKind::TurnRightDown
            | ModuleKind::TurnDownRight
            | ModuleKind::LeftSplit
            | ModuleKind::TopSplit
            | ModuleKind::RightJoin
            | ModuleKind::DownJoin => ModuleCategory::Routing,
            ModuleKind::SubpatchInput | ModuleKind::SubpatchOutput | ModuleKind::Subpatch => {
                ModuleCategory::Subpatch
            }
            ModuleKind::Output => ModuleCategory::Output,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ModuleKind::Freq => "Freq",
            ModuleKind::Gate => "Gate",
            ModuleKind::Degree => "Degree",
            ModuleKind::DegreeGate => "Degree Gate",
            ModuleKind::Osc => "Osc",
            ModuleKind::Rise => "Rise",
            ModuleKind::Fall => "Fall",
            ModuleKind::Ramp => "Ramp",
            ModuleKind::Adsr => "ADSR",
            ModuleKind::Envelope => "Envelope",
            ModuleKind::Lowpass => "LPF",
            ModuleKind::Highpass => "HPF",
            ModuleKind::Comb => "Comb",
            ModuleKind::Allpass => "Allpass",
            ModuleKind::Delay => "Delay",
            ModuleKind::DelayTap => "Delay Tap",
            ModuleKind::Reverb => "Reverb",
            ModuleKind::Distortion => "Distortion",
            ModuleKind::Compressor => "Compressor",
            ModuleKind::Flanger => "Flanger",
            ModuleKind::Probe => "Probe",
            ModuleKind::Multiply => "Mul",
            ModuleKind::Add => "Add",
            ModuleKind::GreaterThan => "GT",
            ModuleKind::LessThan => "LT",
            ModuleKind::Switch => "Switch",
            ModuleKind::Random => "RNG",
            ModuleKind::Sample => "Sample",
            ModuleKind::Output => "Output",
            ModuleKind::TurnRightDown => "Turn RD",
            ModuleKind::TurnDownRight => "Turn DR",
            ModuleKind::LeftSplit => "L Split",
            ModuleKind::TopSplit => "T Split",
            ModuleKind::RightJoin => "R Join",
            ModuleKind::DownJoin => "D Join",
            ModuleKind::SubpatchInput => "Sub In",
            ModuleKind::SubpatchOutput => "Sub Out",
            ModuleKind::Subpatch => "Subpatch",
        }
    }

    pub(crate) fn has_visual_editor(self) -> bool {
        self.special_editor().is_some()
    }

    fn default_parameters(self) -> Vec<ModuleParameter> {
        match self {
            ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree => Vec::new(),
            ModuleKind::DegreeGate => vec![int_parameter("Deg", 0, 12, 0)],
            ModuleKind::Osc => vec![
                enum_parameter("Wave", &["sin", "square", "tri", "saw", "rsaw", "noise"], 0),
                time_parameter("Freq", 440, TimeUnit::Hertz),
                float_parameter("Shift", -2400, 2400, 100, 0),
                float_parameter("Gain", 0, 100, 5, 100),
                toggle_parameter("Uni", false),
            ],
            ModuleKind::Rise | ModuleKind::Fall => {
                vec![
                    input_parameter("Gate"),
                    time_parameter("Time", 10, TimeUnit::Seconds),
                ]
            }
            ModuleKind::Ramp => vec![
                float_parameter("Val", -100_000, 100_000, 10, 0),
                time_parameter("Time", 10, TimeUnit::Seconds),
            ],
            ModuleKind::Adsr => vec![
                input_parameter("Rise"),
                input_parameter("Fall"),
                float_parameter("Atk", 0, 100, 5, 25),
                float_parameter("Sus", 0, 100, 5, 70),
            ],
            ModuleKind::Envelope => vec![float_parameter("Phase", 0, 100, 1, 0)],
            ModuleKind::Lowpass => vec![
                float_parameter("In", -100, 100, 1, 0),
                float_parameter("Freq", 0, 99, 1, 25),
                float_parameter("Q", 10, 1000, 10, 70),
            ],
            ModuleKind::Highpass => vec![
                float_parameter("In", -100, 100, 1, 0),
                float_parameter("Freq", 0, 100, 1, 25),
                float_parameter("Q", 10, 1000, 10, 70),
            ],
            ModuleKind::Comb => vec![
                float_parameter("In", -100, 100, 1, 0),
                time_parameter("Time", 441, TimeUnit::Samples),
                float_parameter("Fdbk", 0, 99, 1, 30),
                float_parameter("Damp", 0, 100, 1, 20),
            ],
            ModuleKind::Allpass => vec![
                float_parameter("In", -100, 100, 1, 0),
                time_parameter("Time", 441, TimeUnit::Samples),
                float_parameter("Fdbk", 0, 99, 1, 50),
            ],
            ModuleKind::Delay => vec![
                float_parameter("In", -100, 100, 1, 0),
                time_parameter("Time", 4410, TimeUnit::Samples),
            ],
            ModuleKind::DelayTap => vec![
                enum_parameter("Src", &["delay 1"], 0),
                float_parameter("Gain", 0, 70, 5, 50),
            ],
            ModuleKind::Reverb => vec![
                float_parameter("In", -100, 100, 1, 0),
                float_parameter("Room", 0, 100, 5, 50),
                float_parameter("Damp", 0, 100, 5, 30),
                float_parameter("Mod", 0, 100, 5, 20),
                float_parameter("Diff", 0, 100, 5, 50),
            ],
            ModuleKind::Distortion => vec![
                float_parameter("In", -100, 100, 1, 0),
                enum_parameter("Type", &["tube", "tape", "fuzz", "fold", "clip"], 0),
                float_parameter("Drive", 10, 2000, 10, 200),
                float_parameter("Asym", -100, 100, 5, 0),
            ],
            ModuleKind::Compressor => vec![
                float_parameter("In", -100, 100, 1, 0),
                float_parameter("Thresh", 1, 100, 1, 50),
                float_parameter("Ratio", 100, 2000, 50, 400),
                float_parameter("Atk", 0, 50, 1, 1),
                float_parameter("Rel", 1, 200, 1, 30),
                float_parameter("Gain", 0, 400, 10, 100),
            ],
            ModuleKind::Flanger => vec![
                float_parameter("In", -100, 100, 1, 0),
                float_parameter("Rate", 10, 1000, 10, 50),
                float_parameter("Depth", 0, 100, 5, 50),
                float_parameter("Fdbk", 0, 95, 5, 35),
            ],
            ModuleKind::Multiply => vec![
                float_parameter("A", 0, 10_000, 5, 100),
                float_parameter("B", 0, 10_000, 5, 100),
            ],
            ModuleKind::Add | ModuleKind::GreaterThan | ModuleKind::LessThan => vec![
                float_parameter("A", -100_000, 100_000, 5, 0),
                float_parameter("B", -100_000, 100_000, 5, 0),
            ],
            ModuleKind::Switch => vec![
                input_parameter("Sel"),
                float_parameter("A", -100_000, 100_000, 10, 0),
                float_parameter("B", -100_000, 100_000, 10, 100),
            ],
            ModuleKind::Random => vec![input_parameter("Gate")],
            ModuleKind::Sample => {
                vec![enum_parameter("File", &["none"], 0), input_parameter("Pos")]
            }
            ModuleKind::Probe => vec![float_parameter("In", -100, 100, 1, 0)],
            ModuleKind::Output => vec![
                input_parameter("In"),
                float_parameter("Gain", 0, 100, 1, 100),
            ],
            ModuleKind::SubpatchOutput => vec![input_parameter("In")],
            ModuleKind::TurnRightDown
            | ModuleKind::TurnDownRight
            | ModuleKind::LeftSplit
            | ModuleKind::TopSplit
            | ModuleKind::RightJoin
            | ModuleKind::DownJoin
            | ModuleKind::SubpatchInput
            | ModuleKind::Subpatch => Vec::new(),
        }
    }

    fn default_env_points(self) -> Vec<EnvPoint> {
        if self == ModuleKind::Envelope {
            vec![
                EnvPoint {
                    time: 0,
                    value: 0,
                    curve: false,
                },
                EnvPoint {
                    time: 100,
                    value: 100,
                    curve: false,
                },
            ]
        } else {
            Vec::new()
        }
    }

    fn special_editor(self) -> Option<SpecialEditor> {
        match self {
            ModuleKind::Adsr => Some(SpecialEditor::Adsr),
            ModuleKind::Envelope => Some(SpecialEditor::Envelope),
            ModuleKind::Probe => Some(SpecialEditor::Probe),
            ModuleKind::Sample => Some(SpecialEditor::Sample),
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::DegreeGate
            | ModuleKind::Osc
            | ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Lowpass
            | ModuleKind::Highpass
            | ModuleKind::Comb
            | ModuleKind::Allpass
            | ModuleKind::Delay
            | ModuleKind::DelayTap
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Compressor
            | ModuleKind::Flanger
            | ModuleKind::Multiply
            | ModuleKind::Add
            | ModuleKind::GreaterThan
            | ModuleKind::LessThan
            | ModuleKind::Switch
            | ModuleKind::Random
            | ModuleKind::Output
            | ModuleKind::TurnRightDown
            | ModuleKind::TurnDownRight
            | ModuleKind::LeftSplit
            | ModuleKind::TopSplit
            | ModuleKind::RightJoin
            | ModuleKind::DownJoin
            | ModuleKind::SubpatchInput
            | ModuleKind::SubpatchOutput
            | ModuleKind::Subpatch => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpecialEditor {
    Adsr,
    Envelope,
    Probe,
    Sample,
}

impl ModuleCategory {
    pub const ALL: [Self; 8] = [
        Self::Source,
        Self::Output,
        Self::Shape,
        Self::Filter,
        Self::Effect,
        Self::Logic,
        Self::Routing,
        Self::Subpatch,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ModuleCategory::Source => "Source",
            ModuleCategory::Shape => "Shape",
            ModuleCategory::Filter => "Filter",
            ModuleCategory::Effect => "Effect",
            ModuleCategory::Logic => "Logic",
            ModuleCategory::Routing => "Routing",
            ModuleCategory::Subpatch => "Subpatch",
            ModuleCategory::Output => "Output",
        }
    }
}

impl Module {
    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn kind(&self) -> ModuleKind {
        self.kind
    }

    pub fn position(&self) -> GridPos {
        self.position
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn parameters(&self) -> &[ModuleParameter] {
        &self.parameters
    }

    pub fn env_points(&self) -> &[EnvPoint] {
        &self.env_points
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    pub(crate) fn input_count(&self) -> u16 {
        if self.kind.is_routing() {
            return self.kind.input_count();
        }
        self.parameters
            .iter()
            .filter(|parameter| parameter.value.is_port())
            .count() as u16
    }

    pub(crate) fn input_connected(&self, port: u16) -> bool {
        self.parameters
            .iter()
            .filter(|parameter| parameter.value.is_port())
            .nth(port as usize)
            .map(|parameter| parameter.connected)
            .unwrap_or(true)
    }

    pub(crate) fn output_count(&self) -> u16 {
        self.kind.output_count()
    }

    pub(crate) fn has_input_top(&self) -> bool {
        if self.input_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::TopSplit
            | ModuleKind::TurnDownRight
            | ModuleKind::RightJoin
            | ModuleKind::DownJoin => true,
            ModuleKind::LeftSplit | ModuleKind::TurnRightDown => false,
            _ => self.orientation == Orientation::Down,
        }
    }

    pub(crate) fn has_input_left(&self) -> bool {
        if self.input_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LeftSplit
            | ModuleKind::TurnRightDown
            | ModuleKind::RightJoin
            | ModuleKind::DownJoin => true,
            ModuleKind::TopSplit | ModuleKind::TurnDownRight => false,
            _ => self.orientation == Orientation::Right,
        }
    }

    pub(crate) fn has_output_bottom(&self) -> bool {
        if self.output_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LeftSplit
            | ModuleKind::TopSplit
            | ModuleKind::TurnRightDown
            | ModuleKind::DownJoin => true,
            ModuleKind::RightJoin | ModuleKind::TurnDownRight => false,
            _ => self.orientation == Orientation::Down,
        }
    }

    pub(crate) fn has_output_right(&self) -> bool {
        if self.output_count() == 0 {
            return false;
        }
        match self.kind {
            ModuleKind::LeftSplit
            | ModuleKind::TopSplit
            | ModuleKind::TurnDownRight
            | ModuleKind::RightJoin => true,
            ModuleKind::DownJoin | ModuleKind::TurnRightDown => false,
            _ => self.orientation == Orientation::Right,
        }
    }
}

impl ModuleParameter {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn value(&self) -> &ParameterValue {
        &self.value
    }

    pub fn connected(&self) -> bool {
        self.connected
    }

    pub fn value_label(&self) -> String {
        self.value.label()
    }
}

impl ParameterValue {
    pub fn label(&self) -> String {
        match self {
            ParameterValue::Float { value, .. } => format!("{:.2}", *value as f32 / 100.0),
            ParameterValue::Int { value, .. } => value.to_string(),
            ParameterValue::Time { value, unit } => match unit {
                TimeUnit::Seconds => format!("{:.2}s", *value as f32 / 100.0),
                TimeUnit::Samples => format!("{value} smp"),
                TimeUnit::Bars => format!("{}/16", value.max(&1)),
                TimeUnit::Hertz => format!("{value} hz"),
            },
            ParameterValue::Input => "input".to_string(),
            ParameterValue::Enum { index, options } => {
                options.get(*index).copied().unwrap_or_default().to_string()
            }
            ParameterValue::Toggle(value) => {
                if *value {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
        }
    }

    fn accepts_typed_value(&self) -> bool {
        matches!(
            self,
            ParameterValue::Float { .. } | ParameterValue::Time { .. }
        )
    }

    fn is_port(&self) -> bool {
        matches!(
            self,
            ParameterValue::Float { .. } | ParameterValue::Time { .. } | ParameterValue::Input
        )
    }

    fn input_text(&self) -> String {
        match self {
            ParameterValue::Float { value, .. } => format!("{}", *value as f32 / 100.0),
            ParameterValue::Time { value, unit } => match unit {
                TimeUnit::Seconds => format!("{}", *value as f32 / 100.0),
                TimeUnit::Samples | TimeUnit::Hertz => value.to_string(),
                TimeUnit::Bars => format!("{}/16", value.max(&1)),
            },
            ParameterValue::Int { value, .. } => value.to_string(),
            ParameterValue::Input => String::new(),
            ParameterValue::Enum { index, options } => {
                options.get(*index).copied().unwrap_or_default().to_string()
            }
            ParameterValue::Toggle(value) => {
                if *value {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
        }
    }

    fn adjust(&mut self, up: bool, fast: bool, step_scale: i32) -> bool {
        match self {
            ParameterValue::Float {
                value,
                min,
                max,
                step,
            } => {
                let delta = *step * if fast { 10 } else { 1 } * step_scale;
                let next = if up {
                    (*value + delta).min(*max)
                } else {
                    (*value - delta).max(*min)
                };
                let changed = next != *value;
                *value = next;
                changed
            }
            ParameterValue::Int { value, min, max } => {
                let delta = if fast { 10 } else { 1 };
                let next = if up {
                    (*value + delta).min(*max)
                } else {
                    (*value - delta).max(*min)
                };
                let changed = next != *value;
                *value = next;
                changed
            }
            ParameterValue::Time { value, .. } => {
                let delta = if fast { 10 } else { 1 } * step_scale;
                let next = if up {
                    (*value + delta).min(10_000)
                } else {
                    (*value - delta).max(1)
                };
                let changed = next != *value;
                *value = next;
                changed
            }
            ParameterValue::Input => false,
            ParameterValue::Enum { index, options } => {
                if options.is_empty() {
                    return false;
                }
                *index = if up {
                    (*index + 1) % options.len()
                } else if *index == 0 {
                    options.len() - 1
                } else {
                    *index - 1
                };
                true
            }
            ParameterValue::Toggle(value) => {
                *value = !*value;
                true
            }
        }
    }

    fn cycle_unit(&mut self) -> bool {
        let ParameterValue::Time { unit, .. } = self else {
            return false;
        };
        *unit = match unit {
            TimeUnit::Seconds => TimeUnit::Samples,
            TimeUnit::Samples => TimeUnit::Bars,
            TimeUnit::Bars => TimeUnit::Hertz,
            TimeUnit::Hertz => TimeUnit::Seconds,
        };
        true
    }
}

impl Instrument {
    fn new() -> Self {
        Self {
            root: PatchSurface::new(),
            track_text: "(0/2/4/7)".to_string(),
            subpatches: Vec::new(),
            editing_subpatch: None,
            subpatch_stack: Vec::new(),
        }
    }

    fn surface(&self) -> &PatchSurface {
        self.editing_subpatch
            .and_then(|owner| {
                self.subpatches
                    .iter()
                    .find(|subpatch| subpatch.owner == owner)
                    .map(|subpatch| &subpatch.surface)
            })
            .unwrap_or(&self.root)
    }

    fn surface_mut(&mut self) -> &mut PatchSurface {
        let Some(owner) = self.editing_subpatch else {
            return &mut self.root;
        };
        if let Some(index) = self
            .subpatches
            .iter()
            .position(|subpatch| subpatch.owner == owner)
        {
            &mut self.subpatches[index].surface
        } else {
            self.editing_subpatch = None;
            &mut self.root
        }
    }

    fn ensure_subpatch(&mut self, owner: ModuleId) {
        if self
            .subpatches
            .iter()
            .all(|subpatch| subpatch.owner != owner)
        {
            self.subpatches.push(SubpatchSurface {
                owner,
                surface: PatchSurface::new(),
            });
        }
    }
}

impl PatchSurface {
    fn new() -> Self {
        Self {
            cursor: GridPos::new(0, 0),
            modules: Vec::new(),
        }
    }
}

fn update_surface_disabled_states(surface: &mut PatchSurface, footprints: &[(ModuleId, u16, u16)]) {
    let mut disabled = Vec::new();
    for left in 0..surface.modules.len() {
        for right in left + 1..surface.modules.len() {
            let a = &surface.modules[left];
            let b = &surface.modules[right];
            let (_, a_width, a_height) = footprints[left];
            let (_, b_width, b_height) = footprints[right];
            if a.position.x < b.position.x + b_width
                && b.position.x < a.position.x + a_width
                && a.position.y < b.position.y + b_height
                && b.position.y < a.position.y + a_height
            {
                disabled.push(a.id);
                disabled.push(b.id);
            }
        }
    }
    disabled.sort_by_key(|id| id.0);
    disabled.dedup();
    for module in &mut surface.modules {
        module.disabled = disabled.contains(&module.id);
    }
}

fn module_footprint(module: &Module, subpatch_ports: Option<(u16, u16)>) -> (u16, u16) {
    if module.kind.is_routing() {
        return (1, 1);
    }
    let (inputs, outputs) =
        subpatch_ports.unwrap_or_else(|| (module.input_count(), module.output_count()));
    match module.orientation {
        Orientation::Right => (outputs.max(1), inputs.max(1)),
        Orientation::Down => (inputs.max(1), outputs.max(1)),
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self::new(32, 24)
    }
}

impl GuiState {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            grid_view: GridPos::new(0, 0),
            grid_view_size: GridViewSize::new(width, height),
            mode: Mode::Normal,
            instruments: (0..INSTRUMENT_COUNT).map(|_| Instrument::new()).collect(),
            active_instrument: 0,
            playing: false,
            show_meters: false,
            bpm: 120,
            scale_index: 2,
            probe_voice: 0,
            track_edit_requested: false,
            dirty: false,
            should_quit: false,
            step_size: 1,
            probe_len: 4410,
            palette_category: ModuleCategory::Source,
            palette_index: 0,
            palette_searching: false,
            palette_filter: String::new(),
            palette_filter_index: 0,
            next_module_id: 0,
            prompt_text: String::new(),
            prompt_cursor: 0,
            prompt_input: TextState::new(""),
            saved_path: None,
            exported_path: None,
            load_requested: false,
            export_loops: 1,
            undo: Vec::new(),
            redo: Vec::new(),
            play_button: ButtonState::default(),
            meters_button: ButtonState::default(),
            load_button: ButtonState::default(),
            save_button: ButtonState::default(),
            export_button: ButtonState::default(),
            track_button: ButtonState::default(),
            cancel_button: ButtonState::default(),
            confirm_button: ButtonState::default(),
            pointer_drag: None,
            held_move: None,
            held_selection: None,
            audio: None,
            audio_status: "Audio disabled".to_string(),
        }
    }

    pub fn set_audio(&mut self, mut audio: AudioHandle) {
        audio.set_playing(self.playing);
        self.audio = Some(audio);
        self.sync_audio_patch();
        self.sync_audio_track();
    }

    pub fn set_audio_unavailable(&mut self, reason: impl Into<String>) {
        self.audio = None;
        self.audio_status = format!("Audio disabled: {}", reason.into());
    }

    pub fn cursor(&self) -> GridPos {
        self.instrument().surface().cursor
    }

    pub(crate) fn grid_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub(crate) fn set_grid_view_size(&mut self, size: GridViewSize) {
        if self.grid_view_size != size {
            self.grid_view_size = size;
            self.update_grid_view();
        }
    }

    pub(crate) fn grid_view_offset_for_size(&self, size: GridViewSize) -> GridPos {
        let (min, max) = self.active_grid_rect();
        GridPos::new(
            updated_axis_view(self.grid_view.x, min.x, max.x, self.width, size.columns()),
            updated_axis_view(self.grid_view.y, min.y, max.y, self.height, size.rows()),
        )
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn modules(&self) -> &[Module] {
        &self.instrument().surface().modules
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn audio_status(&self) -> &str {
        &self.audio_status
    }

    pub fn show_meters(&self) -> bool {
        self.show_meters
    }

    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    pub fn scale_index(&self) -> usize {
        self.scale_index
    }

    pub fn scale_name(&self) -> &'static str {
        SCALE_NAMES[self.scale_index]
    }

    pub fn probe_voice(&self) -> usize {
        self.probe_voice
    }

    pub fn track_edit_requested(&self) -> bool {
        self.track_edit_requested
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn step_size(&self) -> usize {
        self.step_size
    }

    pub fn probe_len(&self) -> u32 {
        self.probe_len
    }

    pub(crate) fn probe_history(&self, module: ModuleId) -> Vec<f32> {
        self.audio
            .as_ref()
            .map(|audio| {
                audio.probe_history(module.value(), self.probe_voice, self.probe_len as usize)
            })
            .unwrap_or_default()
    }

    pub(crate) fn meter_values(&self, module: ModuleId) -> Vec<f32> {
        self.audio
            .as_ref()
            .map(|audio| audio.meter_values(module.value(), self.probe_voice))
            .unwrap_or_default()
    }

    pub fn track_text(&self) -> &str {
        &self.instrument().track_text
    }

    pub fn active_instrument(&self) -> usize {
        self.active_instrument
    }

    pub fn instrument_count(&self) -> usize {
        self.instruments.len()
    }

    pub fn subpatch_depth(&self) -> usize {
        self.instrument()
            .subpatch_stack
            .len()
            .max(usize::from(self.instrument().editing_subpatch.is_some()))
    }

    pub fn prompt_text(&self) -> &str {
        &self.prompt_text
    }

    pub fn prompt_cursor(&self) -> usize {
        self.prompt_cursor
    }

    pub(crate) fn sync_track_prompt_text(&mut self, text: String) {
        self.prompt_text = text;
        self.prompt_cursor = self.prompt_text.len();
    }

    pub fn saved_path(&self) -> Option<&str> {
        self.saved_path.as_deref()
    }

    pub fn exported_path(&self) -> Option<&str> {
        self.exported_path.as_deref()
    }

    pub fn take_load_request(&mut self) -> bool {
        let requested = self.load_requested;
        self.load_requested = false;
        requested
    }

    pub fn load_project(&mut self, path: &Path) -> std::io::Result<()> {
        let project = project::load(path)?;
        self.set_project(project, path.to_string_lossy().into_owned());
        Ok(())
    }

    pub fn export_loops(&self) -> u16 {
        self.export_loops
    }

    pub fn palette_category(&self) -> ModuleCategory {
        self.palette_category
    }

    pub fn palette_searching(&self) -> bool {
        self.palette_searching
    }

    pub fn palette_filter(&self) -> &str {
        &self.palette_filter
    }

    pub fn palette_modules(&self) -> Vec<ModuleKind> {
        all_modules()
            .iter()
            .copied()
            .filter(|kind| kind.category() == self.palette_category)
            .collect()
    }

    pub fn filtered_palette_modules(&self) -> Vec<ModuleKind> {
        if self.palette_filter.is_empty() {
            return Vec::new();
        }
        let filter = self.palette_filter.to_lowercase();
        all_modules()
            .iter()
            .copied()
            .filter(|kind| kind.label().to_lowercase().contains(&filter))
            .collect()
    }

    pub fn selected_filtered_palette_module(&self) -> Option<ModuleKind> {
        self.filtered_palette_modules()
            .get(self.palette_filter_index)
            .copied()
    }

    pub fn selected_palette_module(&self) -> ModuleKind {
        let modules = self.palette_modules();
        modules[self.palette_index.min(modules.len().saturating_sub(1))]
    }

    pub fn module_at(&self, position: GridPos) -> Option<&Module> {
        self.modules()
            .iter()
            .find(|module| self.module_contains(module, position))
    }

    pub(crate) fn moving_module(&self, module: ModuleId) -> Option<&Module> {
        self.modules()
            .iter()
            .find(|candidate| candidate.id == module)
            .or_else(|| {
                self.held_move
                    .as_ref()
                    .and_then(|held| (held.module.id == module).then_some(&held.module))
            })
    }

    pub(crate) fn module_width(&self, module: &Module) -> u16 {
        if module.kind.is_routing() {
            return 1;
        }
        match module.orientation {
            Orientation::Right => self.module_output_count(module).max(1),
            Orientation::Down => self.module_input_count(module).max(1),
        }
    }

    pub(crate) fn module_height(&self, module: &Module) -> u16 {
        if module.kind.is_routing() {
            return 1;
        }
        match module.orientation {
            Orientation::Right => self.module_input_count(module).max(1),
            Orientation::Down => self.module_output_count(module).max(1),
        }
    }

    pub(crate) fn module_input_count(&self, module: &Module) -> u16 {
        if module.kind == ModuleKind::Subpatch {
            return self.subpatch_port_counts(module.id).0;
        }
        module.input_count()
    }

    pub(crate) fn module_output_count(&self, module: &Module) -> u16 {
        if module.kind == ModuleKind::Subpatch {
            return self.subpatch_port_counts(module.id).1;
        }
        module.output_count()
    }

    pub(crate) fn module_input_connected(&self, module: &Module, port: u16) -> bool {
        if module.kind == ModuleKind::Subpatch {
            return true;
        }
        module.input_connected(port)
    }

    pub(crate) fn module_has_input_top(&self, module: &Module) -> bool {
        if self.module_input_count(module) == 0 {
            return false;
        }
        if module.kind == ModuleKind::Subpatch {
            return module.orientation == Orientation::Down;
        }
        module.has_input_top()
    }

    pub(crate) fn module_has_input_left(&self, module: &Module) -> bool {
        if self.module_input_count(module) == 0 {
            return false;
        }
        if module.kind == ModuleKind::Subpatch {
            return module.orientation == Orientation::Right;
        }
        module.has_input_left()
    }

    pub(crate) fn module_has_output_bottom(&self, module: &Module) -> bool {
        if self.module_output_count(module) == 0 {
            return false;
        }
        if module.kind == ModuleKind::Subpatch {
            return module.orientation == Orientation::Down;
        }
        module.has_output_bottom()
    }

    pub(crate) fn module_has_output_right(&self, module: &Module) -> bool {
        if self.module_output_count(module) == 0 {
            return false;
        }
        if module.kind == ModuleKind::Subpatch {
            return module.orientation == Orientation::Right;
        }
        module.has_output_right()
    }

    fn subpatch_port_counts(&self, owner: ModuleId) -> (u16, u16) {
        self.instrument()
            .subpatches
            .iter()
            .find(|subpatch| subpatch.owner == owner)
            .map(|subpatch| {
                let inputs = subpatch
                    .surface
                    .modules
                    .iter()
                    .filter(|module| module.kind == ModuleKind::SubpatchInput)
                    .count() as u16;
                let outputs = subpatch
                    .surface
                    .modules
                    .iter()
                    .filter(|module| module.kind == ModuleKind::SubpatchOutput)
                    .count() as u16;
                (inputs, outputs)
            })
            .unwrap_or((0, 0))
    }

    fn module_contains(&self, module: &Module, position: GridPos) -> bool {
        position.x >= module.position.x
            && position.x < module.position.x + self.module_width(module)
            && position.y >= module.position.y
            && position.y < module.position.y + self.module_height(module)
    }

    fn module_overlaps(&self, module: &Module, position: GridPos, width: u16, height: u16) -> bool {
        module.position.x < position.x + width
            && module.position.x + self.module_width(module) > position.x
            && module.position.y < position.y + height
            && module.position.y + self.module_height(module) > position.y
    }

    fn moved_position(
        &self,
        module: &Module,
        _origin: GridPos,
        grab: GridPos,
        target: GridPos,
    ) -> GridPos {
        let x = module.position.x as i16 + target.x as i16 - grab.x as i16;
        let y = module.position.y as i16 + target.y as i16 - grab.y as i16;
        self.bounded_module_position(module, x, y)
            .unwrap_or(module.position)
    }

    fn bounded_module_position(&self, module: &Module, x: i16, y: i16) -> Option<GridPos> {
        let max_x = self.width.checked_sub(self.module_width(module))? as i16;
        let max_y = self.height.checked_sub(self.module_height(module))? as i16;
        Some(GridPos::new(
            x.clamp(0, max_x) as u16,
            y.clamp(0, max_y) as u16,
        ))
    }

    pub fn selected_modules(&self) -> Vec<ModuleId> {
        let Some((anchor, extent)) = self.selection() else {
            return Vec::new();
        };
        let min_x = anchor.x.min(extent.x);
        let max_x = anchor.x.max(extent.x);
        let min_y = anchor.y.min(extent.y);
        let max_y = anchor.y.max(extent.y);
        let position = GridPos::new(min_x, min_y);
        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        self.modules()
            .iter()
            .filter(|module| self.module_overlaps(module, position, width, height))
            .map(|module| module.id)
            .collect()
    }

    pub fn selection(&self) -> Option<(GridPos, GridPos)> {
        match self.mode {
            Mode::Select { anchor } => Some((anchor, self.cursor())),
            Mode::SelectMove { anchor, extent, .. } => Some((anchor, extent)),
            Mode::CopySelection { anchor, extent, .. } => Some((anchor, extent)),
            Mode::Normal
            | Mode::QuitConfirm
            | Mode::Palette
            | Mode::Move { .. }
            | Mode::Copy { .. } => None,
            Mode::Edit { .. }
            | Mode::ValueInput { .. }
            | Mode::AdsrEdit { .. }
            | Mode::EnvEdit { .. }
            | Mode::ProbeEdit { .. }
            | Mode::SampleView { .. }
            | Mode::SavePrompt
            | Mode::SaveConfirm
            | Mode::ExportPrompt
            | Mode::ExportConfirm
            | Mode::TrackPrompt
            | Mode::TrackSettings { .. } => None,
        }
    }

    pub fn connections(&self) -> Vec<Connection> {
        self.surface_connections(self.modules())
    }

    fn surface_connections(&self, modules: &[Module]) -> Vec<Connection> {
        let mut connections = Vec::new();
        for source in modules {
            if source.disabled {
                continue;
            }
            if self.module_has_output_right(source) {
                let y = source.position.y;
                let from_cell = GridPos::new(source.position.x + self.module_width(source) - 1, y);
                for x in source.position.x + self.module_width(source)..self.width {
                    let Some(target) = modules.iter().find(|module| {
                        !module.disabled && self.module_contains(module, GridPos::new(x, y))
                    }) else {
                        continue;
                    };
                    if target.id == source.id {
                        continue;
                    }
                    let local_y = y - target.position.y;
                    if x == target.position.x && self.module_has_input_left(target) {
                        connections.push(Connection {
                            from: source.id,
                            to: target.id,
                            from_cell,
                            to_cell: GridPos::new(target.position.x, target.position.y + local_y),
                            orientation: Orientation::Right,
                        });
                    }
                    break;
                }
            }

            if self.module_has_output_bottom(source) {
                let x = source.position.x;
                let from_cell = GridPos::new(x, source.position.y + self.module_height(source) - 1);
                for y in source.position.y + self.module_height(source)..self.height {
                    let Some(target) = modules.iter().find(|module| {
                        !module.disabled && self.module_contains(module, GridPos::new(x, y))
                    }) else {
                        continue;
                    };
                    if target.id == source.id {
                        continue;
                    }
                    let local_x = x - target.position.x;
                    if y == target.position.y && self.module_has_input_top(target) {
                        connections.push(Connection {
                            from: source.id,
                            to: target.id,
                            from_cell,
                            to_cell: GridPos::new(target.position.x + local_x, target.position.y),
                            orientation: Orientation::Down,
                        });
                    }
                    break;
                }
            }
        }
        connections
    }

    fn module_fits(&self, module: &Module, position: GridPos, ignored: &[ModuleId]) -> bool {
        self.area_fits(
            self.module_width(module),
            self.module_height(module),
            position,
            ignored,
        )
    }

    fn kind_fits(&self, kind: ModuleKind, orientation: Orientation, position: GridPos) -> bool {
        let width = kind.width(orientation);
        let height = kind.height(orientation);
        self.area_fits(width, height, position, &[])
    }

    fn area_fits(&self, width: u16, height: u16, position: GridPos, ignored: &[ModuleId]) -> bool {
        if position.x + width > self.width || position.y + height > self.height {
            return false;
        }
        !self
            .modules()
            .iter()
            .filter(|module| !ignored.contains(&module.id))
            .any(|module| self.module_overlaps(module, position, width, height))
    }

    pub fn compile_audio_patch(&self, rate: SampleRate) -> Result<CompiledPatch, AudioPatchError> {
        self.compile_gui_audio_patch(rate).map(|audio| audio.patch)
    }

    fn compile_gui_audio_patch(&self, rate: SampleRate) -> Result<GuiAudioPatch, AudioPatchError> {
        let instrument = self.instrument();
        let root_modules = &instrument.root.modules;
        let output = self
            .instrument()
            .root
            .modules
            .iter()
            .find(|module| !module.disabled && module.kind == ModuleKind::Output)
            .ok_or(AudioPatchError::MissingOutput)?;
        let connections = self.surface_connections(root_modules);
        let mut needed = Vec::new();
        let mut has_output_signal = false;
        for connection in connections
            .iter()
            .filter(|connection| connection.to == output.id)
        {
            let Some(input) = connection_input(connection, output, self.module_input_count(output))
            else {
                continue;
            };
            if !self.module_input_connected(output, input as u16) {
                continue;
            }
            if input == 0 {
                has_output_signal = true;
            }
            collect_audio_inputs(connection.from, &connections, &mut needed);
        }
        if !has_output_signal {
            return Err(AudioPatchError::MissingOutput);
        }
        for module in root_modules
            .iter()
            .filter(|module| !module.disabled && module.kind == ModuleKind::Probe)
        {
            collect_audio_inputs(module.id, &connections, &mut needed);
        }
        for subpatch in &instrument.subpatches {
            if subpatch
                .surface
                .modules
                .iter()
                .any(|module| !module.disabled && module.kind == ModuleKind::Probe)
                && let Some(owner) = root_modules
                    .iter()
                    .find(|module| !module.disabled && module.id == subpatch.owner)
            {
                collect_audio_inputs(owner.id, &connections, &mut needed);
            }
        }
        let voice_mode = if root_modules
            .iter()
            .any(|module| needed.contains(&module.id) && module_uses_voice_controls(module.kind))
            || needed
                .iter()
                .filter_map(|id| {
                    instrument
                        .subpatches
                        .iter()
                        .find(|subpatch| subpatch.owner == *id)
                })
                .any(|subpatch| {
                    subpatch
                        .surface
                        .modules
                        .iter()
                        .any(|module| !module.disabled && module_uses_voice_controls(module.kind))
                }) {
            VoiceMode::Polyphonic
        } else {
            VoiceMode::Single
        };

        let mut patch = Patch::new();
        let mut ids = Vec::new();
        let mut probes = Vec::new();
        let mut meters = Vec::new();
        for id in needed.iter().rev().copied() {
            let module = root_modules
                .iter()
                .find(|module| !module.disabled && module.id == id)
                .ok_or(AudioPatchError::Compile(CompileError::MissingModule))?;
            if module.kind == ModuleKind::Subpatch {
                continue;
            }
            let audio = audio_module(module, rate, self.bpm)?;
            let audio_id = patch.insert(audio);
            if module.kind == ModuleKind::Probe {
                probes.push(ProbeRoute {
                    source: audio_id,
                    target: module.id.value(),
                });
            }
            let input_count = self.module_input_count(module);
            if self.show_meters && input_count > 0 {
                if let Some(route) =
                    MeterRoute::new(audio_id, module.id.value(), input_count as usize)
                {
                    meters.push(route);
                }
            }
            ids.push((AudioKey::Root(id), audio_id));
        }

        for owner in needed
            .iter()
            .filter_map(|id| {
                root_modules
                    .iter()
                    .find(|module| module.id == *id && module.kind == ModuleKind::Subpatch)
            })
            .filter_map(|module| {
                instrument
                    .subpatches
                    .iter()
                    .find(|subpatch| subpatch.owner == module.id)
                    .map(|subpatch| (module.id, subpatch))
            })
        {
            let (owner_id, subpatch) = owner;
            for module in subpatch
                .surface
                .modules
                .iter()
                .filter(|module| !module.disabled)
            {
                let audio = audio_module(module, rate, self.bpm)?;
                let audio_id = patch.insert(audio);
                if module.kind == ModuleKind::Probe {
                    probes.push(ProbeRoute {
                        source: audio_id,
                        target: module.id.value(),
                    });
                }
                let input_count = self.module_input_count(module);
                if self.show_meters && input_count > 0 {
                    if let Some(route) =
                        MeterRoute::new(audio_id, module.id.value(), input_count as usize)
                    {
                        meters.push(route);
                    }
                }
                ids.push((
                    AudioKey::Subpatch {
                        owner: owner_id,
                        module: module.id,
                    },
                    audio_id,
                ));
            }
        }

        let output_gain = audio_float(output, 1)?;
        let output_id = patch.insert(AudioModule::Binary {
            op: BinaryOp::Multiply,
            a: 0.0,
            b: output_gain,
        });

        for connection in &connections {
            let from = root_connection_source(root_modules, instrument, &ids, connection);
            if connection.to == output.id {
                let Some(input) =
                    connection_input(connection, output, self.module_input_count(output))
                else {
                    continue;
                };
                if !self.module_input_connected(output, input as u16) {
                    continue;
                }
                let Some(from) = from else {
                    if input == 0 {
                        return Err(AudioPatchError::MissingOutput);
                    }
                    continue;
                };
                patch
                    .connect_input(from, output_id, input)
                    .map_err(AudioPatchError::Connect)?;
                continue;
            }
            let Some(target) = root_modules
                .iter()
                .find(|module| module.id == connection.to)
            else {
                continue;
            };
            let Some(input) = connection_input(connection, target, self.module_input_count(target))
            else {
                continue;
            };
            if !self.module_input_connected(target, input as u16) {
                continue;
            }
            if target.kind == ModuleKind::Subpatch {
                let Some(from) = from else {
                    continue;
                };
                let Some(subpatch) = instrument
                    .subpatches
                    .iter()
                    .find(|subpatch| subpatch.owner == target.id)
                else {
                    continue;
                };
                let inputs = subpatch_inputs(&subpatch.surface);
                let Some(input_id) = inputs.get(input).copied() else {
                    continue;
                };
                let Some(to) = audio_id(
                    &ids,
                    AudioKey::Subpatch {
                        owner: target.id,
                        module: input_id,
                    },
                ) else {
                    continue;
                };
                patch
                    .connect_input(from, to, 0)
                    .map_err(AudioPatchError::Connect)?;
            } else {
                let Some(from) = from else {
                    continue;
                };
                let Some(to) = audio_id(&ids, AudioKey::Root(connection.to)) else {
                    continue;
                };
                patch
                    .connect_input(from, to, input)
                    .map_err(AudioPatchError::Connect)?;
            }
        }

        for (owner, subpatch) in needed.iter().filter_map(|id| {
            instrument
                .subpatches
                .iter()
                .find(|subpatch| subpatch.owner == *id)
                .map(|subpatch| (*id, subpatch))
        }) {
            for connection in self.surface_connections(&subpatch.surface.modules) {
                let Some(from) = audio_id(
                    &ids,
                    AudioKey::Subpatch {
                        owner,
                        module: connection.from,
                    },
                ) else {
                    continue;
                };
                let Some(target) = subpatch
                    .surface
                    .modules
                    .iter()
                    .find(|module| module.id == connection.to)
                else {
                    continue;
                };
                let Some(to) = audio_id(
                    &ids,
                    AudioKey::Subpatch {
                        owner,
                        module: connection.to,
                    },
                ) else {
                    continue;
                };
                let Some(input) =
                    connection_input(&connection, target, self.module_input_count(target))
                else {
                    continue;
                };
                if !self.module_input_connected(target, input as u16) {
                    continue;
                }
                patch
                    .connect_input(from, to, input)
                    .map_err(AudioPatchError::Connect)?;
            }
        }

        patch.output(output_id).map_err(AudioPatchError::Connect)?;
        let patch = CompiledPatch::new(&patch, rate).map_err(AudioPatchError::Compile)?;
        Ok(GuiAudioPatch {
            patch,
            probes,
            meters,
            voice_mode,
        })
    }

    fn instrument(&self) -> &Instrument {
        &self.instruments[self.active_instrument]
    }

    fn instrument_mut(&mut self) -> &mut Instrument {
        &mut self.instruments[self.active_instrument]
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            instruments: self.instruments.clone(),
            active_instrument: self.active_instrument,
            next_module_id: self.next_module_id,
        }
    }

    fn commit(&mut self, before: Snapshot) {
        self.update_disabled_states();
        if before != self.snapshot() {
            self.undo.push(before);
            self.redo.clear();
            self.dirty = true;
            self.sync_audio_patch();
        }
    }

    fn update_disabled_states(&mut self) {
        for instrument in &mut self.instruments {
            let root_footprints = instrument
                .root
                .modules
                .iter()
                .map(|module| {
                    let subpatch_ports = if module.kind == ModuleKind::Subpatch {
                        Some(
                            instrument
                                .subpatches
                                .iter()
                                .find(|subpatch| subpatch.owner == module.id)
                                .map(|subpatch| {
                                    let inputs = subpatch
                                        .surface
                                        .modules
                                        .iter()
                                        .filter(|module| module.kind == ModuleKind::SubpatchInput)
                                        .count()
                                        as u16;
                                    let outputs = subpatch
                                        .surface
                                        .modules
                                        .iter()
                                        .filter(|module| module.kind == ModuleKind::SubpatchOutput)
                                        .count()
                                        as u16;
                                    (inputs, outputs)
                                })
                                .unwrap_or((0, 0)),
                        )
                    } else {
                        None
                    };
                    let (width, height) = module_footprint(module, subpatch_ports);
                    (module.id, width, height)
                })
                .collect::<Vec<_>>();
            update_surface_disabled_states(&mut instrument.root, &root_footprints);
            for subpatch in &mut instrument.subpatches {
                let footprints = subpatch
                    .surface
                    .modules
                    .iter()
                    .map(|module| {
                        let (width, height) = module_footprint(module, None);
                        (module.id, width, height)
                    })
                    .collect::<Vec<_>>();
                update_surface_disabled_states(&mut subpatch.surface, &footprints);
            }
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.instruments = snapshot.instruments;
        self.active_instrument = snapshot.active_instrument;
        self.next_module_id = snapshot.next_module_id;
        self.mode = Mode::Normal;
        self.pointer_drag = None;
        self.held_move = None;
        self.held_selection = None;
        self.update_disabled_states();
        self.sync_audio_patch();
    }

    fn sync_audio_patch(&mut self) {
        let Some(rate) = self.audio.as_ref().map(AudioHandle::sample_rate) else {
            self.audio_status = "Audio disabled".to_string();
            return;
        };
        match self.compile_gui_audio_patch(rate) {
            Ok(patch) => {
                let Some(audio) = self.audio.as_mut() else {
                    self.audio_status = "Audio disabled".to_string();
                    return;
                };
                audio.submit_with_telemetry(
                    patch.patch,
                    &patch.probes,
                    &patch.meters,
                    patch.voice_mode,
                );
                self.audio_status = "Audio ready".to_string();
            }
            Err(error) => {
                if let Some(audio) = self.audio.as_mut() {
                    audio.collect_retired();
                }
                self.audio_status = format!("Silent: {}", audio_patch_error_message(&error));
            }
        }
    }

    fn sync_audio_track(&mut self) {
        if self.audio.is_none() {
            return;
        }
        match Track::parse(self.track_text(), &scale_from_index(self.scale_index)) {
            Ok(track) => {
                let Some(audio) = self.audio.as_mut() else {
                    return;
                };
                audio.submit_track(track, self.bpm);
                self.audio_status = "Audio ready".to_string();
            }
            Err(_) => {
                self.audio_status = "Silent: invalid track".to_string();
            }
        }
    }

    fn sync_audio_playing(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.set_playing(self.playing);
        }
    }

    fn collect_audio_retired(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.collect_retired();
        }
    }

    fn path_exists(&self, path: &str) -> bool {
        self.saved_path.as_deref() == Some(path) || self.exported_path.as_deref() == Some(path)
    }

    fn commit_save_path(&mut self, path: String) {
        self.saved_path = Some(path);
        self.dirty = false;
    }

    fn set_project(&mut self, project: Project, path: String) {
        let subpatches = project
            .subpatches
            .iter()
            .map(|subpatch| (subpatch.id, subpatch))
            .collect::<HashMap<_, _>>();
        let (root, subpatch_owners) = surface_from_project_modules(&project.modules);
        let mut instrument = Instrument {
            root,
            track_text: project.track.unwrap_or_else(|| "(0/2/4/7)".to_string()),
            subpatches: Vec::new(),
            editing_subpatch: None,
            subpatch_stack: Vec::new(),
        };
        add_project_subpatches(&mut instrument, subpatch_owners, &subpatches);
        let mut instruments = vec![instrument];
        instruments.resize_with(INSTRUMENT_COUNT, Instrument::new);
        self.instruments = instruments;
        self.active_instrument = 0;
        self.bpm = project.bpm.round().clamp(1.0, u16::MAX as f32) as u16;
        self.scale_index = project.scale_idx.min(SCALE_NAMES.len().saturating_sub(1));
        self.next_module_id = self
            .instruments
            .iter()
            .flat_map(|instrument| {
                instrument.root.modules.iter().chain(
                    instrument
                        .subpatches
                        .iter()
                        .flat_map(|subpatch| subpatch.surface.modules.iter()),
                )
            })
            .map(|module| module.id.0)
            .max()
            .map(|id| id + 1)
            .unwrap_or(0);
        self.mode = Mode::Normal;
        self.palette_searching = false;
        self.palette_filter.clear();
        self.prompt_text.clear();
        self.prompt_cursor = 0;
        self.saved_path = Some(path);
        self.exported_path = None;
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
        self.pointer_drag = None;
        self.held_move = None;
        self.held_selection = None;
        self.update_disabled_states();
        self.sync_audio_patch();
        self.sync_audio_track();
    }

    pub fn apply(&mut self, action: GuiAction) {
        let playing = self.playing;
        let bpm = self.bpm;
        let scale_index = self.scale_index;
        match self.mode {
            Mode::Normal => self.apply_normal(action),
            Mode::QuitConfirm => self.apply_quit_confirm(action),
            Mode::Palette => self.apply_palette(action),
            Mode::Move { module, origin } => self.apply_move(action, module, origin),
            Mode::Copy { source } => self.apply_copy(action, source),
            Mode::Edit { module, parameter } => self.apply_edit(action, module, parameter),
            Mode::ValueInput { module, parameter } => {
                self.apply_value_input(action, module, parameter);
            }
            Mode::AdsrEdit { module, parameter } => {
                self.apply_adsr_edit(action, module, parameter);
            }
            Mode::EnvEdit {
                module,
                point,
                editing,
            } => self.apply_env_edit(action, module, point, editing),
            Mode::ProbeEdit { module } => self.apply_probe_edit(action, module),
            Mode::SampleView {
                module,
                zoom,
                offset,
            } => self.apply_sample_view(action, module, zoom, offset),
            Mode::Select { anchor } => self.apply_select(action, anchor),
            Mode::SelectMove {
                anchor,
                extent,
                origin,
            } => self.apply_select_move(action, anchor, extent, origin),
            Mode::CopySelection {
                anchor,
                extent,
                origin,
            } => self.apply_copy_selection(action, anchor, extent, origin),
            Mode::SavePrompt => self.apply_save_prompt(action),
            Mode::SaveConfirm => self.apply_save_confirm(action),
            Mode::ExportPrompt => self.apply_export_prompt(action),
            Mode::ExportConfirm => self.apply_export_confirm(action),
            Mode::TrackPrompt => self.apply_track_prompt(action),
            Mode::TrackSettings { parameter } => self.apply_track_settings(action, parameter),
        }
        if self.playing != playing {
            self.sync_audio_playing();
        }
        if self.bpm != bpm || self.scale_index != scale_index {
            self.sync_audio_track();
        }
        self.update_grid_view();
        self.collect_audio_retired();
    }

    pub(crate) fn focus_cell(&mut self, position: GridPos) {
        let x = position.x.min(self.width.saturating_sub(1));
        let y = position.y.min(self.height.saturating_sub(1));
        self.instrument_mut().surface_mut().cursor = GridPos::new(x, y);
        self.update_grid_view();
    }

    pub(crate) fn open_category(&mut self, category: ModuleCategory) {
        self.mode = Mode::Palette;
        self.palette_searching = false;
        self.palette_filter.clear();
        self.palette_filter_index = 0;
        self.palette_category = category;
        self.palette_index = 0;
    }

    pub(crate) fn choose_palette_index(&mut self, index: usize) {
        self.mode = Mode::Palette;
        self.palette_searching = false;
        self.palette_filter.clear();
        self.palette_filter_index = 0;
        self.palette_index = index.min(self.palette_modules().len().saturating_sub(1));
    }

    pub(crate) fn choose_filtered_palette_index(&mut self, index: usize) {
        self.mode = Mode::Palette;
        self.palette_searching = true;
        self.palette_filter_index =
            index.min(self.filtered_palette_modules().len().saturating_sub(1));
    }

    pub(crate) fn click_grid_cell(&mut self, position: GridPos) {
        let cursor = self.cursor();
        self.focus_cell(position);
        match self.mode {
            Mode::Palette => self.apply(GuiAction::Confirm),
            Mode::Select { anchor } => {
                let (anchor, extent) = normalized_rect(anchor, cursor);
                if position.x >= anchor.x
                    && position.x <= extent.x
                    && position.y >= anchor.y
                    && position.y <= extent.y
                {
                    self.mode = Mode::SelectMove {
                        anchor,
                        extent,
                        origin: position,
                    };
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Mode::SelectMove { anchor, extent, .. } => {
                if position.x >= anchor.x
                    && position.x <= extent.x
                    && position.y >= anchor.y
                    && position.y <= extent.y
                {
                    self.mode = Mode::SelectMove {
                        anchor,
                        extent,
                        origin: position,
                    };
                } else {
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn drag_grid_cell(&mut self, phase: GridPointerPhase, position: GridPos) {
        match phase {
            GridPointerPhase::Start => self.start_grid_drag(position),
            GridPointerPhase::Drag => self.update_grid_drag(position),
            GridPointerPhase::End => self.end_grid_drag(position),
        }
    }

    pub(crate) fn drag_env_point(
        &mut self,
        phase: EnvPointerPhase,
        module: ModuleId,
        time: f32,
        value: f32,
    ) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let point_count = self.modules()[module_index].env_points.len();
        if point_count == 0 {
            self.mode = Mode::Normal;
            return;
        }
        let target_time = (time * 100.).round().clamp(0., 100.) as i32;
        let target_value = (value * 100.).round().clamp(-100., 100.) as i32;
        let point = match (phase, self.mode) {
            (
                EnvPointerPhase::Drag | EnvPointerPhase::End,
                Mode::EnvEdit {
                    module: active,
                    point,
                    editing: true,
                },
            ) if active == module => point.min(point_count - 1),
            _ => self.modules()[module_index]
                .env_points
                .iter()
                .enumerate()
                .min_by_key(|(_, point)| {
                    let time_delta = point.time - target_time;
                    let value_delta = point.value - target_value;
                    time_delta * time_delta + value_delta * value_delta
                })
                .map(|(index, _)| index)
                .unwrap_or(0),
        };
        let before = self.snapshot();
        let (point, changed) = {
            let points = &mut self.instrument_mut().surface_mut().modules[module_index].env_points;
            let changed = points[point].time != target_time || points[point].value != target_value;
            points[point].time = target_time;
            points[point].value = target_value;
            let moved = points[point];
            points.sort_by_key(|point| point.time);
            let point = points.iter().position(|point| *point == moved).unwrap_or(0);
            (point, changed)
        };
        if changed {
            self.commit(before);
        }
        self.mode = Mode::EnvEdit {
            module,
            point,
            editing: phase != EnvPointerPhase::End,
        };
    }

    fn start_grid_drag(&mut self, position: GridPos) {
        let cursor = self.cursor();
        self.focus_cell(position);
        self.pointer_drag = match self.mode {
            Mode::Select { anchor } => {
                let (anchor, extent) = normalized_rect(anchor, cursor);
                if position.x >= anchor.x
                    && position.x <= extent.x
                    && position.y >= anchor.y
                    && position.y <= extent.y
                {
                    self.mode = Mode::SelectMove {
                        anchor,
                        extent,
                        origin: position,
                    };
                    Some(PointerDrag::SelectionMove {
                        anchor,
                        extent,
                        origin: position,
                    })
                } else {
                    self.mode = Mode::Normal;
                    None
                }
            }
            Mode::Normal => {
                let module_drag = self.module_at(position).map(|module| PointerDrag::Module {
                    module: module.id,
                    origin: module.position,
                    grab: position,
                });
                module_drag.or(Some(PointerDrag::Selection { anchor: position }))
            }
            _ => None,
        };
    }

    fn update_grid_drag(&mut self, position: GridPos) {
        let Some(drag) = self.pointer_drag else {
            return;
        };
        match drag {
            PointerDrag::Module {
                module,
                origin,
                grab,
            } => {
                if position == self.cursor() {
                    return;
                }
                self.focus_cell(position);
                let Some(found) = self.modules().iter().find(|found| found.id == module) else {
                    return;
                };
                let next = self.moved_position(found, origin, grab, position);
                let before = self.snapshot();
                if let Some(found) = self
                    .instrument_mut()
                    .surface_mut()
                    .modules
                    .iter_mut()
                    .find(|found| found.id == module)
                {
                    found.position = next;
                    self.pointer_drag = Some(PointerDrag::Module {
                        module,
                        origin,
                        grab,
                    });
                    self.commit(before);
                }
            }
            PointerDrag::Selection { anchor } => {
                self.focus_cell(position);
                if position != anchor {
                    self.mode = Mode::Select { anchor };
                }
            }
            PointerDrag::SelectionMove {
                anchor,
                extent,
                origin,
            } => {
                if position == origin {
                    return;
                }
                let Some((anchor, extent)) =
                    self.move_selection_rect(anchor, extent, origin, position)
                else {
                    return;
                };
                self.focus_cell(position);
                self.mode = Mode::SelectMove {
                    anchor,
                    extent,
                    origin: position,
                };
                self.pointer_drag = Some(PointerDrag::SelectionMove {
                    anchor,
                    extent,
                    origin: position,
                });
            }
        }
    }

    fn end_grid_drag(&mut self, position: GridPos) {
        self.update_grid_drag(position);
        if let Some(PointerDrag::Selection { anchor }) = self.pointer_drag
            && anchor == self.cursor()
        {
            self.mode = Mode::Normal;
        }
        self.pointer_drag = None;
    }

    fn move_selection_rect(
        &mut self,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
        target: GridPos,
    ) -> Option<(GridPos, GridPos)> {
        let ids = self
            .modules()
            .iter()
            .filter(|module| {
                module.position.x >= anchor.x
                    && module.position.x <= extent.x
                    && module.position.y >= anchor.y
                    && module.position.y <= extent.y
            })
            .map(|module| module.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return None;
        }
        let dx = target.x as i16 - origin.x as i16;
        let dy = target.y as i16 - origin.y as i16;
        let targets: Vec<(ModuleId, GridPos)> = self
            .modules()
            .iter()
            .filter(|module| ids.contains(&module.id))
            .map(|module| {
                let x =
                    (module.position.x as i16 + dx).clamp(0, self.width.saturating_sub(1) as i16);
                let y =
                    (module.position.y as i16 + dy).clamp(0, self.height.saturating_sub(1) as i16);
                (module.id, GridPos::new(x as u16, y as u16))
            })
            .collect();
        if targets.iter().any(|(id, position)| {
            let Some(module) = self.modules().iter().find(|module| module.id == *id) else {
                return true;
            };
            !self.module_fits(module, *position, &ids)
        }) {
            return None;
        }
        let before = self.snapshot();
        for (id, position) in targets {
            if let Some(module) = self
                .instrument_mut()
                .surface_mut()
                .modules
                .iter_mut()
                .find(|module| module.id == id)
            {
                module.position = position;
            }
        }
        self.commit(before);
        let x = (anchor.x as i16 + dx).clamp(0, self.width.saturating_sub(1) as i16);
        let y = (anchor.y as i16 + dy).clamp(0, self.height.saturating_sub(1) as i16);
        let end_x = (extent.x as i16 + dx).clamp(0, self.width.saturating_sub(1) as i16);
        let end_y = (extent.y as i16 + dy).clamp(0, self.height.saturating_sub(1) as i16);
        Some((
            GridPos::new(x as u16, y as u16),
            GridPos::new(end_x as u16, end_y as u16),
        ))
    }

    fn open_prompt(&mut self, mode: Mode, text: impl Into<String>) {
        self.prompt_text = text.into();
        self.prompt_cursor = self.prompt_text.len();
        self.prompt_input = TextState::new(&self.prompt_text);
        self.mode = mode;
    }

    fn apply_text_action(&mut self, action: GuiAction, kind: TextInputKind) {
        match action {
            GuiAction::InputChar(character) => {
                if text_input_accepts(kind, character) {
                    self.prompt_text.insert(self.prompt_cursor, character);
                    self.prompt_cursor += character.len_utf8();
                }
            }
            GuiAction::Backspace => {
                if let Some(index) = self.prev_prompt_index() {
                    self.prompt_text.drain(index..self.prompt_cursor);
                    self.prompt_cursor = index;
                }
            }
            GuiAction::DeleteChar => {
                if let Some(index) = self.next_prompt_index() {
                    self.prompt_text.drain(self.prompt_cursor..index);
                }
            }
            GuiAction::Left => {
                if let Some(index) = self.prev_prompt_index() {
                    self.prompt_cursor = index;
                }
            }
            GuiAction::Right => {
                if let Some(index) = self.next_prompt_index() {
                    self.prompt_cursor = index;
                }
            }
            GuiAction::TextStart => self.prompt_cursor = 0,
            GuiAction::TextEnd => self.prompt_cursor = self.prompt_text.len(),
            _ => {}
        }
        self.prompt_input = TextState::new(&self.prompt_text);
    }

    fn text_input_kind(&self, module: ModuleId, parameter: usize) -> TextInputKind {
        self.modules()
            .iter()
            .find(|candidate| candidate.id == module)
            .and_then(|module| module.parameters.get(parameter))
            .map(|parameter| match parameter.value {
                ParameterValue::Time { .. } => TextInputKind::Time,
                _ => TextInputKind::Number,
            })
            .unwrap_or(TextInputKind::Number)
    }

    fn prev_prompt_index(&self) -> Option<usize> {
        self.prompt_text[..self.prompt_cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
    }

    fn next_prompt_index(&self) -> Option<usize> {
        self.prompt_text[self.prompt_cursor..]
            .char_indices()
            .nth(1)
            .map(|(index, _)| self.prompt_cursor + index)
            .or_else(|| {
                (self.prompt_cursor < self.prompt_text.len()).then_some(self.prompt_text.len())
            })
    }

    fn apply_normal(&mut self, action: GuiAction) {
        match action {
            GuiAction::Quit => {
                if self.dirty {
                    self.mode = Mode::QuitConfirm;
                } else {
                    self.should_quit = true;
                }
            }
            GuiAction::Save => {
                if self.saved_path.is_some() {
                    self.dirty = false;
                } else {
                    self.open_prompt(Mode::SavePrompt, "patch.bw");
                }
            }
            GuiAction::SaveAs => {
                let text = self.saved_path.as_deref().unwrap_or("patch.bw").to_string();
                self.open_prompt(Mode::SavePrompt, text);
            }
            GuiAction::Load => {
                self.load_requested = true;
            }
            GuiAction::Export => {
                let text = self
                    .saved_path
                    .as_deref()
                    .map(|path| path.trim_end_matches(".bw").to_string() + ".wav")
                    .unwrap_or_else(|| "output.wav".to_string());
                self.open_prompt(Mode::ExportPrompt, text);
            }
            GuiAction::TrackSettings => {
                self.mode = Mode::TrackSettings { parameter: 0 };
            }
            GuiAction::TrackEdit => {
                let text = self.track_text().to_string();
                self.open_prompt(Mode::TrackPrompt, text);
            }
            GuiAction::EditSubpatch => self.toggle_subpatch(),
            GuiAction::ExitSubpatch => self.exit_subpatch(),
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::OpenPalette => {
                self.mode = Mode::Palette;
                self.palette_searching = false;
                self.palette_filter.clear();
                self.palette_filter_index = 0;
                self.palette_index = 0;
            }
            GuiAction::Palette(category) => self.open_category(category),
            GuiAction::TogglePlay => self.playing = !self.playing,
            GuiAction::ToggleMeters => {
                self.show_meters = !self.show_meters;
                self.sync_audio_patch();
            }
            GuiAction::Undo => {
                if let Some(snapshot) = self.undo.pop() {
                    self.redo.push(self.snapshot());
                    self.restore(snapshot);
                }
            }
            GuiAction::Redo => {
                if let Some(snapshot) = self.redo.pop() {
                    self.undo.push(self.snapshot());
                    self.restore(snapshot);
                }
            }
            GuiAction::Instrument(index) => {
                if index < self.instruments.len() {
                    self.active_instrument = index;
                    self.mode = Mode::Normal;
                    self.sync_audio_patch();
                }
            }
            GuiAction::Delete => {
                if let Some(module) = self.module_at(self.cursor()) {
                    let before = self.snapshot();
                    let id = module.id;
                    self.instrument_mut()
                        .surface_mut()
                        .modules
                        .retain(|module| module.id != id);
                    self.remove_subpatch_surface(id);
                    self.commit(before);
                }
            }
            GuiAction::Edit => {
                if let Some(module) = self.module_at(self.cursor())
                    && !module.parameters.is_empty()
                {
                    self.mode = Mode::Edit {
                        module: module.id,
                        parameter: 0,
                    };
                }
            }
            GuiAction::Move => {
                if let Some(module) = self.module_at(self.cursor()) {
                    self.mode = Mode::Move {
                        module: module.id,
                        origin: self.cursor(),
                    };
                }
            }
            GuiAction::Copy => {
                if let Some(module) = self.module_at(self.cursor()) {
                    self.mode = Mode::Copy { source: module.id };
                }
            }
            GuiAction::Select => {
                self.mode = Mode::Select {
                    anchor: self.cursor(),
                };
            }
            GuiAction::Rotate => {
                let cursor = self.cursor();
                if let Some(module) = self.module_at(cursor) {
                    let orientation = match module.orientation {
                        Orientation::Right => Orientation::Down,
                        Orientation::Down => Orientation::Right,
                    };
                    if self.area_fits(
                        module.kind.width(orientation),
                        module.kind.height(orientation),
                        module.position,
                        &[module.id],
                    ) {
                        let id = module.id;
                        let before = self.snapshot();
                        if let Some(module) = self
                            .instrument_mut()
                            .surface_mut()
                            .modules
                            .iter_mut()
                            .find(|module| module.id == id)
                        {
                            module.orientation = orientation;
                            self.commit(before);
                        }
                    }
                }
            }
            GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Confirm
            | GuiAction::Cancel
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::Search
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_quit_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm | GuiAction::Copy => self.should_quit = true,
            GuiAction::Cancel | GuiAction::OpenPalette => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn toggle_subpatch(&mut self) {
        let target = self
            .module_at(self.cursor())
            .filter(|module| module.kind == ModuleKind::Subpatch)
            .map(|module| module.id);
        if let Some(target) = target {
            self.enter_subpatch_surface(target);
            self.mode = Mode::Normal;
        } else {
            self.exit_subpatch_surface();
            self.mode = Mode::Normal;
        }
    }

    fn enter_subpatch_surface(&mut self, owner: ModuleId) {
        let cursor = self.cursor();
        let inst = self.instrument_mut();
        inst.ensure_subpatch(owner);
        inst.subpatch_stack.push((inst.editing_subpatch, cursor));
        inst.editing_subpatch = Some(owner);
        inst.surface_mut().cursor = GridPos::new(0, 0);
    }

    fn exit_subpatch_surface(&mut self) {
        let inst = self.instrument_mut();
        if inst.editing_subpatch.is_none() {
            return;
        }
        if let Some((parent, cursor)) = inst.subpatch_stack.pop() {
            inst.editing_subpatch = parent;
            inst.surface_mut().cursor = cursor;
        } else {
            inst.editing_subpatch = None;
        }
    }

    fn exit_subpatch(&mut self) {
        self.exit_subpatch_surface();
        self.mode = Mode::Normal;
    }

    fn apply_palette(&mut self, action: GuiAction) {
        if self.palette_searching {
            self.apply_palette_search(action);
            return;
        }

        match action {
            GuiAction::PaletteLeft | GuiAction::Left => self.move_palette_category(-1),
            GuiAction::PaletteRight | GuiAction::Right => self.move_palette_category(1),
            GuiAction::Palette(category) => self.open_category(category),
            GuiAction::PaletteUp | GuiAction::Up => self.move_palette_selection(-1),
            GuiAction::PaletteDown | GuiAction::Down => self.move_palette_selection(1),
            GuiAction::Confirm | GuiAction::OpenPalette => {
                let kind = self.selected_palette_module();
                self.insert_at_cursor(kind);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Search => {
                self.palette_filter.clear();
                self.palette_filter_index = 0;
                self.palette_searching = true;
            }
            GuiAction::LeftFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::RightFast
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::Delete
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_palette_search(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => {
                self.palette_searching = false;
                self.palette_filter.clear();
                self.palette_filter_index = 0;
            }
            GuiAction::Backspace => {
                self.palette_filter.pop();
                self.palette_filter_index = 0;
            }
            GuiAction::Down => {
                if self.palette_filter_index + 1 < self.filtered_palette_modules().len() {
                    self.palette_filter_index += 1;
                }
            }
            GuiAction::Up => {
                self.palette_filter_index = self.palette_filter_index.saturating_sub(1);
            }
            GuiAction::Confirm => {
                if let Some(kind) = self.selected_filtered_palette_module() {
                    self.insert_at_cursor(kind);
                }
                self.palette_searching = false;
                self.palette_filter.clear();
                self.palette_filter_index = 0;
                self.mode = Mode::Normal;
            }
            GuiAction::InputChar(character) => {
                self.palette_filter.push(character);
                self.palette_filter_index = 0;
            }
            _ => {}
        }
    }

    fn apply_move(&mut self, action: GuiAction, module: ModuleId, origin: GridPos) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let cursor = self.cursor();
                if let Some(mut held) = self.held_move.take() {
                    let before = held.before.clone();
                    held.module.position = self
                        .bounded_module_position(&held.module, cursor.x as i16, cursor.y as i16)
                        .unwrap_or(cursor);
                    let kind = held.module.kind;
                    self.instrument_mut()
                        .surface_mut()
                        .modules
                        .push(held.module);
                    if kind == ModuleKind::Subpatch {
                        let id = self
                            .modules()
                            .iter()
                            .find(|module| module.position == cursor)
                            .map(|module| module.id);
                        if let Some(id) = id {
                            self.instrument_mut().ensure_subpatch(id);
                        }
                    }
                    self.commit(before);
                    self.mode = Mode::Normal;
                    return;
                }
                let Some(found) = self.modules().iter().find(|found| found.id == module) else {
                    self.mode = Mode::Normal;
                    return;
                };
                let next = self.moved_position(found, found.position, origin, cursor);
                let before = self.snapshot();
                if let Some(found) = self
                    .instrument_mut()
                    .surface_mut()
                    .modules
                    .iter_mut()
                    .find(|found| found.id == module)
                {
                    found.position = next;
                    self.commit(before);
                }
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => {
                if let Some(mut held) = self.held_move.take() {
                    let origin = held.origin;
                    held.module.position = origin;
                    let inst = self.instrument_mut();
                    inst.editing_subpatch = held.origin_surface;
                    inst.subpatch_stack = held.origin_stack;
                    inst.surface_mut().cursor = origin;
                    inst.surface_mut().modules.push(held.module);
                    self.mode = Mode::Normal;
                    return;
                }
                self.instrument_mut().surface_mut().cursor = origin;
                self.mode = Mode::Normal;
            }
            GuiAction::EditSubpatch => self.move_across_subpatch(module, origin),
            GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::ExitSubpatch
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::OpenPalette
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn move_across_subpatch(&mut self, module: ModuleId, origin: GridPos) {
        let target = self
            .module_at(self.cursor())
            .filter(|candidate| candidate.kind == ModuleKind::Subpatch && candidate.id != module)
            .map(|candidate| candidate.id);
        let can_exit = self.instrument().editing_subpatch.is_some();
        if target.is_none() && !can_exit {
            return;
        }

        if self.held_move.is_none() {
            let before = self.snapshot();
            let origin_surface = self.instrument().editing_subpatch;
            let origin_stack = self.instrument().subpatch_stack.clone();
            let Some(index) = self
                .instrument()
                .surface()
                .modules
                .iter()
                .position(|candidate| candidate.id == module)
            else {
                self.mode = Mode::Normal;
                return;
            };
            let module_value = self.instrument_mut().surface_mut().modules.remove(index);
            self.held_move = Some(HeldMove {
                module: module_value,
                origin_surface,
                origin_stack,
                origin,
                before,
            });
        }

        if let Some(target) = target {
            self.enter_subpatch_surface(target);
        } else {
            self.exit_subpatch_surface();
        }
        self.mode = Mode::Move { module, origin };
    }

    fn apply_copy(&mut self, action: GuiAction, source: ModuleId) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let cursor = self.cursor();
                let Some(module) = self.modules().iter().find(|module| module.id == source) else {
                    self.mode = Mode::Normal;
                    return;
                };
                let before = self.snapshot();
                let kind = module.kind;
                let orientation = module.orientation;
                let parameters = module.parameters.clone();
                let env_points = module.env_points.clone();
                let position = self
                    .bounded_module_position(module, cursor.x as i16, cursor.y as i16)
                    .unwrap_or(cursor);
                let id = ModuleId(self.next_module_id);
                self.instrument_mut().surface_mut().modules.push(Module {
                    id,
                    kind,
                    position,
                    orientation,
                    parameters,
                    env_points,
                    disabled: false,
                });
                if kind == ModuleKind::Subpatch {
                    self.instrument_mut().ensure_subpatch(id);
                }
                self.next_module_id += 1;
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_edit(&mut self, action: GuiAction, module: ModuleId, parameter: usize) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let module_kind = self.modules()[module_index].kind;
        let parameter_count = self.modules()[module_index].parameters.len();
        let special = module_kind.special_editor();
        let total = parameter_count + usize::from(special.is_some());
        if total == 0 {
            self.mode = Mode::Normal;
            return;
        }
        let parameter = parameter.min(total - 1);

        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                if parameter == parameter_count {
                    match special {
                        Some(SpecialEditor::Adsr) => {
                            self.mode = Mode::AdsrEdit {
                                module,
                                parameter: 0,
                            };
                        }
                        Some(SpecialEditor::Envelope) => {
                            self.mode = Mode::EnvEdit {
                                module,
                                point: 0,
                                editing: false,
                            };
                        }
                        Some(SpecialEditor::Probe) => self.mode = Mode::ProbeEdit { module },
                        Some(SpecialEditor::Sample) => {
                            self.mode = Mode::SampleView {
                                module,
                                zoom: 1,
                                offset: 0,
                            };
                        }
                        None => {}
                    }
                }
            }
            GuiAction::Down => {
                self.mode = Mode::Edit {
                    module,
                    parameter: (parameter + 1) % total,
                };
            }
            GuiAction::Up => {
                self.mode = Mode::Edit {
                    module,
                    parameter: if parameter == 0 {
                        total - 1
                    } else {
                        parameter - 1
                    },
                };
            }
            GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast => {
                if parameter >= parameter_count {
                    return;
                }
                let before = self.snapshot();
                let up = matches!(action, GuiAction::ValueUp | GuiAction::ValueUpFast);
                let fast = matches!(action, GuiAction::ValueDownFast | GuiAction::ValueUpFast);
                let step_scale = self.step_scale();
                let changed = {
                    let parameter = &mut self.instrument_mut().surface_mut().modules[module_index]
                        .parameters[parameter];
                    let changed = parameter.value.adjust(up, fast, step_scale);
                    if changed
                        && matches!(
                            parameter.value,
                            ParameterValue::Float { .. } | ParameterValue::Time { .. }
                        )
                    {
                        parameter.connected = false;
                    }
                    changed
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::TogglePort => {
                if parameter >= parameter_count {
                    return;
                }
                let before = self.snapshot();
                let changed = {
                    let parameter = &mut self.instrument_mut().surface_mut().modules[module_index]
                        .parameters[parameter];
                    if matches!(
                        parameter.value,
                        ParameterValue::Float { .. } | ParameterValue::Time { .. }
                    ) {
                        parameter.connected = !parameter.connected;
                        true
                    } else {
                        false
                    }
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::CycleUnit => {
                if parameter >= parameter_count {
                    return;
                }
                let before = self.snapshot();
                let changed = self.instrument_mut().surface_mut().modules[module_index].parameters
                    [parameter]
                    .value
                    .cycle_unit();
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::CycleStep => self.step_size = (self.step_size + 1) % 5,
            GuiAction::TypeValue => {
                if parameter < parameter_count
                    && self.modules()[module_index].parameters[parameter]
                        .value
                        .accepts_typed_value()
                {
                    self.prompt_text = self.modules()[module_index].parameters[parameter]
                        .value
                        .input_text();
                    self.prompt_cursor = self.prompt_text.len();
                    self.mode = Mode::ValueInput { module, parameter };
                }
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            GuiAction::Left => self.apply_edit(GuiAction::ValueDown, module, parameter),
            GuiAction::Right => self.apply_edit(GuiAction::ValueUp, module, parameter),
            GuiAction::LeftFast => self.apply_edit(GuiAction::ValueDownFast, module, parameter),
            GuiAction::RightFast => self.apply_edit(GuiAction::ValueUpFast, module, parameter),
            GuiAction::DownFast | GuiAction::UpFast => {}
            GuiAction::OpenPalette
            | GuiAction::ToggleMeters
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Delete
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_value_input(&mut self, action: GuiAction, module: ModuleId, parameter: usize) {
        match action {
            GuiAction::Cancel => {
                self.mode = Mode::Edit { module, parameter };
            }
            GuiAction::Confirm => {
                let before = self.snapshot();
                let changed = self.set_typed_value(module, parameter);
                if changed {
                    self.commit(before);
                }
                self.mode = Mode::Edit { module, parameter };
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => {
                self.apply_text_action(action, self.text_input_kind(module, parameter));
            }
            _ => {}
        }
    }

    fn apply_save_prompt(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                let path = self.prompt_text.trim();
                if path.is_empty() {
                    self.mode = Mode::Normal;
                    return;
                }
                let is_current = self.saved_path.as_deref() == Some(path);
                if !is_current && self.path_exists(path) {
                    self.mode = Mode::SaveConfirm;
                } else {
                    self.commit_save_path(path.to_string());
                    self.mode = Mode::Normal;
                }
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => self.apply_text_action(action, TextInputKind::Free),
            _ => {}
        }
    }

    fn apply_save_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm | GuiAction::Copy => {
                let path = self.prompt_text.trim();
                if !path.is_empty() {
                    self.commit_save_path(path.to_string());
                }
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::OpenPalette => self.mode = Mode::SavePrompt,
            _ => {}
        }
    }

    fn apply_export_prompt(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                let path = self.prompt_text.trim();
                if path.is_empty() {
                    self.mode = Mode::Normal;
                    return;
                }
                if self.path_exists(path) {
                    self.mode = Mode::ExportConfirm;
                } else {
                    self.exported_path = Some(path.to_string());
                    self.mode = Mode::Normal;
                }
            }
            GuiAction::Up => {
                self.export_loops = (self.export_loops + 1).min(100);
            }
            GuiAction::Down => {
                self.export_loops = self.export_loops.saturating_sub(1).max(1);
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => self.apply_text_action(action, TextInputKind::Free),
            _ => {}
        }
    }

    fn apply_export_confirm(&mut self, action: GuiAction) {
        match action {
            GuiAction::Confirm | GuiAction::Copy => {
                let path = self.prompt_text.trim();
                if !path.is_empty() {
                    self.exported_path = Some(path.to_string());
                }
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::OpenPalette => self.mode = Mode::ExportPrompt,
            _ => {}
        }
    }

    fn apply_track_prompt(&mut self, action: GuiAction) {
        match action {
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::Confirm => {
                let before = self.snapshot();
                let text = self.prompt_text.trim().to_string();
                self.instrument_mut().track_text = text;
                self.commit(before);
                self.sync_audio_track();
                self.mode = Mode::Normal;
            }
            GuiAction::Left
            | GuiAction::Right
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd => self.apply_text_action(action, TextInputKind::Free),
            GuiAction::Up
            | GuiAction::Down
            | GuiAction::LeftFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::RightFast
            | GuiAction::OpenPalette
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_track_settings(&mut self, action: GuiAction, parameter: usize) {
        let parameter = parameter.min(2);
        match action {
            GuiAction::Cancel | GuiAction::TrackSettings | GuiAction::Confirm => {
                self.mode = Mode::Normal;
            }
            GuiAction::Down => {
                self.mode = Mode::TrackSettings {
                    parameter: (parameter + 1) % 3,
                };
            }
            GuiAction::Up => {
                self.mode = Mode::TrackSettings {
                    parameter: if parameter == 0 { 2 } else { parameter - 1 },
                };
            }
            GuiAction::ValueUp | GuiAction::Right => match parameter {
                0 => self.bpm = (self.bpm + 5).min(300),
                1 => self.scale_index = (self.scale_index + 1) % SCALE_NAMES.len(),
                2 => self.probe_voice = (self.probe_voice + 1) % NUM_VOICES,
                _ => {}
            },
            GuiAction::ValueDown | GuiAction::Left => match parameter {
                0 => self.bpm = self.bpm.saturating_sub(5).max(20),
                1 => {
                    self.scale_index = if self.scale_index == 0 {
                        SCALE_NAMES.len() - 1
                    } else {
                        self.scale_index - 1
                    };
                }
                2 => {
                    self.probe_voice = if self.probe_voice == 0 {
                        NUM_VOICES - 1
                    } else {
                        self.probe_voice - 1
                    };
                }
                _ => {}
            },
            GuiAction::ValueUpFast => {
                if parameter == 0 {
                    self.bpm = (self.bpm + 20).min(300);
                }
            }
            GuiAction::ValueDownFast => {
                if parameter == 0 {
                    self.bpm = self.bpm.saturating_sub(20).max(20);
                }
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn set_typed_value(&mut self, module: ModuleId, parameter: usize) -> bool {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            return false;
        };
        let Some(parameter_value) = self.instrument().surface().modules[module_index]
            .parameters
            .get(parameter)
            .map(|parameter| parameter.value.clone())
        else {
            return false;
        };
        let Some(value) = typed_parameter_value(&parameter_value, self.prompt_text.trim()) else {
            return false;
        };
        let parameter =
            &mut self.instrument_mut().surface_mut().modules[module_index].parameters[parameter];
        if parameter.value == value && !parameter.connected {
            return false;
        }
        parameter.value = value;
        parameter.connected = false;
        true
    }

    fn apply_adsr_edit(&mut self, action: GuiAction, module: ModuleId, parameter: usize) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let parameter = parameter.min(1);
        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Down => {
                self.mode = Mode::AdsrEdit {
                    module,
                    parameter: (parameter + 1) % 2,
                };
            }
            GuiAction::Up => {
                self.mode = Mode::AdsrEdit {
                    module,
                    parameter: if parameter == 0 { 1 } else { 0 },
                };
            }
            GuiAction::ValueUp
            | GuiAction::ValueDown
            | GuiAction::ValueUpFast
            | GuiAction::ValueDownFast => {
                let before = self.snapshot();
                let param_index = parameter + 2;
                let value = &mut self.instrument_mut().surface_mut().modules[module_index]
                    .parameters[param_index]
                    .value;
                let changed = match (value, action) {
                    (ParameterValue::Float { value, max, .. }, GuiAction::ValueUp) => {
                        let next = (*value + 5).min(*max);
                        let changed = next != *value;
                        *value = next;
                        changed
                    }
                    (ParameterValue::Float { value, min, .. }, GuiAction::ValueDown) => {
                        let next = (*value - 5).max(*min);
                        let changed = next != *value;
                        *value = next;
                        changed
                    }
                    (ParameterValue::Float { value, max, .. }, GuiAction::ValueUpFast) => {
                        let changed = *value != *max;
                        *value = *max;
                        changed
                    }
                    (ParameterValue::Float { value, min, .. }, GuiAction::ValueDownFast) => {
                        let changed = *value != *min;
                        *value = *min;
                        changed
                    }
                    _ => false,
                };
                if changed {
                    self.commit(before);
                }
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            GuiAction::OpenPalette
            | GuiAction::ToggleMeters
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::Confirm
            | GuiAction::Delete
            | GuiAction::Move
            | GuiAction::Copy
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::Left
            | GuiAction::Right
            | GuiAction::LeftFast
            | GuiAction::RightFast
            | GuiAction::DownFast
            | GuiAction::UpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_env_edit(&mut self, action: GuiAction, module: ModuleId, point: usize, editing: bool) {
        let Some(module_index) = self
            .modules()
            .iter()
            .position(|candidate| candidate.id == module)
        else {
            self.mode = Mode::Normal;
            return;
        };
        let point_count = self.modules()[module_index].env_points.len();
        if point_count == 0 {
            self.mode = Mode::Normal;
            return;
        }
        let point = point.min(point_count - 1);

        if editing {
            match action {
                GuiAction::Cancel | GuiAction::Confirm | GuiAction::Move => {
                    self.mode = Mode::EnvEdit {
                        module,
                        point,
                        editing: false,
                    };
                }
                GuiAction::Left
                | GuiAction::Right
                | GuiAction::LeftFast
                | GuiAction::RightFast
                | GuiAction::Up
                | GuiAction::Down
                | GuiAction::UpFast
                | GuiAction::DownFast => {
                    let before = self.snapshot();
                    let delta = self.step_scale();
                    let moved_point = {
                        let points = &mut self.instrument_mut().surface_mut().modules[module_index]
                            .env_points;
                        match action {
                            GuiAction::Left => points[point].time -= delta,
                            GuiAction::Right => points[point].time += delta,
                            GuiAction::LeftFast => points[point].time -= delta * 10,
                            GuiAction::RightFast => points[point].time += delta * 10,
                            GuiAction::Up => points[point].value += delta,
                            GuiAction::Down => points[point].value -= delta,
                            GuiAction::UpFast => points[point].value = 100,
                            GuiAction::DownFast => points[point].value = -100,
                            _ => {}
                        }
                        points[point].time = points[point].time.clamp(0, 100);
                        points[point].value = points[point].value.clamp(-100, 100);
                        let target_time = points[point].time;
                        points.sort_by_key(|point| point.time);
                        points
                            .iter()
                            .position(|point| point.time == target_time)
                            .unwrap_or(0)
                    };
                    self.commit(before);
                    self.mode = Mode::EnvEdit {
                        module,
                        point: moved_point,
                        editing: true,
                    };
                }
                GuiAction::CycleStep => self.step_size = (self.step_size + 1) % 5,
                GuiAction::TogglePlay => self.playing = !self.playing,
                _ => {}
            }
            return;
        }

        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::Right => {
                self.mode = Mode::EnvEdit {
                    module,
                    point: (point + 1) % point_count,
                    editing: false,
                };
            }
            GuiAction::Left => {
                self.mode = Mode::EnvEdit {
                    module,
                    point: if point == 0 {
                        point_count - 1
                    } else {
                        point - 1
                    },
                    editing: false,
                };
            }
            GuiAction::Move => {
                self.mode = Mode::EnvEdit {
                    module,
                    point,
                    editing: true,
                };
            }
            GuiAction::ToggleCurve => {
                let before = self.snapshot();
                let curve = &mut self.instrument_mut().surface_mut().modules[module_index]
                    .env_points[point]
                    .curve;
                *curve = !*curve;
                self.commit(before);
            }
            GuiAction::AddPoint => {
                let before = self.snapshot();
                let new_index = {
                    let points =
                        &mut self.instrument_mut().surface_mut().modules[module_index].env_points;
                    let current = points[point];
                    let next_time = points.get(point + 1).map(|next| next.time).unwrap_or(100);
                    let time = ((current.time + next_time) / 2).clamp(0, 100);
                    points.push(EnvPoint {
                        time,
                        value: current.value,
                        curve: false,
                    });
                    points.sort_by_key(|point| point.time);
                    points
                        .iter()
                        .position(|point| point.time == time)
                        .unwrap_or(point)
                };
                self.commit(before);
                self.mode = Mode::EnvEdit {
                    module,
                    point: new_index,
                    editing: false,
                };
            }
            GuiAction::DeletePoint => {
                if point_count > 2 {
                    let before = self.snapshot();
                    self.instrument_mut().surface_mut().modules[module_index]
                        .env_points
                        .remove(point);
                    self.commit(before);
                    self.mode = Mode::EnvEdit {
                        module,
                        point: point.min(point_count - 2),
                        editing: false,
                    };
                }
            }
            GuiAction::CycleStep => self.step_size = (self.step_size + 1) % 5,
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn apply_probe_edit(&mut self, action: GuiAction, _module: ModuleId) {
        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::ValueUp | GuiAction::Right | GuiAction::Up => {
                self.probe_len = (self.probe_len * 2).min(44_100 * 2);
            }
            GuiAction::ValueDown | GuiAction::Left | GuiAction::Down => {
                self.probe_len = (self.probe_len / 2).max(8);
            }
            GuiAction::ValueUpFast => {
                self.probe_len = (self.probe_len * 4).min(44_100 * 2);
            }
            GuiAction::ValueDownFast => {
                self.probe_len = (self.probe_len / 4).max(8);
            }
            GuiAction::Delete => self.probe_len = 4410,
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn apply_sample_view(&mut self, action: GuiAction, module: ModuleId, zoom: u16, offset: u16) {
        match action {
            GuiAction::Cancel | GuiAction::Edit => self.mode = Mode::Normal,
            GuiAction::ValueUp | GuiAction::Up => {
                let zoom = (zoom * 2).min(64);
                let max_offset = 100_u16.saturating_sub(100 / zoom);
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: offset.min(max_offset),
                };
            }
            GuiAction::ValueDown | GuiAction::Down => {
                let zoom = (zoom / 2).max(1);
                let max_offset = 100_u16.saturating_sub(100 / zoom);
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: offset.min(max_offset),
                };
            }
            GuiAction::Left => {
                let step = (10 / zoom.max(1)).max(1);
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: offset.saturating_sub(step),
                };
            }
            GuiAction::Right => {
                let step = (10 / zoom.max(1)).max(1);
                let max_offset = 100_u16.saturating_sub(100 / zoom.max(1));
                self.mode = Mode::SampleView {
                    module,
                    zoom,
                    offset: (offset + step).min(max_offset),
                };
            }
            GuiAction::Delete => {
                self.mode = Mode::SampleView {
                    module,
                    zoom: 1,
                    offset: 0,
                };
            }
            GuiAction::TogglePlay => self.playing = !self.playing,
            _ => {}
        }
    }

    fn apply_select(&mut self, action: GuiAction, anchor: GridPos) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Move | GuiAction::Confirm => {
                self.mode = Mode::SelectMove {
                    anchor,
                    extent: self.cursor(),
                    origin: self.cursor(),
                };
            }
            GuiAction::Copy => {
                if !self.selected_modules().is_empty() {
                    self.mode = Mode::CopySelection {
                        anchor,
                        extent: self.cursor(),
                        origin: self.cursor(),
                    };
                }
            }
            GuiAction::Delete => {
                let ids = self.selected_modules();
                let before = self.snapshot();
                self.instrument_mut()
                    .surface_mut()
                    .modules
                    .retain(|module| !ids.contains(&module.id));
                for id in ids {
                    self.remove_subpatch_surface(id);
                }
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel | GuiAction::Select => self.mode = Mode::Normal,
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {}
        }
    }

    fn apply_select_move(
        &mut self,
        action: GuiAction,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    ) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let cursor = self.cursor();
                let dx = cursor.x as i16 - origin.x as i16;
                let dy = cursor.y as i16 - origin.y as i16;
                if let Some(mut held) = self.held_selection.take() {
                    let before = held.before.clone();
                    let targets = held
                        .modules
                        .iter()
                        .map(|module| {
                            self.bounded_module_position(
                                module,
                                module.position.x as i16 + dx,
                                module.position.y as i16 + dy,
                            )
                            .unwrap_or(module.position)
                        })
                        .collect::<Vec<_>>();
                    for (module, position) in held.modules.iter_mut().zip(targets) {
                        module.position = position;
                    }
                    self.instrument_mut()
                        .surface_mut()
                        .modules
                        .extend(held.modules);
                    self.commit(before);
                    self.mode = Mode::Normal;
                    return;
                }
                let ids = self.selected_modules();
                let targets: Vec<(ModuleId, GridPos)> = self
                    .modules()
                    .iter()
                    .filter(|module| ids.contains(&module.id))
                    .map(|module| {
                        (
                            module.id,
                            self.bounded_module_position(
                                module,
                                module.position.x as i16 + dx,
                                module.position.y as i16 + dy,
                            )
                            .unwrap_or(module.position),
                        )
                    })
                    .collect();
                let before = self.snapshot();
                for (id, position) in targets {
                    if let Some(module) = self
                        .instrument_mut()
                        .surface_mut()
                        .modules
                        .iter_mut()
                        .find(|module| module.id == id)
                    {
                        module.position = position;
                    }
                }
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => {
                if let Some(held) = self.held_selection.take() {
                    let inst = self.instrument_mut();
                    inst.editing_subpatch = held.origin_surface;
                    inst.subpatch_stack = held.origin_stack;
                    inst.surface_mut().cursor = held.origin;
                    inst.surface_mut().modules.extend(held.modules);
                    self.mode = Mode::Normal;
                    return;
                }
                self.instrument_mut().surface_mut().cursor = origin;
                self.mode = Mode::Normal;
            }
            GuiAction::EditSubpatch => {
                self.move_selection_across_subpatch(anchor, extent, origin);
            }
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::ExitSubpatch
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {
                self.mode = Mode::SelectMove {
                    anchor,
                    extent,
                    origin,
                };
            }
        }
    }

    fn move_selection_across_subpatch(
        &mut self,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    ) {
        let selected = self
            .held_selection
            .as_ref()
            .map(|held| {
                held.modules
                    .iter()
                    .map(|module| module.id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                self.modules()
                    .iter()
                    .filter(|module| {
                        module.position.x >= anchor.x
                            && module.position.x <= extent.x
                            && module.position.y >= anchor.y
                            && module.position.y <= extent.y
                    })
                    .map(|module| module.id)
                    .collect()
            });
        let target = self
            .module_at(self.cursor())
            .filter(|candidate| {
                candidate.kind == ModuleKind::Subpatch && !selected.contains(&candidate.id)
            })
            .map(|candidate| candidate.id);
        let can_exit = self.instrument().editing_subpatch.is_some();
        if target.is_none() && !can_exit {
            return;
        }

        if self.held_selection.is_none() {
            let before = self.snapshot();
            let origin_surface = self.instrument().editing_subpatch;
            let origin_stack = self.instrument().subpatch_stack.clone();
            let mut modules = Vec::new();
            let surface = self.instrument_mut().surface_mut();
            let mut index = 0;
            while index < surface.modules.len() {
                if selected.contains(&surface.modules[index].id) {
                    modules.push(surface.modules.remove(index));
                } else {
                    index += 1;
                }
            }
            if modules.is_empty() {
                self.mode = Mode::Normal;
                return;
            }
            self.held_selection = Some(HeldSelection {
                modules,
                origin_surface,
                origin_stack,
                origin,
                before,
            });
        }

        if let Some(target) = target {
            self.enter_subpatch_surface(target);
        } else {
            self.exit_subpatch_surface();
        }
        self.mode = Mode::SelectMove {
            anchor,
            extent,
            origin,
        };
    }

    fn apply_copy_selection(
        &mut self,
        action: GuiAction,
        anchor: GridPos,
        extent: GridPos,
        origin: GridPos,
    ) {
        match action {
            GuiAction::Left => self.move_cursor(-1, 0),
            GuiAction::Down => self.move_cursor(0, 1),
            GuiAction::Up => self.move_cursor(0, -1),
            GuiAction::Right => self.move_cursor(1, 0),
            GuiAction::LeftFast => self.move_cursor(-4, 0),
            GuiAction::DownFast => self.move_cursor(0, 4),
            GuiAction::UpFast => self.move_cursor(0, -4),
            GuiAction::RightFast => self.move_cursor(4, 0),
            GuiAction::Confirm | GuiAction::Move | GuiAction::Copy => {
                let ids = self.selected_modules();
                let cursor = self.cursor();
                let dx = cursor.x as i16 - origin.x as i16;
                let dy = cursor.y as i16 - origin.y as i16;
                let copies: Vec<(
                    ModuleKind,
                    Orientation,
                    GridPos,
                    Vec<ModuleParameter>,
                    Vec<EnvPoint>,
                )> = self
                    .modules()
                    .iter()
                    .filter(|module| ids.contains(&module.id))
                    .map(|module| {
                        let x = (module.position.x as i16 + dx)
                            .clamp(0, self.width.saturating_sub(1) as i16);
                        let y = (module.position.y as i16 + dy)
                            .clamp(0, self.height.saturating_sub(1) as i16);
                        (
                            module.kind,
                            module.orientation,
                            GridPos::new(x as u16, y as u16),
                            module.parameters.clone(),
                            module.env_points.clone(),
                        )
                    })
                    .collect();
                let before = self.snapshot();
                for (kind, orientation, position, parameters, env_points) in copies {
                    let id = ModuleId(self.next_module_id);
                    self.instrument_mut().surface_mut().modules.push(Module {
                        id,
                        kind,
                        position,
                        orientation,
                        parameters,
                        env_points,
                        disabled: false,
                    });
                    if kind == ModuleKind::Subpatch {
                        self.instrument_mut().ensure_subpatch(id);
                    }
                    self.next_module_id += 1;
                }
                self.commit(before);
                self.mode = Mode::Normal;
            }
            GuiAction::Cancel => self.mode = Mode::Normal,
            GuiAction::OpenPalette
            | GuiAction::PaletteLeft
            | GuiAction::PaletteRight
            | GuiAction::PaletteUp
            | GuiAction::PaletteDown
            | GuiAction::Palette(_)
            | GuiAction::Quit
            | GuiAction::Save
            | GuiAction::SaveAs
            | GuiAction::Load
            | GuiAction::Export
            | GuiAction::TrackSettings
            | GuiAction::TrackEdit
            | GuiAction::Search
            | GuiAction::EditSubpatch
            | GuiAction::ExitSubpatch
            | GuiAction::TogglePlay
            | GuiAction::ToggleMeters
            | GuiAction::Delete
            | GuiAction::Edit
            | GuiAction::Rotate
            | GuiAction::Select
            | GuiAction::Undo
            | GuiAction::Redo
            | GuiAction::Instrument(_)
            | GuiAction::ValueDown
            | GuiAction::ValueUp
            | GuiAction::ValueDownFast
            | GuiAction::ValueUpFast
            | GuiAction::TogglePort
            | GuiAction::CycleUnit
            | GuiAction::CycleStep
            | GuiAction::TypeValue
            | GuiAction::InputChar(_)
            | GuiAction::Backspace
            | GuiAction::DeleteChar
            | GuiAction::TextStart
            | GuiAction::TextEnd
            | GuiAction::AddPoint
            | GuiAction::DeletePoint
            | GuiAction::ToggleCurve => {
                self.mode = Mode::CopySelection {
                    anchor,
                    extent,
                    origin,
                };
            }
        }
    }

    fn step_scale(&self) -> i32 {
        match self.step_size {
            0 => 1,
            1 => 2,
            2 => 5,
            3 => 10,
            _ => 25,
        }
    }

    fn move_cursor(&mut self, dx: i16, dy: i16) {
        let cursor = self.cursor();
        let x = (cursor.x as i16 + dx).clamp(0, self.width.saturating_sub(1) as i16);
        let y = (cursor.y as i16 + dy).clamp(0, self.height.saturating_sub(1) as i16);
        self.instrument_mut().surface_mut().cursor = GridPos::new(x as u16, y as u16);
    }

    fn update_grid_view(&mut self) {
        let (min, max) = self.active_grid_rect();
        self.grid_view.x = updated_axis_view(
            self.grid_view.x,
            min.x,
            max.x,
            self.width,
            self.grid_view_size.columns(),
        );
        self.grid_view.y = updated_axis_view(
            self.grid_view.y,
            min.y,
            max.y,
            self.height,
            self.grid_view_size.rows(),
        );
    }

    fn active_grid_rect(&self) -> (GridPos, GridPos) {
        match self.mode {
            Mode::Move { module, origin } => {
                let Some(module) = self.moving_module(module) else {
                    let cursor = self.cursor();
                    return (cursor, cursor);
                };
                let width = self.module_width(module).saturating_sub(1);
                let height = self.module_height(module).saturating_sub(1);
                let dx = bounded_grid_delta(
                    module.position.x,
                    module.position.x + width,
                    self.cursor().x as i16 - origin.x as i16,
                    self.width,
                );
                let dy = bounded_grid_delta(
                    module.position.y,
                    module.position.y + height,
                    self.cursor().y as i16 - origin.y as i16,
                    self.height,
                );
                let min_x = (module.position.x as i16 + dx).max(0) as u16;
                let min_y = (module.position.y as i16 + dy).max(0) as u16;
                let max_x = (module.position.x + width) as i16 + dx;
                let max_y = (module.position.y + height) as i16 + dy;
                (
                    GridPos::new(min_x, min_y),
                    GridPos::new(max_x.max(0) as u16, max_y.max(0) as u16),
                )
            }
            Mode::SelectMove {
                anchor,
                extent,
                origin,
            }
            | Mode::CopySelection {
                anchor,
                extent,
                origin,
            } => {
                let source_min = GridPos::new(anchor.x.min(extent.x), anchor.y.min(extent.y));
                let source_max = GridPos::new(anchor.x.max(extent.x), anchor.y.max(extent.y));
                let dx = bounded_grid_delta(
                    source_min.x,
                    source_max.x,
                    self.cursor().x as i16 - origin.x as i16,
                    self.width,
                );
                let dy = bounded_grid_delta(
                    source_min.y,
                    source_max.y,
                    self.cursor().y as i16 - origin.y as i16,
                    self.height,
                );
                let min_x = (source_min.x as i16 + dx).max(0) as u16;
                let min_y = (source_min.y as i16 + dy).max(0) as u16;
                let max_x = source_max.x as i16 + dx;
                let max_y = source_max.y as i16 + dy;
                (
                    GridPos::new(min_x, min_y),
                    GridPos::new(max_x.max(0) as u16, max_y.max(0) as u16),
                )
            }
            Mode::Select { anchor } => (
                GridPos::new(anchor.x.min(self.cursor().x), anchor.y.min(self.cursor().y)),
                GridPos::new(anchor.x.max(self.cursor().x), anchor.y.max(self.cursor().y)),
            ),
            _ => {
                let cursor = self.cursor();
                (cursor, cursor)
            }
        }
    }

    fn move_palette_category(&mut self, delta: i16) {
        let current = ModuleCategory::ALL
            .iter()
            .position(|category| *category == self.palette_category)
            .unwrap_or(0);
        let max = ModuleCategory::ALL.len() as i16 - 1;
        let next = (current as i16 + delta).clamp(0, max) as usize;
        self.palette_category = ModuleCategory::ALL[next];
        self.palette_index = 0;
    }

    fn move_palette_selection(&mut self, delta: i16) {
        let max = self.palette_modules().len().saturating_sub(1) as i16;
        self.palette_index = (self.palette_index as i16 + delta).clamp(0, max) as usize;
    }

    fn insert_at_cursor(&mut self, kind: ModuleKind) {
        let cursor = self.cursor();
        if !self.kind_fits(kind, Orientation::Right, cursor) {
            return;
        }
        let before = self.snapshot();
        let id = ModuleId(self.next_module_id);
        let module = Module {
            id,
            kind,
            position: cursor,
            orientation: Orientation::Right,
            parameters: kind.default_parameters(),
            env_points: kind.default_env_points(),
            disabled: false,
        };
        self.next_module_id += 1;
        self.instrument_mut().surface_mut().modules.push(module);
        if kind == ModuleKind::Subpatch {
            self.instrument_mut().ensure_subpatch(id);
        }
        self.commit(before);
    }

    fn remove_subpatch_surface(&mut self, owner: ModuleId) {
        let inst = self.instrument_mut();
        inst.subpatches.retain(|subpatch| subpatch.owner != owner);
        inst.subpatch_stack
            .retain(|(parent, _)| *parent != Some(owner));
        if inst.editing_subpatch == Some(owner) {
            inst.editing_subpatch = None;
        }
    }
}

fn surface_from_project_modules(
    modules: &[ProjectModuleDef],
) -> (PatchSurface, Vec<(ModuleId, u32)>) {
    let mut subpatch_owners = Vec::new();
    let modules = modules
        .iter()
        .filter_map(|definition| {
            let (module, subpatch_id) = module_from_project(definition)?;
            if let Some(subpatch_id) = subpatch_id {
                subpatch_owners.push((module.id, subpatch_id));
            }
            Some(module)
        })
        .collect();
    (
        PatchSurface {
            cursor: GridPos::new(0, 0),
            modules,
        },
        subpatch_owners,
    )
}

fn add_project_subpatches(
    instrument: &mut Instrument,
    owners: Vec<(ModuleId, u32)>,
    subpatches: &HashMap<u32, &ProjectSubpatchDef>,
) {
    for (owner, project_id) in owners {
        let Some(definition) = subpatches.get(&project_id) else {
            continue;
        };
        let (surface, nested_owners) = surface_from_project_modules(&definition.modules);
        instrument
            .subpatches
            .push(SubpatchSurface { owner, surface });
        add_project_subpatches(instrument, nested_owners, subpatches);
    }
}

fn module_from_project(definition: &ProjectModuleDef) -> Option<(Module, Option<u32>)> {
    let (kind, subpatch_id) = kind_from_project(definition.kind)?;
    let mut module = Module {
        id: ModuleId(definition.id),
        kind,
        position: GridPos::new(definition.x, definition.y),
        orientation: orientation_from_project(definition.orientation),
        parameters: kind.default_parameters(),
        env_points: kind.default_env_points(),
        disabled: false,
    };
    apply_project_params(&mut module, &definition.params);
    Some((module, subpatch_id))
}

fn kind_from_project(kind: ProjectModuleKind) -> Option<(ModuleKind, Option<u32>)> {
    Some(match kind {
        ProjectModuleKind::Routing(routing) => (
            match routing {
                ProjectRoutingModule::LSplit => ModuleKind::LeftSplit,
                ProjectRoutingModule::TSplit => ModuleKind::TopSplit,
                ProjectRoutingModule::RJoin => ModuleKind::RightJoin,
                ProjectRoutingModule::DJoin => ModuleKind::DownJoin,
                ProjectRoutingModule::TurnRD => ModuleKind::TurnRightDown,
                ProjectRoutingModule::TurnDR => ModuleKind::TurnDownRight,
            },
            None,
        ),
        ProjectModuleKind::Subpatch(subpatch) => match subpatch {
            ProjectSubpatchModule::SubIn => (ModuleKind::SubpatchInput, None),
            ProjectSubpatchModule::SubOut => (ModuleKind::SubpatchOutput, None),
            ProjectSubpatchModule::SubPatch(id) => (ModuleKind::Subpatch, Some(id.0)),
        },
        ProjectModuleKind::Standard(standard) => (
            match standard {
                ProjectStandardModule::Freq => ModuleKind::Freq,
                ProjectStandardModule::Gate => ModuleKind::Gate,
                ProjectStandardModule::Degree => ModuleKind::Degree,
                ProjectStandardModule::DegreeGate => ModuleKind::DegreeGate,
                ProjectStandardModule::Osc => ModuleKind::Osc,
                ProjectStandardModule::Rise => ModuleKind::Rise,
                ProjectStandardModule::Fall => ModuleKind::Fall,
                ProjectStandardModule::Ramp => ModuleKind::Ramp,
                ProjectStandardModule::Adsr => ModuleKind::Adsr,
                ProjectStandardModule::Envelope => ModuleKind::Envelope,
                ProjectStandardModule::Lpf => ModuleKind::Lowpass,
                ProjectStandardModule::Hpf => ModuleKind::Highpass,
                ProjectStandardModule::Comb => ModuleKind::Comb,
                ProjectStandardModule::Allpass => ModuleKind::Allpass,
                ProjectStandardModule::Delay => ModuleKind::Delay,
                ProjectStandardModule::DelayTap(_) => ModuleKind::DelayTap,
                ProjectStandardModule::Reverb => ModuleKind::Reverb,
                ProjectStandardModule::Distortion => ModuleKind::Distortion,
                ProjectStandardModule::Compressor => ModuleKind::Compressor,
                ProjectStandardModule::Flanger => ModuleKind::Flanger,
                ProjectStandardModule::Mul => ModuleKind::Multiply,
                ProjectStandardModule::Add => ModuleKind::Add,
                ProjectStandardModule::Gt => ModuleKind::GreaterThan,
                ProjectStandardModule::Lt => ModuleKind::LessThan,
                ProjectStandardModule::Switch => ModuleKind::Switch,
                ProjectStandardModule::Rng => ModuleKind::Random,
                ProjectStandardModule::Sample => ModuleKind::Sample,
                ProjectStandardModule::Probe => ModuleKind::Probe,
                ProjectStandardModule::Output => ModuleKind::Output,
            },
            None,
        ),
    })
}

fn orientation_from_project(orientation: ProjectOrientation) -> Orientation {
    match orientation {
        ProjectOrientation::Horizontal => Orientation::Right,
        ProjectOrientation::Vertical => Orientation::Down,
    }
}

fn apply_project_params(module: &mut Module, params: &ProjectModuleParams) {
    match params {
        ProjectModuleParams::None | ProjectModuleParams::SubPatch { .. } => {}
        ProjectModuleParams::DegreeGate { degree } => set_int(&mut module.parameters, 0, *degree),
        ProjectModuleParams::Osc {
            wave,
            freq,
            shift,
            gain,
            uni,
            connected,
        } => {
            set_enum(&mut module.parameters, 0, wave_index(*wave));
            set_time(&mut module.parameters, 1, *freq);
            set_float(&mut module.parameters, 2, *shift);
            set_float(&mut module.parameters, 3, *gain);
            set_toggle(&mut module.parameters, 4, *uni);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Rise { time, connected }
        | ProjectModuleParams::Fall { time, connected } => {
            set_time(&mut module.parameters, 1, *time);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Ramp {
            value,
            time,
            connected,
        } => {
            set_float(&mut module.parameters, 0, *value);
            set_time(&mut module.parameters, 1, *time);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Adsr {
            attack_ratio,
            sustain,
            connected,
        } => {
            set_float(&mut module.parameters, 2, *attack_ratio);
            set_float(&mut module.parameters, 3, *sustain);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Envelope { points, connected } => {
            module.env_points = points.iter().map(env_point_from_project).collect();
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Filter { freq, q, connected } => {
            set_float(&mut module.parameters, 1, *freq);
            set_float(&mut module.parameters, 2, *q);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Comb {
            time,
            feedback,
            damp,
            connected,
        } => {
            set_time(&mut module.parameters, 1, *time);
            set_float(&mut module.parameters, 2, *feedback);
            set_float(&mut module.parameters, 3, *damp);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Allpass {
            time,
            feedback,
            connected,
        } => {
            set_time(&mut module.parameters, 1, *time);
            set_float(&mut module.parameters, 2, *feedback);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Delay { time, connected } => {
            set_time(&mut module.parameters, 1, *time);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Reverb {
            room,
            damp,
            mod_depth,
            diffusion,
            connected,
        } => {
            set_float(&mut module.parameters, 1, *room);
            set_float(&mut module.parameters, 2, *damp);
            set_float(&mut module.parameters, 3, *mod_depth);
            set_float(&mut module.parameters, 4, *diffusion);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Distortion {
            dist_type,
            drive,
            asymmetry,
            connected,
        } => {
            set_enum(&mut module.parameters, 1, distortion_index(*dist_type));
            set_float(&mut module.parameters, 2, *drive);
            set_float(&mut module.parameters, 3, *asymmetry);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Flanger {
            rate,
            depth,
            feedback,
            connected,
        } => {
            set_float(&mut module.parameters, 1, *rate);
            set_float(&mut module.parameters, 2, *depth);
            set_float(&mut module.parameters, 3, *feedback);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Compressor {
            threshold,
            ratio,
            attack,
            release,
            makeup,
            connected,
        } => {
            set_float(&mut module.parameters, 1, *threshold);
            set_float(&mut module.parameters, 2, *ratio);
            set_float(&mut module.parameters, 3, *attack);
            set_float(&mut module.parameters, 4, *release);
            set_float(&mut module.parameters, 5, *makeup);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Mul { a, b, connected }
        | ProjectModuleParams::Add { a, b, connected }
        | ProjectModuleParams::Gt { a, b, connected }
        | ProjectModuleParams::Lt { a, b, connected } => {
            set_float(&mut module.parameters, 0, *a);
            set_float(&mut module.parameters, 1, *b);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Switch { a, b, connected } => {
            set_float(&mut module.parameters, 1, *a);
            set_float(&mut module.parameters, 2, *b);
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::Sample { connected, .. }
        | ProjectModuleParams::Probe { connected }
        | ProjectModuleParams::Output { connected, .. } => {
            if let ProjectModuleParams::Output { gain, .. } = params {
                set_float(&mut module.parameters, 1, *gain);
            }
            apply_connected(&mut module.parameters, *connected);
        }
        ProjectModuleParams::DelayTap { gain } => set_float(&mut module.parameters, 1, *gain),
    }
}

fn set_float(parameters: &mut [ModuleParameter], index: usize, value: f32) {
    if let Some(parameter) = parameters.get_mut(index)
        && let ParameterValue::Float {
            value: target,
            min,
            max,
            ..
        } = &mut parameter.value
    {
        *target = (value * 100.0).round().clamp(*min as f32, *max as f32) as i32;
    }
}

fn set_int(parameters: &mut [ModuleParameter], index: usize, value: i32) {
    if let Some(parameter) = parameters.get_mut(index)
        && let ParameterValue::Int {
            value: target,
            min,
            max,
        } = &mut parameter.value
    {
        *target = value.clamp(*min, *max);
    }
}

fn set_time(parameters: &mut [ModuleParameter], index: usize, value: ProjectTimeValue) {
    if let Some(parameter) = parameters.get_mut(index)
        && let ParameterValue::Time {
            value: target,
            unit,
        } = &mut parameter.value
    {
        let (next_unit, next_value) = time_from_project(value);
        *unit = next_unit;
        *target = next_value;
    }
}

fn set_enum(parameters: &mut [ModuleParameter], index: usize, value: usize) {
    if let Some(parameter) = parameters.get_mut(index)
        && let ParameterValue::Enum {
            index: target,
            options,
        } = &mut parameter.value
        && value < options.len()
    {
        *target = value;
    }
}

fn set_toggle(parameters: &mut [ModuleParameter], index: usize, value: bool) {
    if let Some(parameter) = parameters.get_mut(index)
        && let ParameterValue::Toggle(target) = &mut parameter.value
    {
        *target = value;
    }
}

fn apply_connected(parameters: &mut [ModuleParameter], connected: u8) {
    for (index, parameter) in parameters.iter_mut().enumerate() {
        if parameter.value.is_port() {
            parameter.connected = (connected & (1 << index)) != 0;
        }
    }
}

fn time_from_project(value: ProjectTimeValue) -> (TimeUnit, i32) {
    match value.unit {
        ProjectTimeUnit::Seconds => (TimeUnit::Seconds, (value.seconds * 100.0).round() as i32),
        ProjectTimeUnit::Samples => (TimeUnit::Samples, value.samples.round() as i32),
        ProjectTimeUnit::Bars => (
            TimeUnit::Bars,
            ((value.bar_num as f32 / value.bar_denom.max(1) as f32) * 16.0).round() as i32,
        ),
        ProjectTimeUnit::Hz => (TimeUnit::Hertz, value.hz.round() as i32),
    }
}

fn env_point_from_project(point: &brainwash::project::EnvPoint) -> EnvPoint {
    EnvPoint {
        time: (point.time * 100.0).round().clamp(0.0, 100.0) as i32,
        value: (point.value * 100.0).round().clamp(-100.0, 100.0) as i32,
        curve: point.curve,
    }
}

fn wave_index(wave: ProjectWaveType) -> usize {
    match wave {
        ProjectWaveType::Sin => 0,
        ProjectWaveType::Squ => 1,
        ProjectWaveType::Tri => 2,
        ProjectWaveType::Saw => 3,
        ProjectWaveType::RSaw => 4,
        ProjectWaveType::Noise => 5,
    }
}

fn distortion_index(distortion: ProjectDistType) -> usize {
    match distortion {
        ProjectDistType::Tube => 0,
        ProjectDistType::Tape => 1,
        ProjectDistType::Fuzz => 2,
        ProjectDistType::Fold => 3,
        ProjectDistType::Clip => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gui_module_kind_has_audio_mapping() {
        let rate = SampleRate::new(44_100).unwrap();
        for kind in all_modules().iter().copied() {
            let module = Module {
                id: ModuleId(0),
                kind,
                position: GridPos::new(0, 0),
                orientation: Orientation::Right,
                parameters: kind.default_parameters(),
                env_points: kind.default_env_points(),
                disabled: false,
            };
            assert!(audio_module(&module, rate, 120).is_ok(), "{kind:?}");
        }
    }
}

impl ModuleKind {
    fn width(self, orientation: Orientation) -> u16 {
        if self.is_routing() {
            return 1;
        }
        match orientation {
            Orientation::Right => self.output_count().max(1),
            Orientation::Down => self.input_count().max(1),
        }
    }

    fn height(self, orientation: Orientation) -> u16 {
        if self.is_routing() {
            return 1;
        }
        match orientation {
            Orientation::Right => self.input_count().max(1),
            Orientation::Down => self.output_count().max(1),
        }
    }

    fn input_count(self) -> u16 {
        match self {
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::Random
            | ModuleKind::Sample
            | ModuleKind::SubpatchInput => 0,
            ModuleKind::RightJoin | ModuleKind::DownJoin => 2,
            ModuleKind::TurnRightDown
            | ModuleKind::TurnDownRight
            | ModuleKind::LeftSplit
            | ModuleKind::TopSplit => 1,
            ModuleKind::DegreeGate
            | ModuleKind::Osc
            | ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope
            | ModuleKind::Lowpass
            | ModuleKind::Highpass
            | ModuleKind::Comb
            | ModuleKind::Allpass
            | ModuleKind::Delay
            | ModuleKind::DelayTap
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Compressor
            | ModuleKind::Flanger
            | ModuleKind::Probe
            | ModuleKind::Multiply
            | ModuleKind::Add
            | ModuleKind::GreaterThan
            | ModuleKind::LessThan
            | ModuleKind::Switch
            | ModuleKind::Output
            | ModuleKind::SubpatchOutput
            | ModuleKind::Subpatch => self
                .default_parameters()
                .iter()
                .filter(|parameter| parameter.value.is_port())
                .count() as u16,
        }
    }

    fn output_count(self) -> u16 {
        match self {
            ModuleKind::Output | ModuleKind::SubpatchOutput => 0,
            ModuleKind::LeftSplit | ModuleKind::TopSplit => 2,
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::DegreeGate
            | ModuleKind::Osc
            | ModuleKind::Rise
            | ModuleKind::Fall
            | ModuleKind::Ramp
            | ModuleKind::Adsr
            | ModuleKind::Envelope
            | ModuleKind::Lowpass
            | ModuleKind::Highpass
            | ModuleKind::Comb
            | ModuleKind::Allpass
            | ModuleKind::Delay
            | ModuleKind::DelayTap
            | ModuleKind::Reverb
            | ModuleKind::Distortion
            | ModuleKind::Compressor
            | ModuleKind::Flanger
            | ModuleKind::Probe
            | ModuleKind::Multiply
            | ModuleKind::Add
            | ModuleKind::GreaterThan
            | ModuleKind::LessThan
            | ModuleKind::Switch
            | ModuleKind::Random
            | ModuleKind::Sample
            | ModuleKind::TurnRightDown
            | ModuleKind::TurnDownRight
            | ModuleKind::RightJoin
            | ModuleKind::DownJoin
            | ModuleKind::SubpatchInput
            | ModuleKind::Subpatch => 1,
        }
    }

    fn is_routing(self) -> bool {
        matches!(
            self,
            ModuleKind::TurnRightDown
                | ModuleKind::TurnDownRight
                | ModuleKind::LeftSplit
                | ModuleKind::TopSplit
                | ModuleKind::RightJoin
                | ModuleKind::DownJoin
        )
    }
}

fn collect_audio_inputs(module: ModuleId, connections: &[Connection], needed: &mut Vec<ModuleId>) {
    if needed.contains(&module) {
        return;
    }
    needed.push(module);
    for connection in connections {
        if connection.to == module {
            collect_audio_inputs(connection.from, connections, needed);
        }
    }
}

fn module_uses_voice_controls(kind: ModuleKind) -> bool {
    matches!(
        kind,
        ModuleKind::Freq | ModuleKind::Gate | ModuleKind::Degree | ModuleKind::DegreeGate
    )
}

fn audio_patch_error_message(error: &AudioPatchError) -> &'static str {
    match error {
        AudioPatchError::MissingOutput => "connect Osc to Output",
        AudioPatchError::InvalidParameter => "invalid parameter",
        AudioPatchError::Connect(_) => "invalid connection",
        AudioPatchError::Compile(CompileError::Cycle) => "cycle",
        AudioPatchError::Compile(CompileError::InvalidDelay) => "invalid delay",
        AudioPatchError::Compile(CompileError::InvalidInput) => "invalid input",
        AudioPatchError::Compile(CompileError::MissingModule) => "missing module",
        AudioPatchError::Compile(CompileError::MissingOutput) => "missing output",
    }
}

fn audio_module(
    module: &Module,
    rate: SampleRate,
    bpm: u16,
) -> Result<AudioModule, AudioPatchError> {
    match module.kind {
        ModuleKind::Freq => Ok(AudioModule::Freq),
        ModuleKind::Gate => Ok(AudioModule::Gate),
        ModuleKind::Degree => Ok(AudioModule::Degree),
        ModuleKind::DegreeGate => Ok(AudioModule::DegreeGate {
            target: audio_int(module, 0)?,
        }),
        ModuleKind::Osc => Ok(AudioModule::Osc {
            wave: audio_wave(module)?,
            frequency: audio_frequency(module, 1, rate)?,
            shift: audio_float(module, 2)?,
            gain: audio_unit(module, 3)?,
            unipolar: audio_toggle(module, 4)?,
        }),
        ModuleKind::Rise => Ok(AudioModule::Rise {
            time: audio_duration(module, 1, rate, bpm)?,
        }),
        ModuleKind::Fall => Ok(AudioModule::Fall {
            time: audio_duration(module, 1, rate, bpm)?,
        }),
        ModuleKind::Ramp => Ok(AudioModule::Ramp {
            value: audio_float(module, 0)?,
            time: audio_duration(module, 1, rate, bpm)?,
        }),
        ModuleKind::Adsr => Ok(AudioModule::Adsr {
            attack_ratio: audio_unit(module, 2)?,
            sustain: audio_unit(module, 3)?,
        }),
        ModuleKind::Envelope => Ok(AudioModule::Envelope {
            points: Arc::new(
                module
                    .env_points
                    .iter()
                    .map(|point| AudioEnvPoint {
                        time: point.time as f32 / 100.0,
                        value: point.value as f32 / 100.0,
                        curve: point.curve,
                    })
                    .collect(),
            ),
        }),
        ModuleKind::Lowpass => Ok(AudioModule::Lowpass {
            cutoff: filter_cutoff(audio_float(module, 1)?)?,
        }),
        ModuleKind::Highpass => Ok(AudioModule::Highpass {
            cutoff: filter_cutoff(audio_float(module, 1)?)?,
        }),
        ModuleKind::Comb => Ok(AudioModule::Comb {
            time: audio_duration(module, 1, rate, bpm)?,
            feedback: audio_unit(module, 2)?,
            damp: audio_unit(module, 3)?,
        }),
        ModuleKind::Allpass => Ok(AudioModule::Allpass {
            time: audio_duration(module, 1, rate, bpm)?,
            feedback: audio_unit(module, 2)?,
        }),
        ModuleKind::Delay => Ok(AudioModule::Delay {
            time: audio_duration(module, 1, rate, bpm)?,
            feedback: Unit::ZERO,
        }),
        ModuleKind::DelayTap => Ok(AudioModule::DelayTap {
            gain: audio_unit(module, 1)?,
        }),
        ModuleKind::Reverb => Ok(AudioModule::Reverb {
            room: audio_unit(module, 1)?,
            damp: audio_unit(module, 2)?,
            mod_depth: audio_unit(module, 3)?,
            diffusion: audio_unit(module, 4)?,
        }),
        ModuleKind::Distortion => Ok(AudioModule::Distortion {
            kind: audio_distortion(module)?,
            drive: Drive::new(audio_float(module, 2)?).ok_or(AudioPatchError::InvalidParameter)?,
        }),
        ModuleKind::Compressor => Ok(AudioModule::Compressor {
            threshold: audio_unit(module, 1)?,
            ratio: audio_float(module, 2)?,
            attack: seconds(audio_float(module, 3)?)?,
            release: seconds(audio_float(module, 4)?)?,
            makeup: audio_float(module, 5)?,
        }),
        ModuleKind::Flanger => Ok(AudioModule::Flanger {
            rate: Hertz::new(audio_float(module, 1)?).ok_or(AudioPatchError::InvalidParameter)?,
            depth: audio_unit(module, 2)?,
            feedback: audio_unit(module, 3)?,
        }),
        ModuleKind::Multiply => Ok(AudioModule::Binary {
            op: BinaryOp::Multiply,
            a: audio_float(module, 0)?,
            b: audio_float(module, 1)?,
        }),
        ModuleKind::Add => Ok(AudioModule::Binary {
            op: BinaryOp::Add,
            a: audio_float(module, 0)?,
            b: audio_float(module, 1)?,
        }),
        ModuleKind::GreaterThan => Ok(AudioModule::Binary {
            op: BinaryOp::GreaterThan,
            a: audio_float(module, 0)?,
            b: audio_float(module, 1)?,
        }),
        ModuleKind::LessThan => Ok(AudioModule::Binary {
            op: BinaryOp::LessThan,
            a: audio_float(module, 0)?,
            b: audio_float(module, 1)?,
        }),
        ModuleKind::Switch => Ok(AudioModule::Switch {
            a: audio_float(module, 1)?,
            b: audio_float(module, 2)?,
        }),
        ModuleKind::Random => Ok(AudioModule::Random),
        ModuleKind::Sample => Ok(AudioModule::Sample {
            samples: Arc::new(Vec::new()),
        }),
        ModuleKind::Probe => Ok(AudioModule::Probe),
        ModuleKind::TurnRightDown
        | ModuleKind::TurnDownRight
        | ModuleKind::LeftSplit
        | ModuleKind::TopSplit
        | ModuleKind::RightJoin
        | ModuleKind::DownJoin
        | ModuleKind::SubpatchInput
        | ModuleKind::SubpatchOutput
        | ModuleKind::Subpatch => Ok(AudioModule::Pass),
        ModuleKind::Output => Ok(AudioModule::Pass),
    }
}

fn connection_input(connection: &Connection, target: &Module, input_count: u16) -> Option<usize> {
    let input = match connection.orientation {
        Orientation::Right => connection.to_cell.y.checked_sub(target.position.y)?,
        Orientation::Down => connection.to_cell.x.checked_sub(target.position.x)?,
    } as usize;
    (input < input_count as usize).then_some(input)
}

fn audio_id(ids: &[(AudioKey, AudioModuleId)], key: AudioKey) -> Option<AudioModuleId> {
    ids.iter()
        .find_map(|(candidate, audio_id)| (*candidate == key).then_some(*audio_id))
}

fn root_connection_source(
    root_modules: &[Module],
    instrument: &Instrument,
    ids: &[(AudioKey, AudioModuleId)],
    connection: &Connection,
) -> Option<AudioModuleId> {
    let source = root_modules
        .iter()
        .find(|module| module.id == connection.from)?;
    if source.kind != ModuleKind::Subpatch {
        return audio_id(ids, AudioKey::Root(source.id));
    }
    let output = subpatch_output_index(connection, source)?;
    let subpatch = instrument
        .subpatches
        .iter()
        .find(|subpatch| subpatch.owner == source.id)?;
    let outputs = subpatch_outputs(&subpatch.surface);
    let module = outputs.get(output).copied()?;
    audio_id(
        ids,
        AudioKey::Subpatch {
            owner: source.id,
            module,
        },
    )
}

fn subpatch_input_key(module: &Module) -> u16 {
    module.position.y
}

fn subpatch_output_key(module: &Module) -> u16 {
    module.position.x
}

fn subpatch_inputs(surface: &PatchSurface) -> Vec<ModuleId> {
    let mut modules = surface
        .modules
        .iter()
        .filter(|module| !module.disabled && module.kind == ModuleKind::SubpatchInput)
        .collect::<Vec<_>>();
    modules.sort_by_key(|module| subpatch_input_key(module));
    modules.into_iter().map(|module| module.id).collect()
}

fn subpatch_outputs(surface: &PatchSurface) -> Vec<ModuleId> {
    let mut modules = surface
        .modules
        .iter()
        .filter(|module| !module.disabled && module.kind == ModuleKind::SubpatchOutput)
        .collect::<Vec<_>>();
    modules.sort_by_key(|module| subpatch_output_key(module));
    modules.into_iter().map(|module| module.id).collect()
}

fn subpatch_output_index(connection: &Connection, source: &Module) -> Option<usize> {
    let index = match connection.orientation {
        Orientation::Right => connection.from_cell.y.checked_sub(source.position.y)?,
        Orientation::Down => connection.from_cell.x.checked_sub(source.position.x)?,
    };
    Some(index as usize)
}

fn audio_wave(module: &Module) -> Result<Wave, AudioPatchError> {
    let Some(ParameterValue::Enum { index, .. }) = module.parameters.first().map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    match *index {
        0 => Ok(Wave::Sine),
        1 => Ok(Wave::Square),
        2 => Ok(Wave::Triangle),
        3 | 4 => Ok(Wave::Saw),
        5 => Ok(Wave::Noise),
        _ => Err(AudioPatchError::InvalidParameter),
    }
}

fn audio_frequency(
    module: &Module,
    parameter: usize,
    rate: SampleRate,
) -> Result<Hertz, AudioPatchError> {
    let Some(ParameterValue::Time { value, unit }) =
        module.parameters.get(parameter).map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    let hz = match unit {
        TimeUnit::Hertz => *value as f32,
        TimeUnit::Seconds => 100.0 / (*value).max(1) as f32,
        TimeUnit::Samples => rate.value() as f32 / (*value).max(1) as f32,
        TimeUnit::Bars => 1.0,
    };
    Hertz::new(hz).ok_or(AudioPatchError::InvalidParameter)
}

fn audio_duration(
    module: &Module,
    parameter: usize,
    rate: SampleRate,
    bpm: u16,
) -> Result<Duration, AudioPatchError> {
    let Some(ParameterValue::Time { value, unit }) =
        module.parameters.get(parameter).map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    match unit {
        TimeUnit::Seconds => seconds(*value as f32 / 100.0),
        TimeUnit::Samples => Ok(Duration::Samples(Samples::new((*value).max(1) as u64))),
        TimeUnit::Bars => {
            let beats = *value as f32 / 16.0 * 4.0;
            seconds(beats * 60.0 / bpm.max(1) as f32)
        }
        TimeUnit::Hertz => {
            let hz = (*value).max(1) as f32;
            seconds(1.0 / hz.min(rate.value() as f32))
        }
    }
}

fn seconds(value: f32) -> Result<Duration, AudioPatchError> {
    Seconds::new(value)
        .map(Duration::Seconds)
        .ok_or(AudioPatchError::InvalidParameter)
}

fn audio_float(module: &Module, parameter: usize) -> Result<f32, AudioPatchError> {
    let Some(ParameterValue::Float { value, .. }) =
        module.parameters.get(parameter).map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    Ok(*value as f32 / 100.0)
}

fn audio_int(module: &Module, parameter: usize) -> Result<i32, AudioPatchError> {
    let Some(ParameterValue::Int { value, .. }) =
        module.parameters.get(parameter).map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    Ok(*value)
}

fn audio_toggle(module: &Module, parameter: usize) -> Result<bool, AudioPatchError> {
    let Some(ParameterValue::Toggle(value)) = module.parameters.get(parameter).map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    Ok(*value)
}

fn audio_unit(module: &Module, parameter: usize) -> Result<Unit, AudioPatchError> {
    Unit::new(audio_float(module, parameter)?.clamp(0.0, 1.0))
        .ok_or(AudioPatchError::InvalidParameter)
}

fn filter_cutoff(value: f32) -> Result<Hertz, AudioPatchError> {
    Hertz::new(20.0 * 1000.0_f32.powf(value.clamp(0.0, 1.0)))
        .ok_or(AudioPatchError::InvalidParameter)
}

fn audio_distortion(module: &Module) -> Result<AudioDistortion, AudioPatchError> {
    let Some(ParameterValue::Enum { index, .. }) = module.parameters.get(1).map(|p| p.value())
    else {
        return Err(AudioPatchError::InvalidParameter);
    };
    match *index {
        0 | 1 | 2 => Ok(AudioDistortion::Tanh),
        3 => Ok(AudioDistortion::Fold),
        4 => Ok(AudioDistortion::Clip),
        _ => Err(AudioPatchError::InvalidParameter),
    }
}

fn scale_from_index(index: usize) -> Scale {
    match index {
        0 => chromatic(),
        1 => cmaj(),
        2 => cmin(),
        3 => csharpmaj(),
        4 => csharpmin(),
        5 => dmaj(),
        6 => dmin(),
        7 => dsharpmaj(),
        8 => dsharpmin(),
        9 => emaj(),
        10 => emin(),
        11 => fmaj(),
        12 => fmin(),
        13 => fsharpmaj(),
        14 => fsharpmin(),
        15 => gmaj(),
        16 => gmin(),
        17 => gsharpmaj(),
        18 => gsharpmin(),
        19 => amaj(),
        20 => amin(),
        21 => asharpmaj(),
        22 => asharpmin(),
        23 => bmaj(),
        24 => bmin(),
        _ => cmin(),
    }
}

fn typed_parameter_value(current: &ParameterValue, text: &str) -> Option<ParameterValue> {
    match current {
        ParameterValue::Float { min, max, step, .. } => {
            let value = (text.parse::<f32>().ok()? * 100.0).round() as i32;
            Some(ParameterValue::Float {
                value: value.clamp(*min, *max),
                min: *min,
                max: *max,
                step: *step,
            })
        }
        ParameterValue::Time { unit, .. } => match unit {
            TimeUnit::Seconds => Some(ParameterValue::Time {
                value: ((text.parse::<f32>().ok()? * 100.0).round() as i32).max(1),
                unit: *unit,
            }),
            TimeUnit::Samples | TimeUnit::Hertz => Some(ParameterValue::Time {
                value: text.parse::<i32>().ok()?.max(1),
                unit: *unit,
            }),
            TimeUnit::Bars => {
                let (num, denom) = text.split_once('/')?;
                let num = num.parse::<i32>().ok()?.max(1);
                let denom = denom.parse::<i32>().ok()?;
                if denom <= 0 {
                    return None;
                }
                Some(ParameterValue::Time {
                    value: ((num * 16) / denom).max(1),
                    unit: *unit,
                })
            }
        },
        ParameterValue::Int { .. }
        | ParameterValue::Input
        | ParameterValue::Enum { .. }
        | ParameterValue::Toggle(_) => None,
    }
}

fn normalized_rect(anchor: GridPos, extent: GridPos) -> (GridPos, GridPos) {
    (
        GridPos::new(anchor.x.min(extent.x), anchor.y.min(extent.y)),
        GridPos::new(anchor.x.max(extent.x), anchor.y.max(extent.y)),
    )
}

fn bounded_grid_delta(min: u16, max: u16, delta: i16, size: u16) -> i16 {
    let lower = -(min as i16);
    let upper = size.saturating_sub(1).saturating_sub(max) as i16;
    delta.clamp(lower, upper)
}

fn updated_axis_view(offset: u16, min: u16, max: u16, size: u16, visible: u16) -> u16 {
    if size <= visible {
        return 0;
    }
    let max_offset = size - visible;
    let margin = GRID_VIEW_MARGIN.min(visible.saturating_sub(1) / 2);
    let low = offset.saturating_add(margin);
    if min < low {
        return min.saturating_sub(margin).min(max_offset);
    }
    let high = offset + visible - 1 - margin;
    if max > high {
        return max
            .saturating_add(margin)
            .saturating_add(1)
            .saturating_sub(visible)
            .min(max_offset);
    }
    offset.min(max_offset)
}

pub fn all_modules() -> &'static [ModuleKind] {
    &[
        ModuleKind::Osc,
        ModuleKind::Output,
        ModuleKind::Freq,
        ModuleKind::Gate,
        ModuleKind::Degree,
        ModuleKind::Adsr,
        ModuleKind::Envelope,
        ModuleKind::Rise,
        ModuleKind::Fall,
        ModuleKind::Ramp,
        ModuleKind::Lowpass,
        ModuleKind::Highpass,
        ModuleKind::Delay,
        ModuleKind::Reverb,
        ModuleKind::Distortion,
        ModuleKind::Compressor,
        ModuleKind::Flanger,
        ModuleKind::Add,
        ModuleKind::Multiply,
        ModuleKind::Switch,
        ModuleKind::Probe,
        ModuleKind::Sample,
        ModuleKind::Random,
        ModuleKind::DegreeGate,
        ModuleKind::GreaterThan,
        ModuleKind::LessThan,
        ModuleKind::Comb,
        ModuleKind::Allpass,
        ModuleKind::DelayTap,
        ModuleKind::TurnRightDown,
        ModuleKind::TurnDownRight,
        ModuleKind::LeftSplit,
        ModuleKind::TopSplit,
        ModuleKind::RightJoin,
        ModuleKind::DownJoin,
        ModuleKind::SubpatchInput,
        ModuleKind::SubpatchOutput,
        ModuleKind::Subpatch,
    ]
}
