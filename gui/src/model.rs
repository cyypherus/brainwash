use crate::audio::{AudioHandle, MeterRoute, ProbeRoute, VoiceMode};
use brainwash::compile::{CompileError, CompiledPatch};
use brainwash::effect::{Distortion as AudioDistortion, Drive};
use brainwash::osc::Wave;
use brainwash::patch::{
    BinaryOp, CompressorRatio, ConnectError, EnvPoint as AudioEnvPoint, Gain,
    InputKind as AudioInputKind, Module as AudioModule, ModuleId as AudioModuleId, Patch,
};
use brainwash::sample::Sample;
use brainwash::sample::{Sample as AudioSample, Unit};
use brainwash::scale::Scale;
use brainwash::scale::{
    amaj, amin, asharpmaj, asharpmin, bmaj, bmin, chromatic, cmaj, cmin, csharpmaj, csharpmin,
    dmaj, dmin, dsharpmaj, dsharpmin, emaj, emin, fmaj, fmin, fsharpmaj, fsharpmin, gmaj, gmin,
    gsharpmaj, gsharpmin,
};
use brainwash::time::{Duration, Hertz, SampleRate, Samples, Seconds};
use brainwash::track::Track;
use brainwash_grid::bounded_delta;
use brainwash_grid::project::{
    self, CompositionDef as ProjectCompositionDef, CompositionId as ProjectCompositionId,
    CompositionModule as ProjectCompositionModule, ModuleDef as ProjectModuleDef,
    ModuleKind as ProjectModuleKind, ModuleParams as ProjectModuleParams, Project,
    RoutingModule as ProjectRoutingModule, StandardModule as ProjectStandardModule,
    TimeUnit as ProjectTimeUnit, TimeValue as ProjectTimeValue, WaveType as ProjectWaveType,
};
pub use brainwash_grid::{
    ModuleId, Orientation, Position as GridPos, Rect as GridRect, Size as GridSize,
};
use haven::{ButtonState, TextState};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use std::sync::Arc;

mod audio_patch;
mod composition;
mod interaction;
mod persistence;

use audio_patch::{audio_module, audio_patch_error_message, composition_inputs, scale_from_index};
use composition::{composition_body, graph_node_label, sync_delay_sources};
use persistence::{instrument_surface_from_project, project_from_instrument};

const GRID_VIEW_MARGIN: u16 = 2;
const INSTRUMENT_COUNT: usize = 5;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connection {
    from: ModuleId,
    to: ModuleId,
    output: usize,
    input: ConnectionInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConnectionInput {
    index: usize,
    audio: AudioInputKind,
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
    OpenModules,
    TrackSettings,
    TrackEdit,
    Search,
    EditComposition,
    ExitComposition,
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
    LoadConfirm,
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
    Composition { owner: ModuleId, module: ModuleId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioNode {
    key: AudioKey,
    id: AudioModuleId,
}

struct OutputDependencies(Vec<ModuleId>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Primitive,
    Freq,
    Gate,
    Degree,
    DegreeGate,
    Rate,
    Transpose,
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
    CompositionInput,
    CompositionOutput,
    Composition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleCategory {
    Source,
    Shape,
    Filter,
    Effect,
    Logic,
    Routing,
    Composition,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteModule {
    kind: ModuleKind,
    name: String,
    user: Option<usize>,
}

impl PaletteModule {
    pub fn kind(&self) -> ModuleKind {
        self.kind
    }

    pub fn label(&self) -> &str {
        &self.name
    }

    pub fn category(&self) -> ModuleCategory {
        self.kind.category()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    id: ModuleId,
    position: GridPos,
    orientation: Orientation,
    body: ModuleBody,
    disabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum ModuleBody {
    Primitive(AudioModule),
    Freq,
    Gate,
    Degree,
    DegreeGate {
        degree: IntParam,
    },
    Rate {
        time: TimeParam,
    },
    Transpose {
        input: InputParam,
        semitones: FloatParam,
    },
    Osc {
        wave: EnumParam,
        frequency: FloatParam,
    },
    Rise {
        gate: InputParam,
        time: TimeParam,
    },
    Fall {
        gate: InputParam,
        time: TimeParam,
    },
    Ramp {
        value: FloatParam,
        time: TimeParam,
    },
    Adsr {
        rise: InputParam,
        fall: InputParam,
        attack: FloatParam,
        sustain: FloatParam,
    },
    Envelope {
        phase: FloatParam,
        points: Vec<EnvPoint>,
    },
    Lowpass {
        input: FloatParam,
        frequency: FloatParam,
        q: FloatParam,
    },
    Highpass {
        input: FloatParam,
        frequency: FloatParam,
        q: FloatParam,
    },
    Comb {
        input: FloatParam,
        time: TimeParam,
        feedback: FloatParam,
        damp: FloatParam,
    },
    Allpass {
        input: FloatParam,
        time: TimeParam,
        feedback: FloatParam,
    },
    Delay {
        input: FloatParam,
        time: TimeParam,
    },
    DelayTap {
        source: DelaySourceParam,
        gain: FloatParam,
    },
    Reverb {
        input: FloatParam,
        room: FloatParam,
        damp: FloatParam,
        mod_depth: FloatParam,
        diffusion: FloatParam,
    },
    Distortion {
        input: FloatParam,
        kind: EnumParam,
        drive: FloatParam,
        asymmetry: FloatParam,
    },
    Compressor {
        input: FloatParam,
        threshold: FloatParam,
        ratio: FloatParam,
        attack: FloatParam,
        release: FloatParam,
        makeup: FloatParam,
    },
    Flanger {
        input: FloatParam,
        rate: FloatParam,
        depth: FloatParam,
        feedback: FloatParam,
    },
    Multiply {
        a: FloatParam,
        b: FloatParam,
    },
    Add {
        a: FloatParam,
        b: FloatParam,
    },
    GreaterThan {
        a: FloatParam,
        b: FloatParam,
    },
    LessThan {
        a: FloatParam,
        b: FloatParam,
    },
    Switch {
        select: InputParam,
        a: FloatParam,
        b: FloatParam,
    },
    Random {
        gate: InputParam,
    },
    Sample {
        file_name: String,
        file_missing: bool,
        position: InputParam,
    },
    Probe {
        input: FloatParam,
    },
    Output {
        input: InputParam,
        gain: FloatParam,
    },
    CompositionOutput {
        label: String,
        input: InputParam,
    },
    TurnRightDown,
    TurnDownRight,
    LeftSplit,
    TopSplit,
    RightJoin,
    DownJoin,
    CompositionInput {
        label: String,
        kind: AudioInputKind,
        value: FloatParam,
    },
    Composition {
        name: String,
        surface: PatchSurface,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FloatParam {
    value: i32,
    min: i32,
    max: i32,
    step: i32,
    connected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IntParam {
    value: i32,
    min: i32,
    max: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TimeParam {
    value: i32,
    unit: TimeUnit,
    denominator: i32,
    connected: bool,
    exact_seconds: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InputParam {
    connected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EnumParam {
    index: usize,
    options: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DelaySourceParam {
    selected: Option<ModuleId>,
    options: Vec<ModuleId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleParameter {
    name: String,
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
    Bars {
        numerator: i32,
        denominator: i32,
    },
    Input,
    File {
        path: String,
        missing: bool,
    },
    Enum {
        index: usize,
        options: Vec<String>,
    },
    Text(String),
}

pub(crate) struct GridRenderModule<'a> {
    pub module: &'a Module,
    pub width: u16,
    pub height: u16,
    pub input_count: u16,
    pub output_count: u16,
    pub input_labels: Vec<char>,
    pub input_connected: Vec<bool>,
    pub has_input_top: bool,
    pub has_input_left: bool,
    pub has_output_bottom: bool,
    pub has_output_right: bool,
    pub left_input_offsets: Vec<Option<u16>>,
    pub top_input_offsets: Vec<Option<u16>>,
    pub right_output_offsets: Vec<Option<u16>>,
    pub bottom_output_offsets: Vec<Option<u16>>,
    pub probe_value: Option<f32>,
    pub meter_values: Vec<f32>,
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

fn float_parameter(name: impl Into<String>, param: FloatParam) -> ModuleParameter {
    ModuleParameter {
        name: name.into(),
        value: ParameterValue::Float {
            value: param.value,
            min: param.min,
            max: param.max,
            step: param.step,
        },
        connected: param.connected,
    }
}

fn int_parameter(name: &'static str, param: IntParam) -> ModuleParameter {
    ModuleParameter {
        name: name.to_string(),
        value: ParameterValue::Int {
            value: param.value,
            min: param.min,
            max: param.max,
        },
        connected: false,
    }
}

fn time_parameter(name: &'static str, param: TimeParam) -> ModuleParameter {
    let value = if param.unit == TimeUnit::Bars {
        ParameterValue::Bars {
            numerator: param.value,
            denominator: param.denominator,
        }
    } else {
        ParameterValue::Time {
            value: param.value,
            unit: param.unit,
        }
    };
    ModuleParameter {
        name: name.to_string(),
        value,
        connected: param.connected,
    }
}

fn input_parameter(name: &'static str, param: InputParam) -> ModuleParameter {
    ModuleParameter {
        name: name.to_string(),
        value: ParameterValue::Input,
        connected: param.connected,
    }
}

fn file_parameter(name: &'static str, path: &str, missing: bool) -> ModuleParameter {
    ModuleParameter {
        name: name.to_string(),
        value: ParameterValue::File {
            path: path.to_string(),
            missing,
        },
        connected: false,
    }
}

fn mark_missing_sample(module: &mut Module, base: &Path, missing: &mut Vec<String>) {
    let ModuleBody::Sample {
        file_name,
        file_missing,
        ..
    } = &mut module.body
    else {
        return;
    };
    let path = Path::new(file_name);
    *file_missing = file_name != "none"
        && !(if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        })
        .is_file();
    if *file_missing && !missing.contains(file_name) {
        missing.push(file_name.clone());
    }
}

fn mark_missing_samples_in_surface(
    surface: &mut PatchSurface,
    base: &Path,
    missing: &mut Vec<String>,
) {
    for module in &mut surface.modules {
        mark_missing_sample(module, base, missing);
        if let Some(composition) = module.composition_surface_mut() {
            mark_missing_samples_in_surface(composition, base, missing);
        }
    }
}

fn enum_parameter(name: &'static str, param: EnumParam) -> ModuleParameter {
    ModuleParameter {
        name: name.to_string(),
        value: ParameterValue::Enum {
            index: param.index,
            options: param.options,
        },
        connected: false,
    }
}

fn delay_source_parameter(name: &'static str, param: &DelaySourceParam) -> ModuleParameter {
    ModuleParameter {
        name: name.to_string(),
        value: ParameterValue::Enum {
            index: param
                .selected
                .and_then(|selected| param.options.iter().position(|id| *id == selected))
                .unwrap_or(0),
            options: (1..=param.options.len())
                .map(|index| format!("delay {index}"))
                .collect(),
        },
        connected: false,
    }
}

fn float_param(min: i32, max: i32, step: i32, value: i32) -> FloatParam {
    FloatParam {
        value,
        min,
        max,
        step,
        connected: true,
    }
}

fn int_param(min: i32, max: i32, value: i32) -> IntParam {
    IntParam { value, min, max }
}

fn time_param(value: i32, unit: TimeUnit) -> TimeParam {
    TimeParam {
        value,
        unit,
        denominator: 4,
        connected: true,
        exact_seconds: None,
    }
}

fn input_param() -> InputParam {
    InputParam { connected: true }
}

fn enum_param(options: &[&str], index: usize) -> EnumParam {
    EnumParam {
        index,
        options: options.iter().map(|option| (*option).to_string()).collect(),
    }
}

#[derive(Debug)]
pub struct GuiState {
    grid_size: GridSize,
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
    save_requested: bool,
    load_after_save: bool,
    relink_sample_request: Option<ModuleId>,
    export_loops: u16,
    open_modules_requested: bool,
    user_compositions: Vec<brainwash::patch::Composition>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    pub(crate) play_button: ButtonState,
    pub(crate) meters_button: ButtonState,
    pub(crate) load_button: ButtonState,
    pub(crate) save_button: ButtonState,
    pub(crate) export_button: ButtonState,
    pub(crate) modules_button: ButtonState,
    pub(crate) track_button: ButtonState,
    pub(crate) cancel_button: ButtonState,
    pub(crate) confirm_button: ButtonState,
    pub(crate) discard_button: ButtonState,
    pub(crate) sample_button: ButtonState,
    pointer_drag: Option<PointerDrag>,
    held_move: Option<HeldMove>,
    held_selection: Option<HeldSelection>,
    audio: Option<AudioHandle>,
    audio_status: String,
    document_status: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Instrument {
    root: PatchSurface,
    track_text: String,
    editing_composition: Option<ModuleId>,
    composition_stack: Vec<(Option<ModuleId>, GridPos)>,
}

#[derive(Clone, Debug, PartialEq)]
struct PatchSurface {
    cursor: GridPos,
    modules: Vec<Module>,
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    instruments: Vec<Instrument>,
    active_instrument: usize,
    next_module_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct HeldMove {
    module: Module,
    origin_surface: Option<ModuleId>,
    origin_stack: Vec<(Option<ModuleId>, GridPos)>,
    origin: GridPos,
    before: Snapshot,
}

#[derive(Clone, Debug, PartialEq)]
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

impl Connection {
    pub fn from(self) -> ModuleId {
        self.from
    }

    pub fn to(self) -> ModuleId {
        self.to
    }

    pub(crate) fn output(self) -> usize {
        self.output
    }

    pub(crate) fn input(self) -> usize {
        self.input.index
    }

    fn audio_input(self) -> AudioInputKind {
        self.input.audio
    }
}

impl ModuleKind {
    pub fn category(self) -> ModuleCategory {
        match self {
            ModuleKind::Primitive => ModuleCategory::Routing,
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::DegreeGate
            | ModuleKind::Rate
            | ModuleKind::Transpose
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
            ModuleKind::CompositionInput
            | ModuleKind::CompositionOutput
            | ModuleKind::Composition => ModuleCategory::Composition,
            ModuleKind::Output => ModuleCategory::Output,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ModuleKind::Primitive => "Unit",
            ModuleKind::Freq => "Freq",
            ModuleKind::Gate => "Gate",
            ModuleKind::Degree => "Degree",
            ModuleKind::DegreeGate => "Degree Gate",
            ModuleKind::Rate => "Rate",
            ModuleKind::Transpose => "Transpose",
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
            ModuleKind::CompositionInput => "Sub In",
            ModuleKind::CompositionOutput => "Sub Out",
            ModuleKind::Composition => "Composition",
        }
    }

    pub(crate) fn has_visual_editor(self) -> bool {
        self.special_editor().is_some()
    }

    fn default_body(self) -> ModuleBody {
        match self {
            ModuleKind::Primitive => ModuleBody::Primitive(AudioModule::Pass),
            ModuleKind::Freq => ModuleBody::Freq,
            ModuleKind::Gate => ModuleBody::Gate,
            ModuleKind::Degree => ModuleBody::Degree,
            ModuleKind::DegreeGate => ModuleBody::DegreeGate {
                degree: int_param(0, 12, 0),
            },
            ModuleKind::Rate => ModuleBody::Rate {
                time: time_param(100, TimeUnit::Seconds),
            },
            ModuleKind::Transpose => ModuleBody::Transpose {
                input: input_param(),
                semitones: float_param(-2400, 2400, 100, 0),
            },
            ModuleKind::Osc => ModuleBody::Osc {
                wave: enum_param(&["sin", "square", "tri", "saw", "rsaw", "noise"], 0),
                frequency: float_param(1, 200_000, 100, 44_000),
            },
            ModuleKind::Rise => ModuleBody::Rise {
                gate: input_param(),
                time: time_param(10, TimeUnit::Seconds),
            },
            ModuleKind::Fall => ModuleBody::Fall {
                gate: input_param(),
                time: time_param(10, TimeUnit::Seconds),
            },
            ModuleKind::Ramp => ModuleBody::Ramp {
                value: float_param(-100_000, 100_000, 10, 0),
                time: time_param(10, TimeUnit::Seconds),
            },
            ModuleKind::Adsr => ModuleBody::Adsr {
                rise: input_param(),
                fall: input_param(),
                attack: float_param(0, 100, 5, 25),
                sustain: float_param(0, 100, 5, 70),
            },
            ModuleKind::Envelope => ModuleBody::Envelope {
                phase: float_param(0, 100, 1, 0),
                points: vec![
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
                ],
            },
            ModuleKind::Lowpass => ModuleBody::Lowpass {
                input: float_param(-100, 100, 1, 0),
                frequency: float_param(0, 99, 1, 25),
                q: float_param(10, 1000, 10, 70),
            },
            ModuleKind::Highpass => ModuleBody::Highpass {
                input: float_param(-100, 100, 1, 0),
                frequency: float_param(0, 100, 1, 25),
                q: float_param(10, 1000, 10, 70),
            },
            ModuleKind::Comb => ModuleBody::Comb {
                input: float_param(-100, 100, 1, 0),
                time: time_param(441, TimeUnit::Samples),
                feedback: float_param(0, 99, 1, 30),
                damp: float_param(0, 100, 1, 20),
            },
            ModuleKind::Allpass => ModuleBody::Allpass {
                input: float_param(-100, 100, 1, 0),
                time: time_param(441, TimeUnit::Samples),
                feedback: float_param(0, 99, 1, 50),
            },
            ModuleKind::Delay => ModuleBody::Delay {
                input: float_param(-100, 100, 1, 0),
                time: time_param(4410, TimeUnit::Samples),
            },
            ModuleKind::DelayTap => ModuleBody::DelayTap {
                source: DelaySourceParam {
                    selected: None,
                    options: Vec::new(),
                },
                gain: float_param(0, 70, 5, 50),
            },
            ModuleKind::Reverb => ModuleBody::Reverb {
                input: float_param(-100, 100, 1, 0),
                room: float_param(0, 100, 5, 50),
                damp: float_param(0, 100, 5, 30),
                mod_depth: float_param(0, 100, 5, 50),
                diffusion: float_param(0, 100, 5, 50),
            },
            ModuleKind::Distortion => ModuleBody::Distortion {
                input: float_param(-100, 100, 1, 0),
                kind: enum_param(&["tube", "tape", "fuzz", "fold", "clip"], 0),
                drive: float_param(10, 2000, 10, 200),
                asymmetry: float_param(-100, 100, 5, 0),
            },
            ModuleKind::Compressor => ModuleBody::Compressor {
                input: float_param(-100, 100, 1, 0),
                threshold: float_param(1, 100, 1, 50),
                ratio: float_param(100, 2000, 50, 400),
                attack: float_param(0, 50, 1, 1),
                release: float_param(1, 200, 1, 30),
                makeup: float_param(0, 400, 10, 100),
            },
            ModuleKind::Flanger => ModuleBody::Flanger {
                input: float_param(-100, 100, 1, 0),
                rate: float_param(10, 1000, 10, 50),
                depth: float_param(0, 100, 5, 50),
                feedback: float_param(0, 95, 5, 35),
            },
            ModuleKind::Multiply => ModuleBody::Multiply {
                a: float_param(0, 10_000, 5, 100),
                b: float_param(0, 10_000, 5, 100),
            },
            ModuleKind::Add => ModuleBody::Add {
                a: float_param(-100_000, 100_000, 5, 0),
                b: float_param(-100_000, 100_000, 5, 0),
            },
            ModuleKind::GreaterThan => ModuleBody::GreaterThan {
                a: float_param(-100_000, 100_000, 5, 0),
                b: float_param(-100_000, 100_000, 5, 0),
            },
            ModuleKind::LessThan => ModuleBody::LessThan {
                a: float_param(-100_000, 100_000, 5, 0),
                b: float_param(-100_000, 100_000, 5, 0),
            },
            ModuleKind::Switch => ModuleBody::Switch {
                select: input_param(),
                a: float_param(-100_000, 100_000, 10, 0),
                b: float_param(-100_000, 100_000, 10, 100),
            },
            ModuleKind::Random => ModuleBody::Random {
                gate: input_param(),
            },
            ModuleKind::Sample => ModuleBody::Sample {
                file_name: "none".to_string(),
                file_missing: false,
                position: input_param(),
            },
            ModuleKind::Probe => ModuleBody::Probe {
                input: float_param(-100, 100, 1, 0),
            },
            ModuleKind::Output => ModuleBody::Output {
                input: input_param(),
                gain: float_param(0, 100, 1, 100),
            },
            ModuleKind::CompositionOutput => ModuleBody::CompositionOutput {
                label: "Output".to_string(),
                input: input_param(),
            },
            ModuleKind::TurnRightDown => ModuleBody::TurnRightDown,
            ModuleKind::TurnDownRight => ModuleBody::TurnDownRight,
            ModuleKind::LeftSplit => ModuleBody::LeftSplit,
            ModuleKind::TopSplit => ModuleBody::TopSplit,
            ModuleKind::RightJoin => ModuleBody::RightJoin,
            ModuleKind::DownJoin => ModuleBody::DownJoin,
            ModuleKind::CompositionInput => ModuleBody::CompositionInput {
                label: "Input".to_string(),
                kind: AudioInputKind::A,
                value: float_param(-100_000, 100_000, 1, 0),
            },
            ModuleKind::Composition => ModuleBody::Composition {
                name: "Composition".to_string(),
                surface: PatchSurface::new(),
            },
        }
    }

    fn special_editor(self) -> Option<SpecialEditor> {
        match self {
            ModuleKind::Primitive => None,
            ModuleKind::Adsr => Some(SpecialEditor::Adsr),
            ModuleKind::Envelope => Some(SpecialEditor::Envelope),
            ModuleKind::Probe => Some(SpecialEditor::Probe),
            ModuleKind::Sample => Some(SpecialEditor::Sample),
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::DegreeGate
            | ModuleKind::Rate
            | ModuleKind::Transpose
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
            | ModuleKind::CompositionInput
            | ModuleKind::CompositionOutput
            | ModuleKind::Composition => None,
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

impl ModuleBody {
    fn duplicate(&self, next_module_id: &mut u32) -> Self {
        match self {
            ModuleBody::Primitive(_) => self.clone(),
            Self::Composition { name, surface } => {
                let mut duplicate = PatchSurface {
                    cursor: surface.cursor,
                    modules: surface
                        .modules
                        .iter()
                        .map(|module| module.duplicate(module.position, next_module_id))
                        .collect(),
                };
                let ids = surface
                    .modules
                    .iter()
                    .zip(&duplicate.modules)
                    .map(|(old, new)| (old.id, new.id))
                    .collect::<Vec<_>>();
                for (old, new) in surface.modules.iter().zip(&mut duplicate.modules) {
                    let ModuleBody::DelayTap { source: old, .. } = &old.body else {
                        continue;
                    };
                    let ModuleBody::DelayTap { source: new, .. } = &mut new.body else {
                        continue;
                    };
                    new.selected = old.selected.and_then(|selected| {
                        ids.iter()
                            .find_map(|(old, new)| (*old == selected).then_some(*new))
                    });
                }
                sync_delay_sources(&mut duplicate);
                Self::Composition {
                    name: name.clone(),
                    surface: duplicate,
                }
            }
            _ => self.clone(),
        }
    }

    fn kind(&self) -> ModuleKind {
        match self {
            ModuleBody::Primitive(_) => ModuleKind::Primitive,
            ModuleBody::Freq => ModuleKind::Freq,
            ModuleBody::Gate => ModuleKind::Gate,
            ModuleBody::Degree => ModuleKind::Degree,
            ModuleBody::DegreeGate { .. } => ModuleKind::DegreeGate,
            ModuleBody::Rate { .. } => ModuleKind::Rate,
            ModuleBody::Transpose { .. } => ModuleKind::Transpose,
            ModuleBody::Osc { .. } => ModuleKind::Osc,
            ModuleBody::Rise { .. } => ModuleKind::Rise,
            ModuleBody::Fall { .. } => ModuleKind::Fall,
            ModuleBody::Ramp { .. } => ModuleKind::Ramp,
            ModuleBody::Adsr { .. } => ModuleKind::Adsr,
            ModuleBody::Envelope { .. } => ModuleKind::Envelope,
            ModuleBody::Lowpass { .. } => ModuleKind::Lowpass,
            ModuleBody::Highpass { .. } => ModuleKind::Highpass,
            ModuleBody::Comb { .. } => ModuleKind::Comb,
            ModuleBody::Allpass { .. } => ModuleKind::Allpass,
            ModuleBody::Delay { .. } => ModuleKind::Delay,
            ModuleBody::DelayTap { .. } => ModuleKind::DelayTap,
            ModuleBody::Reverb { .. } => ModuleKind::Reverb,
            ModuleBody::Distortion { .. } => ModuleKind::Distortion,
            ModuleBody::Compressor { .. } => ModuleKind::Compressor,
            ModuleBody::Flanger { .. } => ModuleKind::Flanger,
            ModuleBody::Multiply { .. } => ModuleKind::Multiply,
            ModuleBody::Add { .. } => ModuleKind::Add,
            ModuleBody::GreaterThan { .. } => ModuleKind::GreaterThan,
            ModuleBody::LessThan { .. } => ModuleKind::LessThan,
            ModuleBody::Switch { .. } => ModuleKind::Switch,
            ModuleBody::Random { .. } => ModuleKind::Random,
            ModuleBody::Sample { .. } => ModuleKind::Sample,
            ModuleBody::Probe { .. } => ModuleKind::Probe,
            ModuleBody::Output { .. } => ModuleKind::Output,
            ModuleBody::CompositionOutput { .. } => ModuleKind::CompositionOutput,
            ModuleBody::TurnRightDown => ModuleKind::TurnRightDown,
            ModuleBody::TurnDownRight => ModuleKind::TurnDownRight,
            ModuleBody::LeftSplit => ModuleKind::LeftSplit,
            ModuleBody::TopSplit => ModuleKind::TopSplit,
            ModuleBody::RightJoin => ModuleKind::RightJoin,
            ModuleBody::DownJoin => ModuleKind::DownJoin,
            ModuleBody::CompositionInput { .. } => ModuleKind::CompositionInput,
            ModuleBody::Composition { .. } => ModuleKind::Composition,
        }
    }

    fn parameters(&self) -> Vec<ModuleParameter> {
        match self {
            ModuleBody::Primitive(_) => Vec::new(),
            ModuleBody::Freq | ModuleBody::Gate | ModuleBody::Degree => Vec::new(),
            ModuleBody::DegreeGate { degree } => vec![int_parameter("Deg", *degree)],
            ModuleBody::Rate { time } => vec![time_parameter("Time", *time)],
            ModuleBody::Transpose { input, semitones } => vec![
                input_parameter("In", *input),
                float_parameter("St", *semitones),
            ],
            ModuleBody::Osc { wave, frequency } => vec![
                enum_parameter("Wave", wave.clone()),
                float_parameter("Hz", *frequency),
            ],
            ModuleBody::Rise { gate, time } | ModuleBody::Fall { gate, time } => {
                vec![
                    input_parameter("Gate", *gate),
                    time_parameter("Time", *time),
                ]
            }
            ModuleBody::Ramp { value, time } => {
                vec![
                    float_parameter("Val", *value),
                    time_parameter("Time", *time),
                ]
            }
            ModuleBody::Adsr {
                rise,
                fall,
                attack,
                sustain,
            } => vec![
                input_parameter("Rise", *rise),
                input_parameter("Fall", *fall),
                float_parameter("Atk", *attack),
                float_parameter("Sus", *sustain),
            ],
            ModuleBody::Envelope { phase, .. } => vec![float_parameter("Phase", *phase)],
            ModuleBody::Lowpass {
                input,
                frequency,
                q,
            }
            | ModuleBody::Highpass {
                input,
                frequency,
                q,
            } => vec![
                float_parameter("In", *input),
                float_parameter("Freq", *frequency),
                float_parameter("Q", *q),
            ],
            ModuleBody::Comb {
                input,
                time,
                feedback,
                damp,
            } => vec![
                float_parameter("In", *input),
                time_parameter("Time", *time),
                float_parameter("Fdbk", *feedback),
                float_parameter("Damp", *damp),
            ],
            ModuleBody::Allpass {
                input,
                time,
                feedback,
            } => vec![
                float_parameter("In", *input),
                time_parameter("Time", *time),
                float_parameter("Fdbk", *feedback),
            ],
            ModuleBody::Delay { input, time } => {
                vec![float_parameter("In", *input), time_parameter("Time", *time)]
            }
            ModuleBody::DelayTap { source, gain } => vec![
                delay_source_parameter("Src", source),
                float_parameter("Gain", *gain),
            ],
            ModuleBody::Reverb {
                input,
                room,
                damp,
                mod_depth,
                diffusion,
            } => vec![
                float_parameter("In", *input),
                float_parameter("Room", *room),
                float_parameter("Damp", *damp),
                float_parameter("Mod", *mod_depth),
                float_parameter("Diff", *diffusion),
            ],
            ModuleBody::Distortion {
                input,
                kind,
                drive,
                asymmetry,
            } => vec![
                float_parameter("In", *input),
                enum_parameter("Type", kind.clone()),
                float_parameter("Drive", *drive),
                float_parameter("Asym", *asymmetry),
            ],
            ModuleBody::Compressor {
                input,
                threshold,
                ratio,
                attack,
                release,
                makeup,
            } => vec![
                float_parameter("In", *input),
                float_parameter("Thresh", *threshold),
                float_parameter("Ratio", *ratio),
                float_parameter("Atk", *attack),
                float_parameter("Rel", *release),
                float_parameter("Gain", *makeup),
            ],
            ModuleBody::Flanger {
                input,
                rate,
                depth,
                feedback,
            } => vec![
                float_parameter("In", *input),
                float_parameter("Rate", *rate),
                float_parameter("Depth", *depth),
                float_parameter("Fdbk", *feedback),
            ],
            ModuleBody::Multiply { a, b } => {
                vec![float_parameter("A", *a), float_parameter("B", *b)]
            }
            ModuleBody::Add { a, b }
            | ModuleBody::GreaterThan { a, b }
            | ModuleBody::LessThan { a, b } => {
                vec![float_parameter("A", *a), float_parameter("B", *b)]
            }
            ModuleBody::Switch { select, a, b } => vec![
                input_parameter("Sel", *select),
                float_parameter("A", *a),
                float_parameter("B", *b),
            ],
            ModuleBody::Random { gate } => vec![input_parameter("Gate", *gate)],
            ModuleBody::Sample {
                file_name,
                file_missing,
                position,
            } => {
                vec![
                    file_parameter("File", file_name, *file_missing),
                    input_parameter("Pos", *position),
                ]
            }
            ModuleBody::Probe { input } => vec![float_parameter("In", *input)],
            ModuleBody::Output { input, gain } => {
                vec![
                    input_parameter("In", *input),
                    float_parameter("Gain", *gain),
                ]
            }
            ModuleBody::CompositionOutput { label, input } => vec![
                ModuleParameter {
                    name: "Label".to_string(),
                    value: ParameterValue::Text(label.clone()),
                    connected: false,
                },
                input_parameter("In", *input),
            ],
            ModuleBody::TurnRightDown
            | ModuleBody::TurnDownRight
            | ModuleBody::LeftSplit
            | ModuleBody::TopSplit
            | ModuleBody::RightJoin
            | ModuleBody::DownJoin => Vec::new(),
            ModuleBody::Composition { name, surface } => {
                let mut parameters = vec![ModuleParameter {
                    name: "Name".to_string(),
                    value: ParameterValue::Text(name.clone()),
                    connected: false,
                }];
                parameters.extend(composition_inputs(surface).into_iter().filter_map(|id| {
                    let module = surface.modules.iter().find(|module| module.id == id)?;
                    let ModuleBody::CompositionInput { label, value, .. } = &module.body else {
                        return None;
                    };
                    Some(float_parameter(label.clone(), *value))
                }));
                parameters
            }
            ModuleBody::CompositionInput { label, value, .. } => vec![
                ModuleParameter {
                    name: "Label".to_string(),
                    value: ParameterValue::Text(label.clone()),
                    connected: false,
                },
                float_parameter("Value", *value),
            ],
        }
    }

    fn parameter(&self, index: usize) -> Option<ModuleParameter> {
        self.parameters().get(index).cloned()
    }

    fn parameter_count(&self) -> usize {
        self.parameters().len()
    }

    fn input_count(&self) -> u16 {
        if let ModuleBody::Primitive(module) = self {
            return module.input_kinds().len() as u16;
        }
        self.audio_inputs().len() as u16
    }

    fn input_connected(&self, port: u16) -> bool {
        self.parameters()
            .into_iter()
            .filter(|parameter| parameter.value.is_port())
            .nth(port as usize)
            .map(|parameter| parameter.connected)
            .unwrap_or(true)
    }

    fn audio_inputs(&self) -> &'static [AudioInputKind] {
        match self {
            ModuleBody::Primitive(_) => &[],
            ModuleBody::Freq
            | ModuleBody::Gate
            | ModuleBody::Degree
            | ModuleBody::DegreeGate { .. } => &[],
            ModuleBody::Rate { .. } => &[],
            ModuleBody::Transpose { .. } => &[AudioInputKind::In, AudioInputKind::Semitones],
            ModuleBody::Osc { .. } => &[AudioInputKind::Freq],
            ModuleBody::Rise { .. } | ModuleBody::Fall { .. } => {
                &[AudioInputKind::Gate, AudioInputKind::Time]
            }
            ModuleBody::Ramp { .. } => &[AudioInputKind::Value, AudioInputKind::Time],
            ModuleBody::Adsr { .. } => &[
                AudioInputKind::Rise,
                AudioInputKind::Fall,
                AudioInputKind::Attack,
                AudioInputKind::Sustain,
            ],
            ModuleBody::Envelope { .. } => &[AudioInputKind::Phase],
            ModuleBody::Lowpass { .. } | ModuleBody::Highpass { .. } => {
                &[AudioInputKind::In, AudioInputKind::Freq, AudioInputKind::Q]
            }
            ModuleBody::Comb { .. } => &[
                AudioInputKind::In,
                AudioInputKind::Time,
                AudioInputKind::Feedback,
                AudioInputKind::Damp,
            ],
            ModuleBody::Allpass { .. } => &[
                AudioInputKind::In,
                AudioInputKind::Time,
                AudioInputKind::Feedback,
            ],
            ModuleBody::Delay { .. } => &[AudioInputKind::Time, AudioInputKind::In],
            ModuleBody::DelayTap { .. } => &[],
            ModuleBody::Reverb { .. } => &[
                AudioInputKind::In,
                AudioInputKind::Room,
                AudioInputKind::Damp,
                AudioInputKind::Mod,
                AudioInputKind::Diff,
            ],
            ModuleBody::Distortion { .. } => &[
                AudioInputKind::In,
                AudioInputKind::Drive,
                AudioInputKind::Asym,
            ],
            ModuleBody::Compressor { .. } => &[
                AudioInputKind::In,
                AudioInputKind::Thresh,
                AudioInputKind::Ratio,
                AudioInputKind::Attack,
                AudioInputKind::Release,
                AudioInputKind::Gain,
            ],
            ModuleBody::Flanger { .. } => &[
                AudioInputKind::In,
                AudioInputKind::Rate,
                AudioInputKind::Depth,
                AudioInputKind::Feedback,
            ],
            ModuleBody::Multiply { .. }
            | ModuleBody::Add { .. }
            | ModuleBody::GreaterThan { .. }
            | ModuleBody::LessThan { .. } => &[AudioInputKind::A, AudioInputKind::B],
            ModuleBody::Switch { .. } => {
                &[AudioInputKind::Select, AudioInputKind::A, AudioInputKind::B]
            }
            ModuleBody::Random { .. } => &[AudioInputKind::Gate],
            ModuleBody::Sample { .. } => &[AudioInputKind::Position],
            ModuleBody::Probe { .. } => &[AudioInputKind::In],
            ModuleBody::Output { .. } => &[AudioInputKind::A, AudioInputKind::B],
            ModuleBody::CompositionOutput { .. } => &[AudioInputKind::In],
            ModuleBody::TurnRightDown
            | ModuleBody::TurnDownRight
            | ModuleBody::LeftSplit
            | ModuleBody::TopSplit => &[AudioInputKind::In],
            ModuleBody::CompositionInput { .. } | ModuleBody::Composition { .. } => &[],
            ModuleBody::RightJoin | ModuleBody::DownJoin => &[AudioInputKind::A, AudioInputKind::B],
        }
    }

    fn env_points(&self) -> &[EnvPoint] {
        match self {
            ModuleBody::Envelope { points, .. } => points,
            _ => &[],
        }
    }

    fn env_points_mut(&mut self) -> Option<&mut Vec<EnvPoint>> {
        match self {
            ModuleBody::Envelope { points, .. } => Some(points),
            _ => None,
        }
    }

    fn set_parameter(&mut self, index: usize, parameter: ModuleParameter) -> bool {
        match self {
            ModuleBody::Primitive(_) => false,
            ModuleBody::DegreeGate { degree } => set_int_param(degree, index, 0, &parameter),
            ModuleBody::Rate { time } => set_time_param(time, index, 0, &parameter),
            ModuleBody::Transpose { input, semitones } => {
                set_input_param(input, index, 0, &parameter)
                    || set_float_param(semitones, index, 1, &parameter)
            }
            ModuleBody::Osc { wave, frequency } => {
                set_enum_param(wave, index, 0, &parameter)
                    || set_float_param(frequency, index, 1, &parameter)
            }
            ModuleBody::Rise { gate, time } | ModuleBody::Fall { gate, time } => {
                set_input_param(gate, index, 0, &parameter)
                    || set_time_param(time, index, 1, &parameter)
            }
            ModuleBody::Ramp { value, time } => {
                set_float_param(value, index, 0, &parameter)
                    || set_time_param(time, index, 1, &parameter)
            }
            ModuleBody::Adsr {
                rise,
                fall,
                attack,
                sustain,
            } => {
                set_input_param(rise, index, 0, &parameter)
                    || set_input_param(fall, index, 1, &parameter)
                    || set_float_param(attack, index, 2, &parameter)
                    || set_float_param(sustain, index, 3, &parameter)
            }
            ModuleBody::Envelope { phase, .. } => set_float_param(phase, index, 0, &parameter),
            ModuleBody::Lowpass {
                input,
                frequency,
                q,
            }
            | ModuleBody::Highpass {
                input,
                frequency,
                q,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_float_param(frequency, index, 1, &parameter)
                    || set_float_param(q, index, 2, &parameter)
            }
            ModuleBody::Comb {
                input,
                time,
                feedback,
                damp,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_time_param(time, index, 1, &parameter)
                    || set_float_param(feedback, index, 2, &parameter)
                    || set_float_param(damp, index, 3, &parameter)
            }
            ModuleBody::Allpass {
                input,
                time,
                feedback,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_time_param(time, index, 1, &parameter)
                    || set_float_param(feedback, index, 2, &parameter)
            }
            ModuleBody::Delay { input, time } => {
                set_float_param(input, index, 0, &parameter)
                    || set_time_param(time, index, 1, &parameter)
            }
            ModuleBody::DelayTap { source, gain } => {
                set_delay_source_param(source, index, 0, &parameter)
                    || set_float_param(gain, index, 1, &parameter)
            }
            ModuleBody::Reverb {
                input,
                room,
                damp,
                mod_depth,
                diffusion,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_float_param(room, index, 1, &parameter)
                    || set_float_param(damp, index, 2, &parameter)
                    || set_float_param(mod_depth, index, 3, &parameter)
                    || set_float_param(diffusion, index, 4, &parameter)
            }
            ModuleBody::Distortion {
                input,
                kind,
                drive,
                asymmetry,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_enum_param(kind, index, 1, &parameter)
                    || set_float_param(drive, index, 2, &parameter)
                    || set_float_param(asymmetry, index, 3, &parameter)
            }
            ModuleBody::Compressor {
                input,
                threshold,
                ratio,
                attack,
                release,
                makeup,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_float_param(threshold, index, 1, &parameter)
                    || set_float_param(ratio, index, 2, &parameter)
                    || set_float_param(attack, index, 3, &parameter)
                    || set_float_param(release, index, 4, &parameter)
                    || set_float_param(makeup, index, 5, &parameter)
            }
            ModuleBody::Flanger {
                input,
                rate,
                depth,
                feedback,
            } => {
                set_float_param(input, index, 0, &parameter)
                    || set_float_param(rate, index, 1, &parameter)
                    || set_float_param(depth, index, 2, &parameter)
                    || set_float_param(feedback, index, 3, &parameter)
            }
            ModuleBody::Multiply { a, b }
            | ModuleBody::Add { a, b }
            | ModuleBody::GreaterThan { a, b }
            | ModuleBody::LessThan { a, b } => {
                set_float_param(a, index, 0, &parameter) || set_float_param(b, index, 1, &parameter)
            }
            ModuleBody::Switch { select, a, b } => {
                set_input_param(select, index, 0, &parameter)
                    || set_float_param(a, index, 1, &parameter)
                    || set_float_param(b, index, 2, &parameter)
            }
            ModuleBody::Random { gate } => set_input_param(gate, index, 0, &parameter),
            ModuleBody::Sample { position, .. } => set_input_param(position, index, 1, &parameter),
            ModuleBody::Probe { input } => set_float_param(input, index, 0, &parameter),
            ModuleBody::Output { input, gain } => {
                set_input_param(input, index, 0, &parameter)
                    || set_float_param(gain, index, 1, &parameter)
            }
            ModuleBody::CompositionOutput { label, input } => match index {
                0 => set_text_param(label, &parameter),
                1 => set_input_param(input, index, 1, &parameter),
                _ => false,
            },
            ModuleBody::Freq
            | ModuleBody::Gate
            | ModuleBody::Degree
            | ModuleBody::TurnRightDown
            | ModuleBody::TurnDownRight
            | ModuleBody::LeftSplit
            | ModuleBody::TopSplit
            | ModuleBody::RightJoin
            | ModuleBody::DownJoin => false,
            ModuleBody::Composition { name, surface } => {
                if index == 0 {
                    return set_text_param(name, &parameter);
                }
                let Some(id) = composition_inputs(surface).get(index - 1).copied() else {
                    return false;
                };
                let Some(module) = surface.modules.iter_mut().find(|module| module.id == id) else {
                    return false;
                };
                let ModuleBody::CompositionInput { value, .. } = &mut module.body else {
                    return false;
                };
                set_float_param(value, 0, 0, &parameter)
            }
            ModuleBody::CompositionInput { label, value, .. } => match index {
                0 => set_text_param(label, &parameter),
                1 => set_float_param(value, index, 1, &parameter),
                _ => false,
            },
        }
    }
}

fn set_float_param(
    target: &mut FloatParam,
    index: usize,
    expected: usize,
    parameter: &ModuleParameter,
) -> bool {
    if index != expected {
        return false;
    }
    let ParameterValue::Float {
        value,
        min,
        max,
        step,
    } = &parameter.value
    else {
        return false;
    };
    if *min != target.min || *max != target.max || *step != target.step {
        return false;
    }
    *target = FloatParam {
        value: (*value).clamp(target.min, target.max),
        min: target.min,
        max: target.max,
        step: target.step,
        connected: parameter.connected,
    };
    true
}

fn set_int_param(
    target: &mut IntParam,
    index: usize,
    expected: usize,
    parameter: &ModuleParameter,
) -> bool {
    if index != expected {
        return false;
    }
    let ParameterValue::Int { value, .. } = &parameter.value else {
        return false;
    };
    target.value = (*value).clamp(target.min, target.max);
    true
}

fn set_time_param(
    target: &mut TimeParam,
    index: usize,
    expected: usize,
    parameter: &ModuleParameter,
) -> bool {
    if index != expected {
        return false;
    }
    match &parameter.value {
        ParameterValue::Time { value, unit } if *unit != TimeUnit::Bars => {
            target.value = *value;
            target.unit = *unit;
            target.exact_seconds = None;
        }
        ParameterValue::Bars {
            numerator,
            denominator,
        } => {
            target.value = (*numerator).max(1);
            target.unit = TimeUnit::Bars;
            target.denominator = (*denominator).max(1);
            target.exact_seconds = None;
        }
        _ => return false,
    }
    target.connected = parameter.connected;
    true
}

fn set_input_param(
    target: &mut InputParam,
    index: usize,
    expected: usize,
    parameter: &ModuleParameter,
) -> bool {
    if index != expected || !matches!(parameter.value, ParameterValue::Input) {
        return false;
    }
    target.connected = parameter.connected;
    true
}

fn set_text_param(value: &mut String, parameter: &ModuleParameter) -> bool {
    let ParameterValue::Text(next) = &parameter.value else {
        return false;
    };
    if next.trim().is_empty() || *value == *next {
        return false;
    }
    *value = next.clone();
    true
}

fn set_enum_param(
    target: &mut EnumParam,
    index: usize,
    expected: usize,
    parameter: &ModuleParameter,
) -> bool {
    if index != expected {
        return false;
    }
    let ParameterValue::Enum { index, .. } = &parameter.value else {
        return false;
    };
    if *index < target.options.len() {
        target.index = *index;
        true
    } else {
        false
    }
}

fn set_delay_source_param(
    target: &mut DelaySourceParam,
    index: usize,
    expected: usize,
    parameter: &ModuleParameter,
) -> bool {
    if index != expected {
        return false;
    }
    let ParameterValue::Enum { index, .. } = &parameter.value else {
        return false;
    };
    let Some(selected) = target.options.get(*index).copied() else {
        return false;
    };
    target.selected = Some(selected);
    true
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
        Self::Composition,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ModuleCategory::Source => "Source",
            ModuleCategory::Shape => "Shape",
            ModuleCategory::Filter => "Filter",
            ModuleCategory::Effect => "Effect",
            ModuleCategory::Logic => "Logic",
            ModuleCategory::Routing => "Routing",
            ModuleCategory::Composition => "Composition",
            ModuleCategory::Output => "Output",
        }
    }
}

impl Module {
    fn duplicate(&self, position: GridPos, next_module_id: &mut u32) -> Self {
        let id = ModuleId::new(*next_module_id);
        *next_module_id += 1;
        Self {
            id,
            position,
            orientation: self.orientation,
            body: self.body.duplicate(next_module_id),
            disabled: false,
        }
    }

    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn kind(&self) -> ModuleKind {
        self.body.kind()
    }

    fn is_wire(&self) -> bool {
        matches!(
            self.kind(),
            ModuleKind::TurnRightDown
                | ModuleKind::TurnDownRight
                | ModuleKind::LeftSplit
                | ModuleKind::TopSplit
        )
    }

    pub fn position(&self) -> GridPos {
        self.position
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn parameters(&self) -> Vec<ModuleParameter> {
        self.body.parameters()
    }

    pub fn label(&self) -> &str {
        match &self.body {
            ModuleBody::Primitive(module) => graph_node_label(module),
            ModuleBody::Composition { name, .. } => name,
            _ => self.kind().label(),
        }
    }

    fn composition_surface(&self) -> Option<&PatchSurface> {
        let ModuleBody::Composition { surface, .. } = &self.body else {
            return None;
        };
        Some(surface)
    }

    fn composition_surface_mut(&mut self) -> Option<&mut PatchSurface> {
        let ModuleBody::Composition { surface, .. } = &mut self.body else {
            return None;
        };
        Some(surface)
    }

    fn is_composition(&self) -> bool {
        matches!(self.body, ModuleBody::Composition { .. })
    }

    pub fn env_points(&self) -> &[EnvPoint] {
        self.body.env_points()
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    fn parameter_count(&self) -> usize {
        self.body.parameter_count()
    }

    fn parameter(&self, index: usize) -> Option<ModuleParameter> {
        self.body.parameter(index)
    }

    fn set_parameter(&mut self, index: usize, parameter: ModuleParameter) -> bool {
        self.body.set_parameter(index, parameter)
    }

    fn env_points_mut(&mut self) -> Option<&mut Vec<EnvPoint>> {
        self.body.env_points_mut()
    }

    pub(crate) fn input_count(&self) -> u16 {
        if self.kind().is_routing() {
            return self.kind().input_count();
        }
        self.body.input_count()
    }

    pub(crate) fn input_connected(&self, port: u16) -> bool {
        self.body.input_connected(port)
    }

    pub(crate) fn output_count(&self) -> u16 {
        self.kind().output_count()
    }

    pub(crate) fn has_input_top(&self) -> bool {
        if self.input_count() == 0 {
            return false;
        }
        match self.kind() {
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
        match self.kind() {
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
        match self.kind() {
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
        match self.kind() {
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
    pub fn name(&self) -> &str {
        &self.name
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
                TimeUnit::Bars => "1/4".to_string(),
                TimeUnit::Hertz => format!("{value} hz"),
            },
            ParameterValue::Bars {
                numerator,
                denominator,
            } => format!("{}/{}", numerator.max(&1), denominator.max(&1)),
            ParameterValue::Input => "input".to_string(),
            ParameterValue::File { path, missing } => {
                if *missing {
                    format!("missing: {path}")
                } else {
                    path.clone()
                }
            }
            ParameterValue::Enum { index, options } => {
                options.get(*index).cloned().unwrap_or_default()
            }
            ParameterValue::Text(value) => value.clone(),
        }
    }

    fn accepts_typed_value(&self) -> bool {
        matches!(
            self,
            ParameterValue::Float { .. }
                | ParameterValue::Time { .. }
                | ParameterValue::Bars { .. }
                | ParameterValue::Text(_)
        )
    }

    fn is_port(&self) -> bool {
        matches!(
            self,
            ParameterValue::Float { .. }
                | ParameterValue::Time { .. }
                | ParameterValue::Bars { .. }
                | ParameterValue::Input
        )
    }

    fn input_text(&self) -> String {
        match self {
            ParameterValue::Float { value, .. } => format!("{}", *value as f32 / 100.0),
            ParameterValue::Time { value, unit } => match unit {
                TimeUnit::Seconds => format!("{}", *value as f32 / 100.0),
                TimeUnit::Samples | TimeUnit::Hertz => value.to_string(),
                TimeUnit::Bars => "1/4".to_string(),
            },
            ParameterValue::Bars {
                numerator,
                denominator,
            } => format!("{}/{}", numerator.max(&1), denominator.max(&1)),
            ParameterValue::Int { value, .. } => value.to_string(),
            ParameterValue::Input => String::new(),
            ParameterValue::File { path, .. } => path.clone(),
            ParameterValue::Enum { index, options } => {
                options.get(*index).cloned().unwrap_or_default()
            }
            ParameterValue::Text(value) => value.clone(),
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
            ParameterValue::Bars {
                numerator,
                denominator,
            } => {
                let before = (*numerator, *denominator);
                let max_num = if *denominator == 1 { 16 } else { *denominator };
                if up {
                    if *numerator < max_num {
                        *numerator += 1;
                    } else if *denominator > 1 {
                        *denominator /= 2;
                        *numerator = 1;
                    }
                } else if *numerator > 1 {
                    *numerator -= 1;
                } else if *denominator < 64 {
                    *denominator *= 2;
                    *numerator = *denominator;
                }
                before != (*numerator, *denominator)
            }
            ParameterValue::Input | ParameterValue::File { .. } | ParameterValue::Text(_) => false,
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
        }
    }

    fn cycle_unit(&mut self) -> bool {
        match self {
            ParameterValue::Time { unit, .. } => {
                *unit = match unit {
                    TimeUnit::Seconds => TimeUnit::Samples,
                    TimeUnit::Samples => {
                        *self = ParameterValue::Bars {
                            numerator: 1,
                            denominator: 4,
                        };
                        return true;
                    }
                    TimeUnit::Bars => TimeUnit::Hertz,
                    TimeUnit::Hertz => TimeUnit::Seconds,
                };
                true
            }
            ParameterValue::Bars { .. } => {
                *self = ParameterValue::Time {
                    value: 1,
                    unit: TimeUnit::Hertz,
                };
                true
            }
            _ => false,
        }
    }
}

impl Instrument {
    fn new() -> Self {
        Self {
            root: PatchSurface::new(),
            track_text: "(0/2/4/7)".to_string(),
            editing_composition: None,
            composition_stack: Vec::new(),
        }
    }

    fn surface(&self) -> &PatchSurface {
        self.editing_composition
            .and_then(|owner| composition_surface(&self.root, owner))
            .unwrap_or(&self.root)
    }

    fn surface_mut(&mut self) -> &mut PatchSurface {
        let Some(owner) = self.editing_composition else {
            return &mut self.root;
        };
        if composition_surface(&self.root, owner).is_none() {
            self.editing_composition = None;
            return &mut self.root;
        }
        composition_surface_mut(&mut self.root, owner).unwrap()
    }
}

fn composition_surface(surface: &PatchSurface, owner: ModuleId) -> Option<&PatchSurface> {
    for module in &surface.modules {
        if module.id == owner {
            return module.composition_surface();
        }
        if let Some(child) = module.composition_surface()
            && let Some(found) = composition_surface(child, owner)
        {
            return Some(found);
        }
    }
    None
}

fn composition_surface_mut(
    surface: &mut PatchSurface,
    owner: ModuleId,
) -> Option<&mut PatchSurface> {
    for module in &mut surface.modules {
        if module.id == owner {
            return module.composition_surface_mut();
        }
        if let Some(child) = module.composition_surface_mut()
            && let Some(found) = composition_surface_mut(child, owner)
        {
            return Some(found);
        }
    }
    None
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
    disabled.sort_by_key(|id| id.value());
    disabled.dedup();
    for module in &mut surface.modules {
        module.disabled = disabled.contains(&module.id);
    }
}

fn update_disabled_states_for_surface(surface: &mut PatchSurface) {
    let footprints = surface
        .modules
        .iter()
        .map(|module| {
            let ports = module.composition_surface().map(composition_ports);
            let (width, height) = module_footprint(module, ports);
            (module.id, width, height)
        })
        .collect::<Vec<_>>();
    update_surface_disabled_states(surface, &footprints);
    for module in &mut surface.modules {
        if let Some(composition) = module.composition_surface_mut() {
            update_disabled_states_for_surface(composition);
        }
    }
}

fn validate_surface_bounds(surface: &PatchSurface, width: u16, height: u16) -> Result<(), String> {
    for module in &surface.modules {
        let footprint =
            module_footprint(module, module.composition_surface().map(composition_ports));
        if module.position.x.saturating_add(footprint.0) > width
            || module.position.y.saturating_add(footprint.1) > height
        {
            return Err(format!("module {} is outside the grid", module.id.value()));
        }
        if let Some(nested) = module.composition_surface() {
            validate_surface_bounds(nested, width, height)?;
        }
    }
    for (index, left) in surface.modules.iter().enumerate() {
        let left_size = module_footprint(left, left.composition_surface().map(composition_ports));
        for right in &surface.modules[index + 1..] {
            let right_size =
                module_footprint(right, right.composition_surface().map(composition_ports));
            if left.position.x < right.position.x.saturating_add(right_size.0)
                && right.position.x < left.position.x.saturating_add(left_size.0)
                && left.position.y < right.position.y.saturating_add(right_size.1)
                && right.position.y < left.position.y.saturating_add(left_size.1)
            {
                return Err(format!(
                    "modules {} and {} overlap",
                    left.id.value(),
                    right.id.value()
                ));
            }
        }
    }
    Ok(())
}

fn max_module_id(surface: &PatchSurface) -> Option<u32> {
    surface
        .modules
        .iter()
        .map(|module| {
            module
                .composition_surface()
                .and_then(max_module_id)
                .map_or(module.id.value(), |child| child.max(module.id.value()))
        })
        .max()
}

#[cfg(test)]
fn surface_extent(surface: &PatchSurface) -> (u16, u16) {
    surface.modules.iter().fold((1, 1), |extent, module| {
        let size = module_footprint(module, module.composition_surface().map(composition_ports));
        let own = (
            module.position.x.saturating_add(size.0),
            module.position.y.saturating_add(size.1),
        );
        (extent.0.max(own.0), extent.1.max(own.1))
    })
}

fn module_footprint(module: &Module, composition_ports: Option<(u16, u16)>) -> (u16, u16) {
    if module.kind().is_routing() {
        return (1, 1);
    }
    let (inputs, outputs) =
        composition_ports.unwrap_or_else(|| (module.input_count(), module.output_count()));
    match module.orientation {
        Orientation::Right => (1, inputs.max(outputs).max(1)),
        Orientation::Down => (inputs.max(outputs).max(1), 1),
    }
}

fn composition_ports(surface: &PatchSurface) -> (u16, u16) {
    let inputs = surface
        .modules
        .iter()
        .filter(|module| module.kind() == ModuleKind::CompositionInput)
        .count() as u16;
    let outputs = surface
        .modules
        .iter()
        .filter(|module| module.kind() == ModuleKind::CompositionOutput)
        .count() as u16;
    (inputs, outputs)
}

impl Default for GuiState {
    fn default() -> Self {
        Self::new(32, 24)
    }
}

impl GuiState {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            grid_size: GridSize::new(width, height),
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
            save_requested: false,
            load_after_save: false,
            relink_sample_request: None,
            export_loops: 1,
            open_modules_requested: false,
            user_compositions: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            play_button: ButtonState::default(),
            meters_button: ButtonState::default(),
            load_button: ButtonState::default(),
            save_button: ButtonState::default(),
            export_button: ButtonState::default(),
            modules_button: ButtonState::default(),
            track_button: ButtonState::default(),
            cancel_button: ButtonState::default(),
            confirm_button: ButtonState::default(),
            discard_button: ButtonState::default(),
            sample_button: ButtonState::default(),
            pointer_drag: None,
            held_move: None,
            held_selection: None,
            audio: None,
            audio_status: "Audio disabled".to_string(),
            document_status: "Ready".to_string(),
        }
    }

    pub fn set_audio(&mut self, mut audio: AudioHandle) {
        if audio.set_playing(self.playing).is_err() {
            self.audio_status = "Audio busy: play command rejected".to_string();
        }
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
        (self.grid_size.width(), self.grid_size.height())
    }

    pub(crate) fn set_grid_view_size(&mut self, size: GridViewSize) {
        if self.grid_view_size != size {
            self.grid_view_size = size;
            self.update_grid_view();
        }
    }

    pub(crate) fn grid_view_offset_for_size(&self, size: GridViewSize) -> GridPos {
        let (min, max) = self.active_grid_rect();
        let (width, height) = self.grid_size();
        GridPos::new(
            updated_axis_view(self.grid_view.x, min.x, max.x, width, size.columns()),
            updated_axis_view(self.grid_view.y, min.y, max.y, height, size.rows()),
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

    pub fn document_status(&self) -> &str {
        &self.document_status
    }

    pub fn document_label(&self) -> String {
        let name = self
            .saved_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        if self.dirty {
            format!("{name} *")
        } else {
            name.to_string()
        }
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
            .map(|audio| audio.probe_history(module, self.probe_voice, self.probe_len as usize))
            .unwrap_or_default()
    }

    pub(crate) fn meter_values(&self, module: ModuleId) -> Vec<f32> {
        self.audio
            .as_ref()
            .map(|audio| audio.meter_values(module, self.probe_voice))
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

    pub fn composition_depth(&self) -> usize {
        self.instrument()
            .composition_stack
            .len()
            .max(usize::from(self.instrument().editing_composition.is_some()))
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

    pub fn take_save_request(&mut self) -> bool {
        let requested = self.save_requested;
        self.save_requested = false;
        requested
    }

    pub fn take_load_after_save(&mut self) -> bool {
        let requested = self.load_after_save;
        self.load_after_save = false;
        requested
    }

    pub fn take_open_modules_request(&mut self) -> bool {
        std::mem::take(&mut self.open_modules_requested)
    }

    pub fn take_relink_sample_request(&mut self) -> Option<ModuleId> {
        self.relink_sample_request.take()
    }

    pub fn load_project(&mut self, path: &Path) -> std::io::Result<()> {
        let result = project::load(path)
            .and_then(|project| self.set_project(project, path.to_string_lossy().into_owned()));
        match result {
            Ok(()) => {
                let missing = self.mark_missing_samples(path);
                let name = self.document_label();
                self.document_status = if missing.is_empty() {
                    format!("Opened {name}")
                } else {
                    format!(
                        "Opened {name}; missing sample files: {}",
                        missing.join(", ")
                    )
                };
                Ok(())
            }
            Err(error) => {
                self.document_status = format!("Open failed: {error}");
                Err(error)
            }
        }
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

    pub fn set_user_compositions(&mut self, compositions: Vec<brainwash::patch::Composition>) {
        self.user_compositions = compositions;
    }

    pub fn palette_modules(&self) -> Vec<PaletteModule> {
        self.palette_modules_for(self.palette_category)
    }

    fn palette_modules_for(&self, category: ModuleCategory) -> Vec<PaletteModule> {
        let mut modules = all_modules()
            .iter()
            .copied()
            .filter(|kind| kind.category() == category)
            .map(|kind| PaletteModule {
                kind,
                name: kind.label().to_string(),
                user: None,
            })
            .collect::<Vec<_>>();
        if category == ModuleCategory::Composition {
            modules.extend(self.user_compositions.iter().enumerate().map(
                |(index, composition)| PaletteModule {
                    kind: ModuleKind::Composition,
                    name: composition.name().to_string(),
                    user: Some(index),
                },
            ));
        }
        modules
    }

    pub fn filtered_palette_modules(&self) -> Vec<PaletteModule> {
        if self.palette_filter.is_empty() {
            return Vec::new();
        }
        let filter = self.palette_filter.to_lowercase();
        ModuleCategory::ALL
            .into_iter()
            .flat_map(|category| self.palette_modules_for(category))
            .filter(|module| module.label().to_lowercase().contains(&filter))
            .collect()
    }

    fn selected_filtered_palette_choice(&self) -> Option<PaletteModule> {
        self.filtered_palette_modules()
            .get(self.palette_filter_index)
            .cloned()
    }

    fn selected_palette_choice(&self) -> PaletteModule {
        let modules = self.palette_modules();
        modules[self.palette_index.min(modules.len().saturating_sub(1))].clone()
    }

    pub fn selected_filtered_palette_module(&self) -> Option<ModuleKind> {
        self.selected_filtered_palette_choice()
            .map(|module| module.kind)
    }

    pub fn selected_palette_module(&self) -> ModuleKind {
        self.selected_palette_choice().kind
    }

    pub fn selected_palette_label(&self) -> String {
        self.selected_palette_choice().name
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

    pub(crate) fn grid_render_modules(&self) -> Vec<GridRenderModule<'_>> {
        self.modules()
            .iter()
            .map(|module| self.grid_render_module(module))
            .collect()
    }

    pub(crate) fn grid_render_module<'a>(&self, module: &'a Module) -> GridRenderModule<'a> {
        let input_count = self.module_input_count(module);
        let output_count = self.module_output_count(module);
        let input_connected = (0..input_count)
            .map(|port| self.module_input_connected(module, port))
            .collect::<Vec<_>>();
        let port_labels = module
            .body
            .parameters()
            .into_iter()
            .filter(|parameter| parameter.value.is_port())
            .map(|parameter| parameter.name.chars().next().unwrap_or(' '))
            .collect::<Vec<_>>();
        let input_labels = (0..input_count as usize)
            .map(|index| {
                if let Some(surface) = module.composition_surface()
                    && let Some(label) = composition_inputs(surface).get(index).and_then(|id| {
                        let module = surface.modules.iter().find(|module| module.id == *id)?;
                        let ModuleBody::CompositionInput { label, .. } = &module.body else {
                            return None;
                        };
                        label.chars().next()
                    })
                {
                    return label;
                }
                port_labels
                    .get(index)
                    .copied()
                    .unwrap_or_else(|| match (module.kind(), index) {
                        (ModuleKind::RightJoin | ModuleKind::DownJoin, 0) => 'A',
                        (ModuleKind::RightJoin | ModuleKind::DownJoin, _) => 'B',
                        (ModuleKind::Composition, _) => 'I',
                        (
                            ModuleKind::TurnRightDown
                            | ModuleKind::TurnDownRight
                            | ModuleKind::LeftSplit
                            | ModuleKind::TopSplit,
                            _,
                        ) => 'I',
                        _ => ' ',
                    })
            })
            .collect::<Vec<_>>();
        GridRenderModule {
            module,
            width: self.module_width(module),
            height: self.module_height(module),
            input_count,
            output_count,
            input_labels,
            input_connected,
            has_input_top: self.module_has_input_top(module),
            has_input_left: self.module_has_input_left(module),
            has_output_bottom: self.module_has_output_bottom(module),
            has_output_right: self.module_has_output_right(module),
            left_input_offsets: (0..input_count as usize)
                .map(|input| self.left_input_offset(module, input))
                .collect(),
            top_input_offsets: (0..input_count as usize)
                .map(|input| self.top_input_offset(module, input))
                .collect(),
            right_output_offsets: (0..output_count as usize)
                .map(|output| self.right_output_offset(module, output))
                .collect(),
            bottom_output_offsets: (0..output_count as usize)
                .map(|output| self.bottom_output_offset(module, output))
                .collect(),
            probe_value: (module.kind() == ModuleKind::Probe)
                .then(|| self.probe_history(module.id()).last().copied())
                .flatten(),
            meter_values: if self.show_meters {
                self.meter_values(module.id())
            } else {
                Vec::new()
            },
        }
    }

    fn module_width(&self, module: &Module) -> u16 {
        if module.kind().is_routing() {
            return 1;
        }
        match module.orientation {
            Orientation::Right => 1,
            Orientation::Down => self
                .module_input_count(module)
                .max(self.module_output_count(module))
                .max(1),
        }
    }

    fn module_height(&self, module: &Module) -> u16 {
        if module.kind().is_routing() {
            return 1;
        }
        match module.orientation {
            Orientation::Right => self
                .module_input_count(module)
                .max(self.module_output_count(module))
                .max(1),
            Orientation::Down => 1,
        }
    }

    fn module_input_count(&self, module: &Module) -> u16 {
        if module.is_composition() {
            return module
                .composition_surface()
                .map(composition_ports)
                .unwrap_or((0, 0))
                .0;
        }
        module.input_count()
    }

    fn module_output_count(&self, module: &Module) -> u16 {
        if module.is_composition() {
            return module
                .composition_surface()
                .map(composition_ports)
                .unwrap_or((0, 0))
                .1;
        }
        module.output_count()
    }

    fn module_input_connected(&self, module: &Module, port: u16) -> bool {
        if module.is_composition() {
            let Some(surface) = module.composition_surface() else {
                return false;
            };
            let Some(id) = composition_inputs(surface).get(port as usize).copied() else {
                return false;
            };
            return surface
                .modules
                .iter()
                .find(|module| module.id == id)
                .and_then(|module| match &module.body {
                    ModuleBody::CompositionInput { value, .. } => Some(value.connected),
                    _ => None,
                })
                .unwrap_or(false);
        }
        module.input_connected(port)
    }

    fn module_audio_input(&self, module: &Module, port: usize) -> Option<AudioInputKind> {
        if let ModuleBody::Primitive(primitive) = &module.body {
            return primitive.input_kinds().get(port).copied();
        }
        if module.is_composition() {
            let surface = module.composition_surface()?;
            let id = composition_inputs(surface).get(port).copied()?;
            return surface.modules.iter().find_map(|module| {
                (module.id == id).then(|| match &module.body {
                    ModuleBody::CompositionInput { kind, .. } => *kind,
                    _ => AudioInputKind::A,
                })
            });
        }
        module.body.audio_inputs().get(port).copied()
    }

    fn module_has_input_top(&self, module: &Module) -> bool {
        if self.module_input_count(module) == 0 {
            return false;
        }
        if module.is_composition() {
            return module.orientation == Orientation::Down;
        }
        module.has_input_top()
    }

    fn module_has_input_left(&self, module: &Module) -> bool {
        if self.module_input_count(module) == 0 {
            return false;
        }
        if module.is_composition() {
            return module.orientation == Orientation::Right;
        }
        module.has_input_left()
    }

    fn module_has_output_bottom(&self, module: &Module) -> bool {
        if self.module_output_count(module) == 0 {
            return false;
        }
        if module.is_composition() {
            return module.orientation == Orientation::Down;
        }
        module.has_output_bottom()
    }

    fn module_has_output_right(&self, module: &Module) -> bool {
        if self.module_output_count(module) == 0 {
            return false;
        }
        if module.is_composition() {
            return module.orientation == Orientation::Right;
        }
        module.has_output_right()
    }

    fn left_input_offset(&self, module: &Module, input: usize) -> Option<u16> {
        if !self.module_has_input_left(module) {
            return None;
        }
        match module.kind() {
            ModuleKind::RightJoin | ModuleKind::DownJoin => (input == 0).then_some(0),
            ModuleKind::LeftSplit | ModuleKind::TurnRightDown => (input == 0).then_some(0),
            _ => (input < self.module_input_count(module) as usize).then_some(input as u16),
        }
    }

    fn top_input_offset(&self, module: &Module, input: usize) -> Option<u16> {
        if !self.module_has_input_top(module) {
            return None;
        }
        match module.kind() {
            ModuleKind::RightJoin | ModuleKind::DownJoin => (input == 1).then_some(0),
            ModuleKind::TopSplit | ModuleKind::TurnDownRight => (input == 0).then_some(0),
            _ => (input < self.module_input_count(module) as usize).then_some(input as u16),
        }
    }

    fn right_output_offset(&self, module: &Module, output: usize) -> Option<u16> {
        if !self.module_has_output_right(module) {
            return None;
        }
        match module.kind() {
            ModuleKind::LeftSplit | ModuleKind::TopSplit => (output == 1).then_some(0),
            ModuleKind::TurnDownRight | ModuleKind::RightJoin => (output == 0).then_some(0),
            _ => (output < self.module_output_count(module) as usize).then_some(output as u16),
        }
    }

    fn bottom_output_offset(&self, module: &Module, output: usize) -> Option<u16> {
        if !self.module_has_output_bottom(module) {
            return None;
        }
        match module.kind() {
            ModuleKind::LeftSplit | ModuleKind::TopSplit => (output == 0).then_some(0),
            ModuleKind::TurnRightDown | ModuleKind::DownJoin => (output == 0).then_some(0),
            _ => (output < self.module_output_count(module) as usize).then_some(output as u16),
        }
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
        origin: GridPos,
        grab: GridPos,
        target: GridPos,
    ) -> GridPos {
        let x = origin.x as i16 + target.x as i16 - grab.x as i16;
        let y = origin.y as i16 + target.y as i16 - grab.y as i16;
        self.bounded_module_position(module, x, y)
            .unwrap_or(module.position)
    }

    fn bounded_module_position(&self, module: &Module, x: i16, y: i16) -> Option<GridPos> {
        let (width, height) = self.grid_size();
        let max_x = width.checked_sub(self.module_width(module))? as i16;
        let max_y = height.checked_sub(self.module_height(module))? as i16;
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
            | Mode::LoadConfirm
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
        let width = self.grid_size.width();
        let height = self.grid_size.height();
        let mut occupied = HashMap::new();
        for module in modules.iter().filter(|module| !module.disabled) {
            for y in module.position.y..module.position.y + self.module_height(module) {
                for x in module.position.x..module.position.x + self.module_width(module) {
                    occupied.insert(GridPos::new(x, y), module);
                }
            }
        }
        for source in modules {
            if source.disabled {
                continue;
            }
            if self.module_has_output_right(source) {
                for output in 0..self.module_output_count(source) as usize {
                    let Some(source_y) = self.right_output_offset(source, output) else {
                        continue;
                    };
                    let y = source.position.y + source_y;
                    for x in source.position.x + self.module_width(source)..width {
                        let Some(target) = occupied.get(&GridPos::new(x, y)).copied() else {
                            continue;
                        };
                        if target.id == source.id {
                            continue;
                        }
                        if x == target.position.x && self.module_has_input_left(target) {
                            for input in 0..self.module_input_count(target) as usize {
                                if self
                                    .left_input_offset(target, input)
                                    .is_some_and(|target_y| target.position.y + target_y == y)
                                    && self.module_input_connected(target, input as u16)
                                    && let Some(audio) = self.module_audio_input(target, input)
                                {
                                    connections.push(Connection {
                                        from: source.id,
                                        to: target.id,
                                        output,
                                        input: ConnectionInput {
                                            index: input,
                                            audio,
                                        },
                                    });
                                    break;
                                }
                            }
                        }
                        break;
                    }
                }
            }

            if self.module_has_output_bottom(source) {
                for output in 0..self.module_output_count(source) as usize {
                    let Some(source_x) = self.bottom_output_offset(source, output) else {
                        continue;
                    };
                    let x = source.position.x + source_x;
                    for y in source.position.y + self.module_height(source)..height {
                        let Some(target) = occupied.get(&GridPos::new(x, y)).copied() else {
                            continue;
                        };
                        if target.id == source.id {
                            continue;
                        }
                        if y == target.position.y && self.module_has_input_top(target) {
                            for input in 0..self.module_input_count(target) as usize {
                                if self
                                    .top_input_offset(target, input)
                                    .is_some_and(|target_x| target.position.x + target_x == x)
                                    && self.module_input_connected(target, input as u16)
                                    && let Some(audio) = self.module_audio_input(target, input)
                                {
                                    connections.push(Connection {
                                        from: source.id,
                                        to: target.id,
                                        output,
                                        input: ConnectionInput {
                                            index: input,
                                            audio,
                                        },
                                    });
                                    break;
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        connections
    }

    fn semantic_surface_connections(
        &self,
        modules: &[Module],
    ) -> Result<Vec<Connection>, AudioPatchError> {
        let physical = self.surface_connections(modules);
        physical
            .iter()
            .filter(|connection| {
                modules
                    .iter()
                    .find(|module| module.id == connection.to)
                    .is_some_and(|module| !module.is_wire())
            })
            .map(|connection| {
                let mut semantic = *connection;
                while modules
                    .iter()
                    .find(|module| module.id == semantic.from)
                    .is_some_and(Module::is_wire)
                {
                    semantic = *physical
                        .iter()
                        .find(|candidate| candidate.to == semantic.from)
                        .ok_or(AudioPatchError::InvalidParameter)?;
                }
                Ok(Connection {
                    to: connection.to,
                    input: connection.input,
                    ..semantic
                })
            })
            .collect()
    }

    fn module_fits(&self, module: &Module, position: GridPos, ignored: &[ModuleId]) -> bool {
        self.area_fits(
            self.module_width(module),
            self.module_height(module),
            position,
            ignored,
        )
    }

    fn area_fits(&self, width: u16, height: u16, position: GridPos, ignored: &[ModuleId]) -> bool {
        let (grid_width, grid_height) = self.grid_size();
        if position.x + width > grid_width || position.y + height > grid_height {
            return false;
        }
        !self
            .modules()
            .iter()
            .filter(|module| !ignored.contains(&module.id))
            .any(|module| self.module_overlaps(module, position, width, height))
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
            update_disabled_states_for_surface(&mut instrument.root);
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
                let modules = self.modules().len();
                let active_modules = self
                    .modules()
                    .iter()
                    .filter(|module| !module.disabled)
                    .count();
                let Some(audio) = self.audio.as_mut() else {
                    self.audio_status = "Audio disabled".to_string();
                    return;
                };
                eprintln!(
                    "[bw dbg model] publish compiled patch modules={} active_modules={} voice_mode={:?} probes={} meters={}",
                    modules,
                    active_modules,
                    patch.voice_mode,
                    patch.probes.len(),
                    patch.meters.len()
                );
                audio.set_latest_patch_with_telemetry(
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
                    audio.set_latest_patch(CompiledPatch::silence());
                }
                eprintln!(
                    "[bw dbg model] compile error {:?}; published silence",
                    error
                );
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
                self.audio_status = if audio.submit_track(track, self.bpm).is_ok() {
                    "Audio ready".to_string()
                } else {
                    "Audio busy: track rejected".to_string()
                };
            }
            Err(_) => {
                self.audio_status = "Silent: invalid track".to_string();
            }
        }
    }

    fn sync_audio_playing(&mut self) {
        if let Some(audio) = &mut self.audio {
            if audio.set_playing(self.playing).is_err() {
                self.audio_status = "Audio busy: play command rejected".to_string();
            }
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

    fn commit_save_path(&mut self, path: &Path) {
        self.saved_path = Some(path.to_string_lossy().into_owned());
        self.dirty = false;
    }

    pub fn save_project(&mut self, path: &Path) -> bool {
        let result = self
            .project()
            .and_then(|project| project::save(path, &project));
        match result {
            Ok(()) => {
                self.commit_save_path(path);
                self.document_status = format!("Saved {}", self.document_label());
                true
            }
            Err(error) => {
                self.document_status = format!("Save failed: {error}");
                false
            }
        }
    }

    pub(crate) fn request_relink_sample(&mut self, module: ModuleId) {
        self.relink_sample_request = Some(module);
    }

    pub fn relink_sample(&mut self, module: ModuleId, path: &Path) -> bool {
        let before = self.snapshot();
        let changed = {
            let Some(module) = self
                .instrument_mut()
                .surface_mut()
                .modules
                .iter_mut()
                .find(|candidate| candidate.id == module)
            else {
                return false;
            };
            let ModuleBody::Sample {
                file_name,
                file_missing,
                ..
            } = &mut module.body
            else {
                return false;
            };
            *file_name = path.to_string_lossy().into_owned();
            *file_missing = false;
            true
        };
        if changed {
            self.commit(before);
            self.document_status = format!("Relinked sample {}", path.display());
        }
        changed
    }

    fn mark_missing_samples(&mut self, project_path: &Path) -> Vec<String> {
        let mut missing = Vec::new();
        let base = project_path.parent().unwrap_or_else(|| Path::new("."));
        for instrument in &mut self.instruments {
            mark_missing_samples_in_surface(&mut instrument.root, base, &mut missing);
        }
        missing
    }

    fn project(&self) -> io::Result<Project> {
        for (index, instrument) in self.instruments.iter().enumerate() {
            if index != self.active_instrument
                && (!instrument.root.modules.is_empty() || instrument.track_text != "(0/2/4/7)")
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "project files support one instrument",
                ));
            }
        }
        project_from_instrument(self.instrument(), self.bpm, self.scale_index)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    fn set_project(&mut self, project: Project, path: String) -> io::Result<()> {
        let bpm = project.bpm.round().clamp(1.0, u16::MAX as f32) as u16;
        let root = instrument_surface_from_project(&project)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        validate_surface_bounds(&root, self.grid_size.width(), self.grid_size.height())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let instrument = Instrument {
            root,
            track_text: project.track.unwrap_or_else(|| "(0/2/4/7)".to_string()),
            editing_composition: None,
            composition_stack: Vec::new(),
        };
        let mut instruments = vec![instrument];
        instruments.resize_with(INSTRUMENT_COUNT, Instrument::new);
        self.instruments = instruments;
        self.active_instrument = 0;
        self.bpm = bpm;
        self.scale_index = project.scale_idx.min(SCALE_NAMES.len().saturating_sub(1));
        self.next_module_id = self
            .instruments
            .iter()
            .filter_map(|instrument| max_module_id(&instrument.root))
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
        Ok(())
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
        let (width, height) = self.grid_size();
        let ((min_x, max_x), (min_y, max_y)) = match self.mode {
            Mode::SelectMove {
                anchor,
                extent,
                origin,
            }
            | Mode::CopySelection {
                anchor,
                extent,
                origin,
            } if self.held_selection.is_none() => {
                let left = anchor.x.min(extent.x);
                let right = anchor.x.max(extent.x);
                let top = anchor.y.min(extent.y);
                let bottom = anchor.y.max(extent.y);
                (
                    (
                        origin.x.saturating_sub(left) as i16,
                        width
                            .saturating_sub(1)
                            .saturating_sub(right.saturating_sub(origin.x))
                            as i16,
                    ),
                    (
                        origin.y.saturating_sub(top) as i16,
                        height
                            .saturating_sub(1)
                            .saturating_sub(bottom.saturating_sub(origin.y))
                            as i16,
                    ),
                )
            }
            _ => (
                (0, width.saturating_sub(1) as i16),
                (0, height.saturating_sub(1) as i16),
            ),
        };
        let grid_x = (cursor.x as i16 + dx).clamp(0, width.saturating_sub(1) as i16);
        let grid_y = (cursor.y as i16 + dy).clamp(0, height.saturating_sub(1) as i16);
        let (x, y) = if self
            .module_at(GridPos::new(grid_x as u16, grid_y as u16))
            .is_some_and(Module::is_composition)
        {
            (grid_x, grid_y)
        } else {
            (grid_x.clamp(min_x, max_x), grid_y.clamp(min_y, max_y))
        };
        self.instrument_mut().surface_mut().cursor = GridPos::new(x as u16, y as u16);
    }

    fn update_grid_view(&mut self) {
        let (min, max) = self.active_grid_rect();
        let (width, height) = self.grid_size();
        self.grid_view.x = updated_axis_view(
            self.grid_view.x,
            min.x,
            max.x,
            width,
            self.grid_view_size.columns(),
        );
        self.grid_view.y = updated_axis_view(
            self.grid_view.y,
            min.y,
            max.y,
            height,
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
                let dx = bounded_delta(
                    module.position.x,
                    module.position.x + width,
                    self.cursor().x as i16 - origin.x as i16,
                    self.grid_size.width(),
                );
                let dy = bounded_delta(
                    module.position.y,
                    module.position.y + height,
                    self.cursor().y as i16 - origin.y as i16,
                    self.grid_size.height(),
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
                let source = GridRect::from_points(anchor, extent);
                let source_min = source.min();
                let source_max = source.max();
                let dx = bounded_delta(
                    source_min.x,
                    source_max.x,
                    self.cursor().x as i16 - origin.x as i16,
                    self.grid_size.width(),
                );
                let dy = bounded_delta(
                    source_min.y,
                    source_max.y,
                    self.cursor().y as i16 - origin.y as i16,
                    self.grid_size.height(),
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

    fn insert_palette_module(&mut self, choice: PaletteModule) {
        let graph = choice
            .user
            .and_then(|index| self.user_compositions.get(index))
            .cloned();
        self.insert_at_cursor(choice.kind, graph);
    }

    fn insert_at_cursor(&mut self, kind: ModuleKind, graph: Option<brainwash::patch::Composition>) {
        let cursor = self.cursor();
        let before = self.snapshot();
        let id = ModuleId::new(self.next_module_id);
        self.next_module_id += 1;
        let mut body = kind.default_body();
        if let Some(graph) = graph {
            let Some(projected) = composition_body(Box::new(graph), &mut self.next_module_id)
            else {
                self.next_module_id = before.next_module_id;
                return;
            };
            body = projected;
        } else {
            let candidate = Module {
                id,
                position: cursor,
                orientation: Orientation::Right,
                body: body.clone(),
                disabled: false,
            };
            if let Ok(AudioModule::Composition(graph)) =
                audio_module(&candidate, SampleRate::new(44_100).unwrap(), self.bpm)
            {
                let Some(projected) = composition_body(graph, &mut self.next_module_id) else {
                    self.next_module_id = before.next_module_id;
                    return;
                };
                body = projected;
            }
        }
        let module = Module {
            id,
            position: cursor,
            orientation: Orientation::Right,
            body,
            disabled: false,
        };
        if module.composition_surface().is_some_and(|surface| {
            validate_surface_bounds(surface, self.grid_size.width(), self.grid_size.height())
                .is_err()
        }) {
            self.next_module_id = before.next_module_id;
            return;
        }
        if !self.module_fits(&module, cursor, &[]) {
            self.next_module_id = before.next_module_id;
            return;
        }
        self.instrument_mut().surface_mut().modules.push(module);
        sync_delay_sources(self.instrument_mut().surface_mut());
        self.commit(before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gui_module_kind_has_audio_mapping() {
        let rate = SampleRate::new(44_100).unwrap();
        for kind in all_modules().iter().copied() {
            if kind == ModuleKind::DelayTap {
                continue;
            }
            let module = Module {
                id: ModuleId::new(0),
                position: GridPos::new(0, 0),
                orientation: Orientation::Right,
                body: kind.default_body(),
                disabled: false,
            };
            assert!(audio_module(&module, rate, 120).is_ok(), "{kind:?}");
        }
    }

    #[test]
    fn delay_tap_audio_mapping_references_delay_state() {
        let delay = Module {
            id: ModuleId::new(1),
            position: GridPos::new(0, 0),
            orientation: Orientation::Right,
            body: ModuleKind::Delay.default_body(),
            disabled: false,
        };
        let mut tap = Module {
            id: ModuleId::new(2),
            position: GridPos::new(0, 0),
            orientation: Orientation::Right,
            body: ModuleKind::DelayTap.default_body(),
            disabled: false,
        };
        let ModuleBody::DelayTap { source, .. } = &mut tap.body else {
            unreachable!()
        };
        source.selected = Some(delay.id);
        source.options.push(delay.id);
        let mut patch = Patch::new();
        let delay_id =
            patch.insert(audio_module(&delay, SampleRate::new(44_100).unwrap(), 120).unwrap());
        let ids = [AudioNode {
            key: AudioKey::Root(delay.id),
            id: delay_id,
        }];

        assert!(
            audio_patch::insert_audio_delay_tap(
                &mut patch,
                &tap,
                &[delay.clone(), tap.clone()],
                &ids,
                AudioKey::Root,
            )
            .is_ok()
        );
    }

    #[test]
    fn disconnected_degree_probe_cannot_change_output_voice_mode() {
        let mut state = GuiState::new(8, 8);
        state.instrument_mut().root.modules = vec![
            Module {
                id: ModuleId::new(1),
                position: GridPos::new(2, 1),
                orientation: Orientation::Right,
                body: ModuleKind::Osc.default_body(),
                disabled: false,
            },
            Module {
                id: ModuleId::new(2),
                position: GridPos::new(6, 1),
                orientation: Orientation::Right,
                body: ModuleKind::Output.default_body(),
                disabled: false,
            },
            Module {
                id: ModuleId::new(3),
                position: GridPos::new(1, 0),
                orientation: Orientation::Right,
                body: ModuleKind::DegreeGate.default_body(),
                disabled: false,
            },
            Module {
                id: ModuleId::new(4),
                position: GridPos::new(2, 0),
                orientation: Orientation::Right,
                body: ModuleKind::Probe.default_body(),
                disabled: false,
            },
        ];

        assert_eq!(
            state
                .compile_gui_audio_patch(SampleRate::new(44_100).unwrap())
                .unwrap()
                .voice_mode,
            VoiceMode::Single
        );
    }

    #[test]
    fn default_module_body_matches_module_kind() {
        for kind in all_modules().iter().copied() {
            let body = kind.default_body();
            assert_eq!(body.kind(), kind);
            assert_eq!(body.env_points().is_empty(), kind != ModuleKind::Envelope);
        }
    }

    #[test]
    fn module_parameter_rows_apply_to_typed_body() {
        let mut module = Module {
            id: ModuleId::new(0),
            position: GridPos::new(0, 0),
            orientation: Orientation::Right,
            body: ModuleKind::Osc.default_body(),
            disabled: false,
        };
        let mut frequency = module.parameter(1).unwrap();
        frequency.value = ParameterValue::Float {
            value: 88_000,
            min: 1,
            max: 200_000,
            step: 100,
        };
        frequency.connected = false;

        assert!(module.set_parameter(1, frequency));
        match module.body {
            ModuleBody::Osc { frequency, .. } => {
                assert_eq!(frequency.value, 88_000);
                assert!(!frequency.connected);
            }
            _ => panic!("wrong body"),
        }
    }

    #[test]
    fn pointer_module_drag_uses_original_module_position() {
        let mut state = GuiState::new(8, 8);
        let module = Module {
            id: ModuleId::new(0),
            position: GridPos::new(1, 1),
            orientation: Orientation::Right,
            body: ModuleKind::Adsr.default_body(),
            disabled: false,
        };
        state.instrument_mut().surface_mut().modules.push(module);

        state.drag_grid_cell(GridPointerPhase::Start, GridPos::new(1, 2));
        state.drag_grid_cell(GridPointerPhase::Drag, GridPos::new(3, 3));
        state.drag_grid_cell(GridPointerPhase::Drag, GridPos::new(4, 4));
        state.drag_grid_cell(GridPointerPhase::End, GridPos::new(4, 4));

        assert_eq!(state.modules()[0].position, GridPos::new(4, 3));
    }

    #[test]
    fn user_composition_is_a_named_palette_module() {
        let mut patch = Patch::new();
        let input = patch.insert(AudioModule::Input {
            kind: AudioInputKind::In,
            default: AudioSample::ZERO,
        });
        let output = patch.output_port(input, 0).unwrap();
        patch.output(output).unwrap();
        let composition = brainwash::patch::Composition::new(
            "My Module",
            patch,
            [("Signal".to_string(), AudioInputKind::In, input)],
            [("Output".to_string(), output)],
        )
        .unwrap();
        let mut state = GuiState::new(16, 16);
        state.set_user_compositions(vec![composition]);
        state.palette_category = ModuleCategory::Composition;

        let modules = state.palette_modules();
        assert_eq!(modules.last().unwrap().label(), "My Module");
        state.choose_palette_index(modules.len() - 1);
        state.mode = Mode::Palette;
        state.apply(GuiAction::Confirm);

        assert_eq!(state.modules()[0].label(), "My Module");
        assert!(state.modules()[0].composition_surface().is_some());
    }

    #[test]
    fn composition_projection_preserves_declared_outputs() {
        let mut patch = Patch::new();
        let first = patch.insert(AudioModule::Constant(AudioSample::new(0.25).unwrap()));
        let second = patch.insert(AudioModule::Constant(AudioSample::new(0.75).unwrap()));
        let first_output = patch.output_port(first, 0).unwrap();
        let second_output = patch.output_port(second, 0).unwrap();
        patch.output(first_output).unwrap();
        let graph = Box::new(
            brainwash::patch::Composition::new(
                "Pair",
                patch,
                Vec::<(String, AudioInputKind, AudioModuleId)>::new(),
                [
                    ("First".to_string(), first_output),
                    ("Second".to_string(), second_output),
                ],
            )
            .unwrap(),
        );
        let mut state = GuiState::default();
        let owner = Module {
            id: ModuleId::new(0),
            position: GridPos::new(0, 0),
            orientation: Orientation::Right,
            body: composition_body(graph, &mut state.next_module_id).unwrap(),
            disabled: false,
        };
        let surface = owner.composition_surface().unwrap();
        assert_eq!(audio_patch::composition_outputs(surface).len(), 2);
        assert_eq!(
            audio_patch::composition_outputs(surface)
                .into_iter()
                .filter_map(|id| surface.modules.iter().find(|module| module.id == id))
                .filter_map(|module| match &module.body {
                    ModuleBody::CompositionOutput { label, .. } => Some(label.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            ["First", "Second"]
        );
        let AudioModule::Composition(projected) =
            audio_patch::gui_composition(&state, &owner, SampleRate::new(44_100).unwrap(), 120)
                .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(projected.outputs().len(), 2);
        let mut outer = Patch::new();
        let pair = outer.insert(AudioModule::Composition(projected));
        outer.output(outer.output_port(pair, 1).unwrap()).unwrap();
        let mut compiled = CompiledPatch::new(&outer, SampleRate::new(44_100).unwrap()).unwrap();
        assert_eq!(compiled.next().left(), AudioSample::new(0.75).unwrap());
    }

    #[test]
    fn composition_projection_rejects_a_surface_larger_than_the_grid() {
        let mut patch = Patch::new();
        let mut output = None;
        for _ in 0..800 {
            output = Some(patch.insert(AudioModule::Constant(AudioSample::ZERO)));
        }
        let output = patch.output_port(output.unwrap(), 0).unwrap();
        patch.output(output).unwrap();
        let composition = brainwash::patch::Composition::new(
            "Too Large",
            patch,
            Vec::<(String, AudioInputKind, AudioModuleId)>::new(),
            [("Output".to_string(), output)],
        )
        .unwrap();
        let mut state = GuiState::default();
        state.set_user_compositions(vec![composition]);
        state.palette_category = ModuleCategory::Composition;
        let index = state.palette_modules().len() - 1;
        state.choose_palette_index(index);
        state.apply(GuiAction::Confirm);

        assert!(state.modules().is_empty());
    }

    #[test]
    fn complex_builtin_opens_as_an_owned_surface() {
        let mut state = GuiState::default();
        state.insert_at_cursor(ModuleKind::Compressor, None);
        let undo = state.undo.len();

        state.apply(GuiAction::EditComposition);

        assert_eq!(state.composition_depth(), 1);
        assert_eq!(state.undo.len(), undo);
        assert_eq!(
            state.instrument().root.modules[0].kind(),
            ModuleKind::Composition
        );
        assert_eq!(state.instrument().root.modules[0].label(), "Compressor");
        let before = state.modules().len();
        assert!(!state.connections().is_empty());
        state.instrument_mut().surface_mut().cursor = state.modules()[0].position;
        state.apply(GuiAction::Delete);
        assert_eq!(state.modules().len(), before - 1);
    }

    #[test]
    fn reverb_projection_stays_bounded() {
        let mut state = GuiState::default();
        state.insert_at_cursor(ModuleKind::Reverb, None);
        let surface = state.modules()[0].composition_surface().unwrap();
        assert_eq!(state.grid_size(), (32, 24));
        assert!(surface.modules.len() < 30);
        assert!(surface_extent(surface).0 < 20);
    }

    #[test]
    fn every_reverb_surface_fits_the_default_grid() {
        fn check(surface: &PatchSurface) {
            let extent = surface_extent(surface);
            assert!(
                extent.0 <= 32 && extent.1 <= 24,
                "surface is {}x{} with {} modules",
                extent.0,
                extent.1,
                surface.modules.len()
            );
            assert!(surface.modules.iter().all(|module| !module.disabled));
            for module in &surface.modules {
                if let Some(nested) = module.composition_surface() {
                    check(nested);
                }
            }
        }

        let mut state = GuiState::default();
        state.insert_at_cursor(ModuleKind::Reverb, None);
        check(state.modules()[0].composition_surface().unwrap());
    }

    #[test]
    fn projected_reverb_compiles_from_the_connections_rendered_by_the_gui() {
        let graph = brainwash::preset::reverb(
            Unit::new(0.7).unwrap(),
            Unit::new(0.2).unwrap(),
            Unit::ZERO,
            Unit::new(0.8).unwrap(),
        );
        let AudioModule::Composition(graph) = graph else {
            unreachable!()
        };
        let mut state = GuiState::default();
        state.next_module_id = 3;
        state.instrument_mut().root = PatchSurface {
            cursor: GridPos::new(0, 0),
            modules: vec![
                Module {
                    id: ModuleId::new(0),
                    position: GridPos::new(0, 0),
                    orientation: Orientation::Right,
                    body: ModuleBody::Gate,
                    disabled: false,
                },
                Module {
                    id: ModuleId::new(1),
                    position: GridPos::new(1, 0),
                    orientation: Orientation::Right,
                    body: composition_body(graph.clone(), &mut state.next_module_id).unwrap(),
                    disabled: false,
                },
                Module {
                    id: ModuleId::new(2),
                    position: GridPos::new(2, 0),
                    orientation: Orientation::Right,
                    body: ModuleKind::Output.default_body(),
                    disabled: false,
                },
            ],
        };
        let rate = SampleRate::new(29_761).unwrap();
        fn check(state: &GuiState, graph: &brainwash::patch::Composition, surface: &PatchSurface) {
            let core_modules = graph.patch().module_entries().collect::<Vec<_>>();
            let gui_modules = surface
                .modules
                .iter()
                .filter(|module| {
                    !module.is_wire() && module.kind() != ModuleKind::CompositionOutput
                })
                .collect::<Vec<_>>();
            assert_eq!(
                core_modules.len(),
                gui_modules.len(),
                "{} modules",
                graph.name()
            );
            let core = graph
                .patch()
                .connection_entries()
                .map(|(from, to)| {
                    (
                        core_modules
                            .iter()
                            .position(|(id, _)| *id == from.module())
                            .unwrap(),
                        from.index(),
                        core_modules
                            .iter()
                            .position(|(id, _)| *id == to.module())
                            .unwrap(),
                        to.kind(),
                    )
                })
                .collect::<HashSet<_>>();
            let gui = state
                .semantic_surface_connections(&surface.modules)
                .unwrap()
                .into_iter()
                .filter_map(|connection| {
                    Some((
                        gui_modules
                            .iter()
                            .position(|module| module.id == connection.from)?,
                        connection.output as u16,
                        gui_modules
                            .iter()
                            .position(|module| module.id == connection.to)?,
                        connection.audio_input(),
                    ))
                })
                .collect::<HashSet<_>>();
            assert_eq!(gui, core, "{} connections", graph.name());
            for ((_, core_module), gui_module) in core_modules.into_iter().zip(gui_modules) {
                if let (AudioModule::Composition(nested), Some(nested_surface)) =
                    (core_module, gui_module.composition_surface())
                {
                    check(state, nested, nested_surface);
                }
            }
        }
        check(
            &state,
            &graph,
            state.instrument().root.modules[1]
                .composition_surface()
                .unwrap(),
        );
        let mut projected = state.compile_audio_patch(rate).unwrap();
        let mut patch = Patch::new();
        let gate = patch.insert(AudioModule::Gate);
        let reverb = patch.insert(AudioModule::Composition(graph));
        let input = patch.input_port(reverb, AudioInputKind::In).unwrap();
        patch
            .connect_input(patch.output_port(gate, 0).unwrap(), input)
            .unwrap();
        patch.output(patch.output_port(reverb, 0).unwrap()).unwrap();
        let mut canonical = CompiledPatch::new(&patch, rate).unwrap();
        for frame in 0..12_000 {
            let controls = brainwash::compile::PatchControls {
                gate: if frame == 0 { 1.0 } else { 0.0 },
                ..brainwash::compile::PatchControls::default()
            };
            let actual = projected.next_with_controls(controls);
            let expected = canonical.next_with_controls(controls);
            assert_eq!(actual, expected, "frame {frame}");
        }
    }

    #[test]
    fn every_nested_reverb_composition_can_be_opened() {
        let mut state = GuiState::default();
        state.insert_at_cursor(ModuleKind::Reverb, None);
        state.apply(GuiAction::EditComposition);

        let diffuser = state
            .modules()
            .iter()
            .find(|module| module.label() == "Input Diffuser")
            .unwrap()
            .position;
        state.instrument_mut().surface_mut().cursor = diffuser;
        state.apply(GuiAction::EditComposition);
        let stage = state
            .modules()
            .iter()
            .find(|module| module.label() == "Diffuser Stage")
            .unwrap()
            .position;
        state.instrument_mut().surface_mut().cursor = stage;
        state.apply(GuiAction::EditComposition);
        assert_eq!(state.composition_depth(), 3);
        state.apply(GuiAction::ExitComposition);
        state.apply(GuiAction::ExitComposition);

        let tank = state
            .modules()
            .iter()
            .find(|module| module.label() == "FDN Tank")
            .unwrap()
            .position;
        state.instrument_mut().surface_mut().cursor = tank;
        state.apply(GuiAction::EditComposition);
        let drive = state
            .modules()
            .iter()
            .find(|module| module.label() == "FDN Drive")
            .unwrap()
            .position;
        state.instrument_mut().surface_mut().cursor = drive;
        state.apply(GuiAction::EditComposition);
        for label in ["Delay Modulation Bank", "Feedback Bank"] {
            let position = state
                .modules()
                .iter()
                .find(|module| module.label() == label)
                .unwrap()
                .position;
            state.instrument_mut().surface_mut().cursor = position;
            state.apply(GuiAction::EditComposition);
            assert_eq!(state.composition_depth(), 4);
            state.apply(GuiAction::ExitComposition);
        }
    }

    #[test]
    fn pointer_can_mutate_a_builtin_composition() {
        let mut state = GuiState::default();
        state.insert_at_cursor(ModuleKind::Compressor, None);
        state.apply(GuiAction::EditComposition);
        let module = &state.modules()[0];
        let id = module.id;
        let position = module.position;

        state.start_grid_drag(position);
        state.update_grid_drag(GridPos::new(5, 5));

        assert_ne!(
            state
                .modules()
                .iter()
                .find(|module| module.id == id)
                .unwrap()
                .position,
            position
        );
    }

    #[test]
    fn lowered_composition_executes_from_its_surface() {
        let mut patch = Patch::new();
        let input = patch.insert(AudioModule::Input {
            kind: AudioInputKind::In,
            default: AudioSample::ZERO,
        });
        let output = patch.output_port(input, 0).unwrap();
        patch.output(output).unwrap();
        let graph = Box::new(
            brainwash::patch::Composition::new(
                "Identity",
                patch,
                [("Signal".to_string(), AudioInputKind::In, input)],
                [("Output".to_string(), output)],
            )
            .unwrap(),
        );
        let mut state = GuiState::new(8, 8);
        state.next_module_id = 3;
        let composition = composition_body(graph, &mut state.next_module_id).unwrap();
        state.instrument_mut().root.modules = vec![
            Module {
                id: ModuleId::new(0),
                position: GridPos::new(0, 0),
                orientation: Orientation::Right,
                body: ModuleBody::Gate,
                disabled: false,
            },
            Module {
                id: ModuleId::new(1),
                position: GridPos::new(1, 0),
                orientation: Orientation::Right,
                body: composition,
                disabled: false,
            },
            Module {
                id: ModuleId::new(2),
                position: GridPos::new(2, 0),
                orientation: Orientation::Right,
                body: ModuleKind::Output.default_body(),
                disabled: false,
            },
        ];

        assert!(
            state
                .compile_audio_patch(SampleRate::new(44_100).unwrap())
                .is_ok()
        );
    }
}

impl ModuleKind {
    fn width(self, orientation: Orientation) -> u16 {
        if self.is_routing() {
            return 1;
        }
        match orientation {
            Orientation::Right => 1,
            Orientation::Down => self.input_count().max(self.output_count()).max(1),
        }
    }

    fn height(self, orientation: Orientation) -> u16 {
        if self.is_routing() {
            return 1;
        }
        match orientation {
            Orientation::Right => self.input_count().max(self.output_count()).max(1),
            Orientation::Down => 1,
        }
    }

    fn input_count(self) -> u16 {
        match self {
            ModuleKind::Primitive => 0,
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::Rate
            | ModuleKind::Random
            | ModuleKind::Sample
            | ModuleKind::CompositionInput => 0,
            ModuleKind::RightJoin | ModuleKind::DownJoin => 2,
            ModuleKind::TurnRightDown
            | ModuleKind::TurnDownRight
            | ModuleKind::LeftSplit
            | ModuleKind::TopSplit => 1,
            ModuleKind::DegreeGate
            | ModuleKind::Transpose
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
            | ModuleKind::CompositionOutput
            | ModuleKind::Composition => self.default_body().input_count(),
        }
    }

    fn output_count(self) -> u16 {
        match self {
            ModuleKind::Primitive => 1,
            ModuleKind::Output | ModuleKind::CompositionOutput => 0,
            ModuleKind::LeftSplit | ModuleKind::TopSplit => 2,
            ModuleKind::Freq
            | ModuleKind::Gate
            | ModuleKind::Degree
            | ModuleKind::DegreeGate
            | ModuleKind::Rate
            | ModuleKind::Transpose
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
            | ModuleKind::CompositionInput
            | ModuleKind::Composition => 1,
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
            TimeUnit::Bars => typed_bars(text),
        },
        ParameterValue::Bars { .. } => typed_bars(text),
        ParameterValue::Text(_) => {
            let value = text.trim();
            (!value.is_empty()).then(|| ParameterValue::Text(value.to_string()))
        }
        ParameterValue::Int { .. }
        | ParameterValue::Input
        | ParameterValue::File { .. }
        | ParameterValue::Enum { .. } => None,
    }
}

fn typed_bars(text: &str) -> Option<ParameterValue> {
    let (num, denom) = text.split_once('/')?;
    let numerator = num.parse::<i32>().ok()?.max(1);
    let denominator = denom.parse::<i32>().ok()?;
    if denominator <= 0 {
        return None;
    }
    Some(ParameterValue::Bars {
        numerator,
        denominator,
    })
}

fn normalized_rect(anchor: GridPos, extent: GridPos) -> (GridPos, GridPos) {
    let rect = GridRect::from_points(anchor, extent);
    (rect.min(), rect.max())
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
        ModuleKind::DegreeGate,
        ModuleKind::Rate,
        ModuleKind::Transpose,
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
        ModuleKind::CompositionInput,
        ModuleKind::CompositionOutput,
        ModuleKind::Composition,
    ]
}
