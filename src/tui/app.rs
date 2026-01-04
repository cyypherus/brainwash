use super::bindings::{self, lookup, Action};
use super::engine::{
    command_channel, compile_patch, compile_voices, meter_channel, output_channel, AudioCommand,
    AudioEngine, CommandSender, CompileContext, CompiledPatch, MeterReceiver, OutputReceiver,
    TrackState, OUTPUT_INTERVAL,
};
use super::grid::GridPos;
use super::instrument::Instrument;
use super::module::{
    Module, ModuleCategory, ModuleId, ModuleKind, ModuleParams, ParamKind, SubPatchId,
};
use super::patch::{Patch, PatchSet};
use super::persist;
use super::render::{
    AdsrWidget, EditWidget, EnvelopeWidget, GridWidget, HelpWidget, PaletteWidget, ProbeWidget,
    SampleWidget, StatusWidget,
};
use crate::live::AudioPlayer;
use crate::scale::{
    amaj, amin, asharpmaj, asharpmin, bmaj, bmin, chromatic, cmaj, cmin, csharpmaj, csharpmin,
    dmaj, dmin, dsharpmaj, dsharpmin, emaj, emin, fmaj, fmin, fsharpmaj, fsharpmin, gmaj, gmin,
    gsharpmaj, gsharpmin, Scale,
};
use crate::track::Track;
use crate::Signal;
use cpal::traits::StreamTrait;
use lilt::{Animated, Easing};
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{io, time::Duration};

use tui_input::{Input, InputRequest};

fn scan_wav_files() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(".")
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();
                    if path.extension().map(|x| x == "wav").unwrap_or(false) {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

const BRAND_FONT: &[(&str, [&str; 6])] = &[
    (
        "B",
        [
            "d8888b.", "88  `8D", "88oooY'", "88~~~b.", "88   8D", "Y8888P'",
        ],
    ),
    (
        "R",
        [
            "d8888b.", "88  `8D", "88oobY'", "88`8b  ", "88 `88.", "88   YD",
        ],
    ),
    (
        "A",
        [
            " .d8b. ", "d8' `8b", "88ooo88", "88~~~88", "88   88", "YP   YP",
        ],
    ),
    (
        "I",
        [
            "d888888b", "  `88'  ", "   88   ", "   88   ", "  .88.  ", "Y888888P",
        ],
    ),
    (
        "N",
        [
            "d8b   db", "888o  88", "88V8o 88", "88 V8o88", "88  V888", "VP   V8P",
        ],
    ),
    (
        "W",
        [
            "db   d8b   db",
            "88   I8I   88",
            "88   I8I   88",
            "Y8   I8I   88",
            "`8b d8'8b d8'",
            " `8b8' `8d8' ",
        ],
    ),
    (
        "S",
        [
            ".d8888.", "88'  YP", "`8bo.  ", "  `Y8b.", "db   8D", "`8888Y'",
        ],
    ),
    (
        "H",
        [
            "db   db", "88   88", "88ooo88", "88~~~88", "88   88", "YP   YP",
        ],
    ),
];

fn render_brand(text: &str, gap: usize) -> Vec<String> {
    let mut rows = vec![String::new(); 6];
    for (i, c) in text.chars().enumerate() {
        if let Some((_, glyph)) = BRAND_FONT
            .iter()
            .find(|(ch, _)| ch.chars().next() == Some(c))
        {
            for (row_idx, row) in glyph.iter().enumerate() {
                if i > 0 {
                    for _ in 0..gap {
                        rows[row_idx].push(' ');
                    }
                }
                rows[row_idx].push_str(row);
            }
        }
    }
    rows
}

fn load_wav_samples(path: &str) -> Arc<Vec<f32>> {
    let Ok(reader) = hound::WavReader::open(path) else {
        return Arc::new(Vec::new());
    };
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .step_by(spec.channels as usize)
            .collect(),
        hound::SampleFormat::Int => {
            let max = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(|s| s.ok())
                .step_by(spec.channels as usize)
                .map(|s| s as f32 / max)
                .collect()
        }
    };
    Arc::new(samples)
}

#[derive(Clone, PartialEq)]
enum Mode {
    Normal,
    Palette,
    Move {
        module_id: ModuleId,
        origin: GridPos,
    },
    Copy {
        module: Module,
    },
    CopySelection {
        modules: Vec<(Module, GridPos)>,
        origin: GridPos,
    },
    Select {
        anchor: GridPos,
    },
    SelectMove {
        anchor: GridPos,
        extent: GridPos,
        move_origin: GridPos,
    },
    MouseSelect {
        anchor: GridPos,
    },

    QuitConfirm,
    Edit {
        module_id: ModuleId,
        param_idx: usize,
    },
    ValueInput {
        module_id: ModuleId,
        param_idx: usize,
    },
    AdsrEdit {
        module_id: ModuleId,
        param_idx: usize,
    },
    EnvEdit {
        module_id: ModuleId,
        point_idx: usize,
        editing: bool,
    },
    ProbeEdit {
        module_id: ModuleId,
    },
    SampleView {
        module_id: ModuleId,
        zoom: f32,
        offset: f32,
    },
    SavePrompt,
    SaveConfirm,
    TrackSettings {
        param_idx: usize,
    },
    ExportPrompt,
    ExportConfirm,
}

enum AppRequest {
    EditTrack,
}

struct App {
    instruments: Vec<Instrument>,
    current_instrument: usize,
    mode: Mode,
    pending_request: Option<AppRequest>,
    palette_category: usize,
    palette_selections: [usize; 9],
    palette_filter: String,
    palette_filter_selection: usize,
    palette_searching: bool,
    message: Option<String>,
    should_quit: bool,
    dirty: bool,
    cmd_tx: CommandSender,
    meter_rx: MeterReceiver,
    output_rx: OutputReceiver,
    show_meters: bool,
    playing: bool,
    file_path: Option<PathBuf>,
    save_input: Input,
    probe_len: usize,
    grid_area: Rect,
    drag_start: Option<(ModuleId, GridPos)>,
    dragging: bool,
    select_start: Option<GridPos>,
    bpm: f32,
    export_input: Input,
    export_loops: usize,
    output_history: VecDeque<f32>,
    value_input: Input,
    step_size: usize,
    probe_voice: usize,
    brand_scroll: Animated<f32, Instant>,
    help_scroll: usize,
    last_mode_for_help: std::mem::Discriminant<Mode>,
    active_pitches: Vec<u8>,
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

fn scale_from_idx(idx: usize) -> Scale {
    match idx {
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

const SUBPATCH_COLORS: &[Color] = &[
    Color::Rgb(255, 150, 50),
    Color::Rgb(50, 200, 150),
    Color::Rgb(150, 100, 255),
    Color::Rgb(255, 100, 150),
    Color::Rgb(100, 200, 255),
    Color::Rgb(255, 200, 100),
];

fn subpatch_color(index: usize) -> Color {
    SUBPATCH_COLORS[index % SUBPATCH_COLORS.len()]
}

fn pitch_to_note(pitch: u8) -> String {
    const NOTES: [&str; 12] = [
        "C ", "C#", "D ", "D#", "E ", "F ", "F#", "G ", "G#", "A ", "A#", "B ",
    ];
    let octave = (pitch / 12) as i8 - 1;
    let note = NOTES[(pitch % 12) as usize];
    format!("{}{}", note, octave)
}

impl App {
    fn new(cmd_tx: CommandSender, meter_rx: MeterReceiver, output_rx: OutputReceiver) -> Self {
        Self {
            instruments: vec![Instrument::new()],
            current_instrument: 0,
            mode: Mode::Normal,
            pending_request: None,
            palette_category: 0,
            palette_selections: [0; 9],
            palette_filter: String::new(),
            palette_filter_selection: 0,
            palette_searching: false,
            message: None,
            should_quit: false,
            dirty: false,
            cmd_tx,
            meter_rx,
            output_rx,
            show_meters: true,
            playing: false,
            file_path: None,
            save_input: Input::new("patch.bw".to_string()),
            probe_len: 4410,
            grid_area: Rect::default(),
            drag_start: None,
            dragging: false,
            select_start: None,
            bpm: 120.0,
            export_input: Input::new("output.wav".to_string()),
            export_loops: 1,
            output_history: VecDeque::with_capacity(128),
            value_input: Input::default(),
            step_size: 1,
            probe_voice: 0,
            brand_scroll: Animated::new(0.0)
                .duration(8000.0)
                .easing(Easing::EaseInOutCubic)
                .repeat_forever()
                .auto_reverse()
                .auto_start(1.0, Instant::now()),
            help_scroll: 0,
            last_mode_for_help: std::mem::discriminant(&Mode::Normal),
            active_pitches: Vec::new(),
        }
    }

    fn inst(&self) -> &Instrument {
        &self.instruments[self.current_instrument]
    }

    fn inst_mut(&mut self) -> &mut Instrument {
        &mut self.instruments[self.current_instrument]
    }

    fn cursor(&self) -> GridPos {
        self.inst().cursor
    }

    fn set_cursor(&mut self, pos: GridPos) {
        self.inst_mut().cursor = pos;
    }

    fn view_center(&self) -> GridPos {
        self.inst().view_center
    }

    fn set_view_center(&mut self, pos: GridPos) {
        self.inst_mut().view_center = pos;
    }

    fn scale_idx(&self) -> usize {
        self.inst().scale_idx
    }

    fn track_text(&self) -> &str {
        &self.inst().track_text
    }

    fn meter_values(&self) -> &HashMap<ModuleId, Vec<f32>> {
        &self.inst().meter_values
    }

    fn probe_values(&self) -> &HashMap<ModuleId, f32> {
        &self.inst().probe_values
    }

    fn editing_subpatch(&self) -> Option<SubPatchId> {
        self.inst().editing_subpatch
    }

    fn switch_instrument(&mut self, idx: usize) {
        if idx < self.instruments.len() && idx != self.current_instrument {
            self.current_instrument = idx;
            self.message = Some(format!("Instrument {}", idx + 1));
        }
    }

    fn add_instrument(&mut self) {
        if self.instruments.len() < 9 {
            self.instruments.push(Instrument::new());
            let new_idx = self.instruments.len() - 1;
            self.current_instrument = new_idx;
            self.send_compile(new_idx);
            self.message = Some(format!("New instrument {}", new_idx + 1));
        } else {
            self.message = Some("Max 9 instruments".into());
        }
    }

    fn send_compile(&self, inst_idx: usize) {
        let inst = &self.instruments[inst_idx];
        let scale = scale_from_idx(inst.scale_idx);
        let track = Track::parse(&inst.track_text, &scale).ok();
        let bars = track.as_ref().map(|t| t.bar_count() as f32).unwrap_or(1.0);
        let ctx = CompileContext {
            sample_rate: 44100.0,
            bpm: self.bpm,
            bars,
        };
        let voices = compile_voices(&inst.patches, NUM_VOICES, &ctx);
        let _ = self.cmd_tx.send(AudioCommand::SetInstrument {
            idx: inst_idx,
            voices,
            track,
            bars,
        });
    }

    fn send_compile_current(&self) {
        self.send_compile(self.current_instrument);
    }

    fn step_value(&self) -> f32 {
        const STEPS: [f32; 5] = [1.0, 0.1, 0.01, 0.001, 10.0];
        STEPS[self.step_size % STEPS.len()]
    }

    fn step_label(&self) -> &'static str {
        const LABELS: [&str; 5] = ["1", "0.1", "0.01", "0.001", "10"];
        LABELS[self.step_size % LABELS.len()]
    }

    fn round_to_step(&self, val: f32) -> f32 {
        let step = self.step_value();
        (val / step).round() * step
    }

    fn patches(&self) -> &PatchSet {
        &self.inst().patches
    }

    fn patches_mut(&mut self) -> &mut PatchSet {
        &mut self.inst_mut().patches
    }

    fn patch(&self) -> &Patch {
        let inst = self.inst();
        match inst.editing_subpatch {
            Some(sub_id) => inst
                .patches
                .subpatch(sub_id)
                .map(|s| &s.patch)
                .unwrap_or(&inst.patches.root),
            None => &inst.patches.root,
        }
    }

    fn patch_mut(&mut self) -> &mut Patch {
        let inst = self.inst_mut();
        match inst.editing_subpatch {
            Some(sub_id) if inst.patches.subpatches.contains_key(&sub_id) => {
                &mut inst.patches.subpatches.get_mut(&sub_id).unwrap().patch
            }
            _ => &mut inst.patches.root,
        }
    }

    fn drain_meters(&mut self) {
        let probe_len = self.probe_len;
        while let Ok(frame) = self.meter_rx.try_recv() {
            if frame.instrument_idx == self.current_instrument {
                let inst = self.inst_mut();
                for (id, values) in frame.ports {
                    inst.meter_values.insert(id, values);
                }
                for (id, samples) in frame.probes {
                    if let Some(&last) = samples.last() {
                        inst.probe_values.insert(id, last);
                    }
                    let history = inst.probe_histories.entry(id).or_default();
                    for val in samples {
                        history.push_back(val);
                    }
                    while history.len() > probe_len {
                        history.pop_front();
                    }
                }
                self.active_pitches = frame.active_pitches;
            }
        }
        while let Ok(level) = self.output_rx.try_recv() {
            self.output_history.push_back(level);
            if self.output_history.len() > 34 {
                self.output_history.pop_front();
            }
        }
    }

    fn reparse_track(&mut self) {
        let scale = scale_from_idx(self.scale_idx());
        match Track::parse(self.track_text(), &scale) {
            Ok(_) => {
                self.send_compile_current();
                self.message = Some("Track updated".into());
            }
            Err(e) => {
                self.message = Some(format!("Parse error: {}", e));
            }
        }
    }

    fn snapshot(&mut self) {
        self.inst_mut().snapshot();
    }

    fn commit_patch(&mut self) {
        self.snapshot();
        self.recompile_patch();
    }

    fn recompile_patch(&mut self) {
        self.send_compile_current();
        self.inst_mut().meter_values.clear();
        self.inst_mut().probe_values.clear();
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.inst_mut().undo_stack.pop() {
            let inst = self.inst_mut();
            inst.redo_stack.push(inst.patches.clone());
            inst.patches = prev;
            self.recompile_patch();
            self.message = Some("Undo".into());
        } else {
            self.message = Some("Nothing to undo".into());
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.inst_mut().redo_stack.pop() {
            let inst = self.inst_mut();
            inst.undo_stack.push(inst.patches.clone());
            inst.patches = next;
            self.recompile_patch();
            self.message = Some("Redo".into());
        } else {
            self.message = Some("Nothing to redo".into());
        }
    }

    fn move_cursor(&mut self, dx: i16, dy: i16) {
        let grid = self.patch().grid();
        let cursor = self.cursor();
        let new_x = (cursor.x as i16 + dx).clamp(0, grid.width() as i16 - 1) as u16;
        let new_y = (cursor.y as i16 + dy).clamp(0, grid.height() as i16 - 1) as u16;
        let new_pos = GridPos::new(new_x, new_y);
        self.set_cursor(new_pos);
        self.set_view_center(new_pos);
    }

    fn screen_to_grid(&self, col: u16, row: u16) -> Option<GridPos> {
        const CELL_WIDTH: u16 = 5;
        const CELL_HEIGHT: u16 = 3;
        const GUTTER_LEFT: u16 = 2;
        const GUTTER_TOP: u16 = 1;
        let grid_x = self.grid_area.x + GUTTER_LEFT;
        let grid_y = self.grid_area.y + GUTTER_TOP;
        if col < grid_x || row < grid_y {
            return None;
        }
        let grid_width = self.grid_area.width.saturating_sub(GUTTER_LEFT);
        let grid_height = self.grid_area.height.saturating_sub(GUTTER_TOP);
        let visible_cols = grid_width / CELL_WIDTH;
        let visible_rows = grid_height / CELL_HEIGHT;
        let half_cols = visible_cols / 2;
        let half_rows = visible_rows / 2;

        let grid = self.patch().grid();
        let origin_x = if self.view_center().x < half_cols {
            0
        } else if self.view_center().x + half_cols >= grid.width() as u16 {
            (grid.width() as u16).saturating_sub(visible_cols)
        } else {
            self.view_center().x - half_cols
        };
        let origin_y = if self.view_center().y < half_rows {
            0
        } else if self.view_center().y + half_rows >= grid.height() as u16 {
            (grid.height() as u16).saturating_sub(visible_rows)
        } else {
            self.view_center().y - half_rows
        };

        let vx = (col - grid_x) / CELL_WIDTH;
        let vy = (row - grid_y) / CELL_HEIGHT;
        let gx = origin_x + vx;
        let gy = origin_y + vy;

        if gx < grid.width() as u16 && gy < grid.height() as u16 {
            Some(GridPos::new(gx, gy))
        } else {
            None
        }
    }

    fn handle_mouse(&mut self, kind: MouseEventKind, col: u16, row: u16) {
        match &self.mode {
            Mode::Normal => self.handle_mouse_normal(kind, col, row),
            Mode::MouseSelect { anchor } => {
                let anchor = *anchor;
                self.handle_mouse_select(kind, col, row, anchor);
            }
            Mode::Select { anchor } => {
                let anchor = *anchor;
                self.handle_mouse_in_select(kind, col, row, anchor);
            }
            Mode::SelectMove {
                anchor,
                extent,
                move_origin,
            } => {
                let (anchor, extent, move_origin) = (*anchor, *extent, *move_origin);
                self.handle_mouse_select_move(kind, col, row, anchor, extent, move_origin);
            }
            _ => {}
        }
    }

    fn handle_mouse_normal(&mut self, kind: MouseEventKind, col: u16, row: u16) {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(pos) = self.screen_to_grid(col, row) {
                    self.set_cursor(pos);
                    if let Some(m) = self.patch().module_at(pos) {
                        self.drag_start = Some((m.id, pos));
                        self.dragging = false;
                    } else {
                        self.select_start = Some(pos);
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((id, start_pos)) = self.drag_start {
                    if let Some(pos) = self.screen_to_grid(col, row) {
                        let dx = (pos.x as i32 - start_pos.x as i32).abs();
                        let dy = (pos.y as i32 - start_pos.y as i32).abs();
                        if dx > 0 || dy > 0 {
                            self.dragging = true;
                        }
                        if self.dragging {
                            self.patch_mut().move_module(id, pos);
                            self.commit_patch();
                            self.set_cursor(pos);
                        }
                    }
                } else if let Some(start_pos) = self.select_start {
                    if let Some(pos) = self.screen_to_grid(col, row) {
                        let dx = (pos.x as i32 - start_pos.x as i32).abs();
                        let dy = (pos.y as i32 - start_pos.y as i32).abs();
                        if dx > 0 || dy > 0 {
                            self.select_start = None;
                            self.mode = Mode::MouseSelect { anchor: start_pos };
                            self.set_cursor(pos);
                        }
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_start = None;
                self.dragging = false;
                self.select_start = None;
                self.set_view_center(self.cursor());
            }
            _ => {}
        }
    }

    fn handle_mouse_select(&mut self, kind: MouseEventKind, col: u16, row: u16, anchor: GridPos) {
        match kind {
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(pos) = self.screen_to_grid(col, row) {
                    self.set_cursor(pos);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.set_view_center(self.cursor());
                if anchor == self.cursor() {
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Select { anchor };
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_in_select(
        &mut self,
        kind: MouseEventKind,
        col: u16,
        row: u16,
        anchor: GridPos,
    ) {
        let extent = self.cursor();
        let (min_x, max_x) = (anchor.x.min(extent.x), anchor.x.max(extent.x));
        let (min_y, max_y) = (anchor.y.min(extent.y), anchor.y.max(extent.y));

        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(pos) = self.screen_to_grid(col, row) {
                    let in_selection =
                        pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y;
                    if in_selection {
                        self.mode = Mode::SelectMove {
                            anchor: GridPos::new(min_x, min_y),
                            extent: GridPos::new(max_x, max_y),
                            move_origin: pos,
                        };
                        self.set_cursor(pos);
                    } else {
                        self.mode = Mode::Normal;
                        self.set_cursor(pos);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_select_move(
        &mut self,
        kind: MouseEventKind,
        col: u16,
        row: u16,
        anchor: GridPos,
        extent: GridPos,
        move_origin: GridPos,
    ) {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(pos) = self.screen_to_grid(col, row) {
                    let in_selection = pos.x >= anchor.x
                        && pos.x <= extent.x
                        && pos.y >= anchor.y
                        && pos.y <= extent.y;
                    if in_selection {
                        self.mode = Mode::SelectMove {
                            anchor,
                            extent,
                            move_origin: pos,
                        };
                        self.set_cursor(pos);
                    } else {
                        self.mode = Mode::Normal;
                        self.set_cursor(pos);
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(pos) = self.screen_to_grid(col, row) {
                    let dx = pos.x as i16 - move_origin.x as i16;
                    let dy = pos.y as i16 - move_origin.y as i16;
                    if dx != 0 || dy != 0 {
                        let mut unique_ids = std::collections::HashSet::new();
                        for x in anchor.x..=extent.x {
                            for y in anchor.y..=extent.y {
                                if let Some(m) = self.patch().module_at(GridPos::new(x, y)) {
                                    unique_ids.insert(m.id);
                                }
                            }
                        }
                        let mut moves = Vec::new();
                        for &id in &unique_ids {
                            if let Some(old_pos) = self.patch().module_position(id) {
                                let new_pos = GridPos::new(
                                    (old_pos.x as i16 + dx).max(0) as u16,
                                    (old_pos.y as i16 + dy).max(0) as u16,
                                );
                                moves.push((id, new_pos));
                            }
                        }
                        let moved = self.patch_mut().move_modules(&moves);
                        if moved > 0 {
                            self.commit_patch();
                            let new_anchor = GridPos::new(
                                (anchor.x as i16 + dx).max(0) as u16,
                                (anchor.y as i16 + dy).max(0) as u16,
                            );
                            let new_extent = GridPos::new(
                                (extent.x as i16 + dx).max(0) as u16,
                                (extent.y as i16 + dy).max(0) as u16,
                            );
                            self.mode = Mode::SelectMove {
                                anchor: new_anchor,
                                extent: new_extent,
                                move_origin: pos,
                            };
                        }
                        self.set_cursor(pos);
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.set_view_center(self.cursor());
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.message = None;

        let current_mode = std::mem::discriminant(&self.mode);
        if current_mode != self.last_mode_for_help {
            self.help_scroll = 0;
            self.last_mode_for_help = current_mode;
        }

        match self.mode.clone() {
            Mode::Normal => self.handle_normal_key(code),
            Mode::Palette => self.handle_palette_key(code),
            Mode::Move { module_id, origin } => self.handle_move_key(code, module_id, origin),
            Mode::Copy { module } => self.handle_copy_key(code, module),
            Mode::CopySelection { modules, origin } => {
                self.handle_copy_selection_key(code, modules, origin)
            }
            Mode::Select { anchor } => self.handle_select_key(code, anchor),
            Mode::SelectMove {
                anchor,
                extent,
                move_origin,
            } => self.handle_select_move_key(code, anchor, extent, move_origin),
            Mode::MouseSelect { anchor } => self.handle_select_key(code, anchor),
            Mode::Edit {
                module_id,
                param_idx,
            } => self.handle_edit_key(code, module_id, param_idx),
            Mode::ValueInput {
                module_id,
                param_idx,
            } => self.handle_value_input_key(code, module_id, param_idx),
            Mode::AdsrEdit {
                module_id,
                param_idx,
            } => self.handle_adsr_edit_key(code, module_id, param_idx),
            Mode::EnvEdit {
                module_id,
                point_idx,
                editing,
            } => self.handle_env_edit_key(code, module_id, point_idx, editing),
            Mode::ProbeEdit { module_id } => self.handle_probe_edit_key(code, module_id),
            Mode::SampleView {
                module_id,
                zoom,
                offset,
            } => self.handle_sample_view_key(code, module_id, zoom, offset),
            Mode::SavePrompt => self.handle_save_prompt_key(code, modifiers),
            Mode::SaveConfirm => self.handle_save_confirm_key(code),
            Mode::QuitConfirm => self.handle_quit_confirm_key(code),
            Mode::TrackSettings { param_idx } => self.handle_track_settings_key(code, param_idx),
            Mode::ExportPrompt => self.handle_export_prompt_key(code),
            Mode::ExportConfirm => self.handle_export_confirm_key(code),
        }
    }

    fn handle_quit_confirm_key(&mut self, code: KeyCode) {
        let Some(action) = lookup(bindings::quit_confirm_bindings(), code) else {
            return;
        };
        match action {
            Action::Confirm => self.should_quit = true,
            Action::Cancel => {
                self.mode = Mode::Normal;
                self.message = Some("Quit cancelled".into());
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        let Some(action) = lookup(bindings::normal_bindings(), code) else {
            return;
        };
        match action {
            Action::Quit => {
                if self.dirty {
                    self.mode = Mode::QuitConfirm;
                    self.message = Some("Unsaved changes. Quit? (y/n)".into());
                } else {
                    self.should_quit = true;
                }
            }
            Action::Left => self.move_cursor(-1, 0),
            Action::Down => self.move_cursor(0, 1),
            Action::Up => self.move_cursor(0, -1),
            Action::Right => self.move_cursor(1, 0),
            Action::LeftFast => self.move_cursor(-4, 0),
            Action::DownFast => self.move_cursor(0, 4),
            Action::UpFast => self.move_cursor(0, -4),
            Action::RightFast => self.move_cursor(4, 0),
            Action::Place => {
                self.mode = Mode::Palette;
            }
            Action::Move => {
                if let Some(id) = self.patch().module_id_at(self.cursor()) {
                    self.mode = Mode::Move {
                        module_id: id,
                        origin: self.cursor(),
                    };
                }
            }
            Action::Delete => {
                if let Some(id) = self.patch().module_id_at(self.cursor()) {
                    if self.patch_mut().remove_module(id) {
                        self.message = Some("Deleted".into());
                        self.commit_patch();
                    }
                }
            }
            Action::Rotate => {
                if let Some(id) = self.patch().module_id_at(self.cursor()) {
                    if let Some(m) = self.patch().module(id) {
                        if m.kind.is_routing() {
                            self.message = Some("Cannot rotate".into());
                        } else if self.patch_mut().rotate_module(id) {
                            self.message = Some("Rotated".into());
                            self.commit_patch();
                        } else {
                            self.message = Some("No room to rotate".into());
                        }
                    }
                }
            }
            Action::Edit => {
                if let Some(id) = self.patch().module_id_at(self.cursor()) {
                    if self.patch().module(id).is_some() {
                        self.mode = Mode::Edit {
                            module_id: id,
                            param_idx: 0,
                        };
                    }
                }
            }
            Action::Copy => {
                if let Some(id) = self.patch().module_id_at(self.cursor()) {
                    if let Some(m) = self.patch().module(id).cloned() {
                        self.mode = Mode::Copy { module: m };
                        self.message = Some("Place copy with space/enter".into());
                    }
                }
            }
            Action::Palette(cat) => self.open_palette_category(cat),
            Action::OpenPalette => {
                self.mode = Mode::Palette;
            }
            Action::TogglePlay => {
                self.playing = !self.playing;
                self.message = Some(if self.playing {
                    "Playing".into()
                } else {
                    "Paused".into()
                });
            }
            Action::TrackEdit => {
                self.pending_request = Some(AppRequest::EditTrack);
            }
            Action::Select => {
                self.mode = Mode::Select {
                    anchor: self.cursor(),
                };
            }
            Action::Save => {
                if let Some(ref path) = self.file_path {
                    self.save_to_file(path.clone());
                } else {
                    self.save_input = Input::new("patch.bw".to_string());
                    self.mode = Mode::SavePrompt;
                }
            }
            Action::SaveAs => {
                let name = self
                    .file_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "patch.bw".into());
                self.save_input = Input::new(name);
                self.mode = Mode::SavePrompt;
            }
            Action::Export => {
                let name = self
                    .file_path
                    .as_ref()
                    .map(|p| p.with_extension("wav").to_string_lossy().to_string())
                    .unwrap_or_else(|| "output.wav".into());
                self.export_input = Input::new(name);
                self.mode = Mode::ExportPrompt;
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::TrackSettings => {
                self.mode = Mode::TrackSettings { param_idx: 0 };
            }
            Action::EditSubpatch => {
                let on_subpatch = self
                    .patch()
                    .module_id_at(self.cursor())
                    .and_then(|id| self.patch().module(id))
                    .and_then(|m| match m.kind {
                        ModuleKind::SubPatch(sub_id) => Some(sub_id),
                        _ => None,
                    });
                if let Some(sub_id) = on_subpatch {
                    let name = self
                        .patches()
                        .subpatch(sub_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_default();
                    let inst = self.inst_mut();
                    inst.subpatch_stack
                        .push((inst.editing_subpatch, inst.cursor));
                    inst.editing_subpatch = Some(sub_id);
                    inst.cursor = GridPos::new(0, 0);
                    self.message = Some(format!("Editing '{}'", name));
                } else if let Some(sub_id) = self.inst_mut().editing_subpatch.take() {
                    let name = self
                        .patches()
                        .subpatch(sub_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_default();
                    self.sync_subpatch_ports(sub_id);
                    if let Some((parent, cursor)) = self.inst_mut().subpatch_stack.pop() {
                        self.inst_mut().editing_subpatch = parent;
                        self.set_cursor(cursor);
                        self.set_view_center(cursor);
                    }
                    self.message = Some(format!("Exited '{}'", name));
                    self.commit_patch();
                }
            }
            Action::ExitSubpatch => {
                if let Some(sub_id) = self.inst_mut().editing_subpatch.take() {
                    let name = self
                        .patches()
                        .subpatch(sub_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_default();
                    self.sync_subpatch_ports(sub_id);
                    if let Some((parent, cursor)) = self.inst_mut().subpatch_stack.pop() {
                        self.inst_mut().editing_subpatch = parent;
                        self.set_cursor(cursor);
                        self.set_view_center(cursor);
                    }
                    self.message = Some(format!("Exited '{}'", name));
                    self.commit_patch();
                }
            }
            Action::ToggleMeters => {
                self.show_meters = !self.show_meters;
            }
            Action::Instrument(idx) => {
                if idx < self.instruments.len() {
                    self.switch_instrument(idx);
                    self.message = Some(format!("Instrument {}", idx + 1));
                }
            }
            Action::NewInstrument => {
                self.add_instrument();
                self.message = Some(format!("New instrument {}", self.instruments.len()));
            }
            Action::HelpScrollUp => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            Action::HelpScrollDown => {
                self.help_scroll += 1;
            }
            _ => {}
        }
    }

    fn open_palette_category(&mut self, cat: usize) {
        self.palette_category = cat;
        self.mode = Mode::Palette;
    }

    fn filtered_modules(&self) -> Vec<ModuleKind> {
        if self.palette_filter.is_empty() {
            return Vec::new();
        }
        let filter = self.palette_filter.to_lowercase();
        ModuleKind::all()
            .iter()
            .copied()
            .filter(|k| k.name().to_lowercase().contains(&filter))
            .collect()
    }

    fn handle_palette_key(&mut self, code: KeyCode) {
        if self.palette_searching {
            self.handle_palette_search_key(code);
            return;
        }

        let Some(action) = lookup(bindings::palette_bindings(), code) else {
            return;
        };

        let categories = ModuleCategory::all();
        let current_cat = categories[self.palette_category];
        let modules = ModuleKind::by_category(current_cat);
        let palette_module = self.palette_selections[self.palette_category];

        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::Search => {
                self.palette_filter.clear();
                self.palette_filter_selection = 0;
                self.palette_searching = true;
            }
            Action::Down => {
                if palette_module + 1 < modules.len() {
                    self.palette_selections[self.palette_category] += 1;
                } else if self.palette_category + 1 < categories.len() {
                    self.palette_category += 1;
                }
            }
            Action::Up => {
                if palette_module > 0 {
                    self.palette_selections[self.palette_category] -= 1;
                } else if self.palette_category > 0 {
                    self.palette_category -= 1;
                }
            }
            Action::Right => {
                self.palette_category = (self.palette_category + 1) % categories.len();
            }
            Action::Left => {
                self.palette_category = if self.palette_category == 0 {
                    categories.len() - 1
                } else {
                    self.palette_category - 1
                };
            }
            Action::Confirm => {
                let cursor = self.cursor();
                if let Some(kind) = modules.get(palette_module) {
                    if *kind == ModuleKind::Output && self.patch().output_id().is_some() {
                        self.message = Some("Output exists".into());
                    } else if matches!(kind, ModuleKind::SubPatch(_)) {
                        let color = subpatch_color(self.patches().subpatches.len());
                        let sub_id = self.patches_mut().create_subpatch("Sub".into(), color);
                        if self
                            .patch_mut()
                            .add_module(ModuleKind::SubPatch(sub_id), cursor)
                            .is_some()
                        {
                            self.message = Some("SubPatch placed".into());
                            self.commit_patch();
                        } else {
                            self.message = Some("Can't place here".into());
                        }
                    } else if matches!(kind, ModuleKind::DelayTap(_)) {
                        let delay_id = self
                            .patch()
                            .all_modules()
                            .find(|m| m.kind == ModuleKind::Delay)
                            .map(|m| m.id);
                        if let Some(delay_id) = delay_id {
                            if self
                                .patch_mut()
                                .add_module(ModuleKind::DelayTap(delay_id), cursor)
                                .is_some()
                            {
                                self.message = Some("DelayTap placed".into());
                                self.commit_patch();
                            } else {
                                self.message = Some("Can't place here".into());
                            }
                        } else {
                            self.message = Some("No Delay module found".into());
                        }
                    } else if self.patch_mut().add_module(*kind, cursor).is_some() {
                        self.message = Some(format!("{} placed", kind.name()));
                        self.commit_patch();
                    } else {
                        self.message = Some("Can't place here".into());
                    }
                }
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn handle_palette_search_key(&mut self, code: KeyCode) {
        let filtered = self.filtered_modules();
        match code {
            KeyCode::Esc => {
                self.palette_filter.clear();
                self.palette_filter_selection = 0;
                self.palette_searching = false;
            }
            KeyCode::Backspace => {
                self.palette_filter.pop();
                self.palette_filter_selection = 0;
            }
            KeyCode::Down => {
                if self.palette_filter_selection + 1 < filtered.len() {
                    self.palette_filter_selection += 1;
                }
            }
            KeyCode::Up => {
                if self.palette_filter_selection > 0 {
                    self.palette_filter_selection -= 1;
                }
            }
            KeyCode::Enter => {
                let cursor = self.cursor();
                if let Some(kind) = filtered.get(self.palette_filter_selection) {
                    if *kind == ModuleKind::Output && self.patch().output_id().is_some() {
                        self.message = Some("Output exists".into());
                    } else if matches!(kind, ModuleKind::SubPatch(_)) {
                        let color = subpatch_color(self.patches().subpatches.len());
                        let sub_id = self.patches_mut().create_subpatch("Sub".into(), color);
                        if self
                            .patch_mut()
                            .add_module(ModuleKind::SubPatch(sub_id), cursor)
                            .is_some()
                        {
                            self.message = Some("SubPatch placed".into());
                            self.commit_patch();
                        } else {
                            self.message = Some("Can't place here".into());
                        }
                    } else if matches!(kind, ModuleKind::DelayTap(_)) {
                        let delay_id = self
                            .patch()
                            .all_modules()
                            .find(|m| m.kind == ModuleKind::Delay)
                            .map(|m| m.id);
                        if let Some(delay_id) = delay_id {
                            if self
                                .patch_mut()
                                .add_module(ModuleKind::DelayTap(delay_id), cursor)
                                .is_some()
                            {
                                self.message = Some("DelayTap placed".into());
                                self.commit_patch();
                            } else {
                                self.message = Some("Can't place here".into());
                            }
                        } else {
                            self.message = Some("No Delay module found".into());
                        }
                    } else if self.patch_mut().add_module(*kind, cursor).is_some() {
                        self.message = Some(format!("{} placed", kind.name()));
                        self.commit_patch();
                    } else {
                        self.message = Some("Can't place here".into());
                    }
                }
                self.palette_filter.clear();
                self.palette_filter_selection = 0;
                self.palette_searching = false;
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                self.palette_filter.push(c);
                self.palette_filter_selection = 0;
            }
            _ => {}
        }
    }

    fn handle_move_key(&mut self, code: KeyCode, module_id: ModuleId, origin: GridPos) {
        let Some(action) = lookup(bindings::move_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.set_cursor(origin);
                self.mode = Mode::Normal;
                self.message = Some("Move cancelled".into());
            }
            Action::Confirm => {
                let cursor = self.cursor();
                if self.patch_mut().move_module(module_id, cursor) {
                    self.mode = Mode::Normal;
                    self.message = Some("Moved".into());
                    self.commit_patch();
                } else {
                    self.message = Some("Can't place here".into());
                }
            }
            Action::Left => self.move_cursor(-1, 0),
            Action::Down => self.move_cursor(0, 1),
            Action::Up => self.move_cursor(0, -1),
            Action::Right => self.move_cursor(1, 0),
            Action::LeftFast => self.move_cursor(-4, 0),
            Action::DownFast => self.move_cursor(0, 4),
            Action::UpFast => self.move_cursor(0, -4),
            Action::RightFast => self.move_cursor(4, 0),
            _ => {}
        }
    }

    fn handle_copy_key(&mut self, code: KeyCode, module: Module) {
        let Some(action) = lookup(bindings::move_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
                self.message = Some("Copy cancelled".into());
            }
            Action::Confirm => {
                let cursor = self.cursor();
                if let Some(_new_id) = self.patch_mut().add_module_clone(&module, cursor) {
                    self.mode = Mode::Normal;
                    self.message = Some("Placed".into());
                    self.commit_patch();
                } else {
                    self.message = Some("Can't place here".into());
                }
            }
            Action::Left => self.move_cursor(-1, 0),
            Action::Down => self.move_cursor(0, 1),
            Action::Up => self.move_cursor(0, -1),
            Action::Right => self.move_cursor(1, 0),
            Action::LeftFast => self.move_cursor(-4, 0),
            Action::DownFast => self.move_cursor(0, 4),
            Action::UpFast => self.move_cursor(0, -4),
            Action::RightFast => self.move_cursor(4, 0),
            _ => {}
        }
    }

    fn handle_copy_selection_key(
        &mut self,
        code: KeyCode,
        modules: Vec<(Module, GridPos)>,
        origin: GridPos,
    ) {
        let Some(action) = lookup(bindings::move_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
                self.message = Some("Copy cancelled".into());
            }
            Action::Confirm => {
                let dx = self.cursor().x as i16 - origin.x as i16;
                let dy = self.cursor().y as i16 - origin.y as i16;
                let mut placed = 0;
                for (module, pos) in &modules {
                    let new_x = (pos.x as i16 + dx).max(0) as u16;
                    let new_y = (pos.y as i16 + dy).max(0) as u16;
                    let new_pos = GridPos::new(new_x, new_y);
                    if self.patch_mut().add_module_clone(module, new_pos).is_some() {
                        placed += 1;
                    }
                }
                self.mode = Mode::Normal;
                self.message = Some(format!("Placed {} copies", placed));
                if placed > 0 {
                    self.commit_patch();
                }
            }
            Action::Left => self.move_cursor(-1, 0),
            Action::Down => self.move_cursor(0, 1),
            Action::Up => self.move_cursor(0, -1),
            Action::Right => self.move_cursor(1, 0),
            Action::LeftFast => self.move_cursor(-4, 0),
            Action::DownFast => self.move_cursor(0, 4),
            Action::UpFast => self.move_cursor(0, -4),
            Action::RightFast => self.move_cursor(4, 0),
            _ => {}
        }
    }

    fn handle_select_key(&mut self, code: KeyCode, anchor: GridPos) {
        let Some(action) = lookup(bindings::select_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::Left => self.move_cursor(-1, 0),
            Action::Down => self.move_cursor(0, 1),
            Action::Up => self.move_cursor(0, -1),
            Action::Right => self.move_cursor(1, 0),
            Action::LeftFast => self.move_cursor(-4, 0),
            Action::DownFast => self.move_cursor(0, 4),
            Action::UpFast => self.move_cursor(0, -4),
            Action::RightFast => self.move_cursor(4, 0),
            Action::Move => {
                self.mode = Mode::SelectMove {
                    anchor,
                    extent: self.cursor(),
                    move_origin: self.cursor(),
                };
            }
            Action::Delete => {
                let ids = self.modules_in_rect(anchor, self.cursor());
                let count = ids.len();
                for id in ids {
                    self.patch_mut().remove_module(id);
                }
                self.mode = Mode::Normal;
                self.message = Some(format!("Deleted {} modules", count));
                self.commit_patch();
            }
            Action::Copy => {
                let ids = self.modules_in_rect(anchor, self.cursor());
                if ids.is_empty() {
                    self.message = Some("No modules to copy".into());
                    return;
                }
                let modules: Vec<(Module, GridPos)> = ids
                    .iter()
                    .filter_map(|id| {
                        let m = self.patch().module(*id)?.clone();
                        let pos = self.patch().module_position(*id)?;
                        Some((m, pos))
                    })
                    .collect();
                if !modules.is_empty() {
                    self.mode = Mode::CopySelection {
                        modules,
                        origin: self.cursor(),
                    };
                    self.message = Some("Place copies with space/enter".into());
                }
            }
            Action::MakeSubpatch => {
                self.create_subpatch_from_selection(anchor);
            }
            _ => {}
        }
    }

    fn handle_select_move_key(
        &mut self,
        code: KeyCode,
        anchor: GridPos,
        extent: GridPos,
        move_origin: GridPos,
    ) {
        let Some(action) = lookup(bindings::move_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.set_cursor(move_origin);
                self.mode = Mode::Normal;
                self.message = Some("Move cancelled".into());
            }
            Action::Left => self.move_cursor(-1, 0),
            Action::Down => self.move_cursor(0, 1),
            Action::Up => self.move_cursor(0, -1),
            Action::Right => self.move_cursor(1, 0),
            Action::LeftFast => self.move_cursor(-4, 0),
            Action::DownFast => self.move_cursor(0, 4),
            Action::UpFast => self.move_cursor(0, -4),
            Action::RightFast => self.move_cursor(4, 0),
            Action::Confirm => {
                let dx = self.cursor().x as i16 - move_origin.x as i16;
                let dy = self.cursor().y as i16 - move_origin.y as i16;
                let ids = self.modules_in_rect(anchor, extent);
                let moves: Vec<_> = ids
                    .iter()
                    .filter_map(|id| {
                        let pos = self.patch().module_position(*id)?;
                        let new_x = (pos.x as i16 + dx).max(0) as u16;
                        let new_y = (pos.y as i16 + dy).max(0) as u16;
                        Some((*id, GridPos::new(new_x, new_y)))
                    })
                    .collect();
                let moved = self.patch_mut().move_modules(&moves);
                self.mode = Mode::Normal;
                self.message = Some(format!("Moved {} modules", moved));
                self.commit_patch();
            }
            _ => {}
        }
    }
    fn create_subpatch_from_selection(&mut self, anchor: GridPos) {
        let ids = self.modules_in_rect(anchor, self.cursor());
        if ids.is_empty() {
            self.message = Some("No modules selected".into());
            return;
        }

        let modules: Vec<(Module, GridPos)> = ids
            .iter()
            .filter_map(|id| {
                let m = self.patch().module(*id)?.clone();
                let pos = self.patch().module_position(*id)?;
                Some((m, pos))
            })
            .collect();

        let min_x = modules.iter().map(|(_, p)| p.x).min().unwrap_or(0);
        let min_y = modules.iter().map(|(_, p)| p.y).min().unwrap_or(0);

        let name = format!("Sub{}", self.patches().subpatches.len());
        let color = subpatch_color(self.patches().subpatches.len());
        let sub_id = self.patches_mut().create_subpatch(name.clone(), color);

        if let Some(subpatch) = self.patches_mut().subpatch_mut(sub_id) {
            for (module, pos) in &modules {
                let new_pos = GridPos::new(pos.x - min_x, pos.y - min_y);
                subpatch.patch.add_module_clone(module, new_pos);
            }
        }

        for id in &ids {
            self.patch_mut().remove_module(*id);
        }

        let place_pos = GridPos::new(min_x, min_y);
        self.patch_mut()
            .add_module(ModuleKind::SubPatch(sub_id), place_pos);

        self.sync_subpatch_ports(sub_id);
        self.mode = Mode::Normal;
        self.message = Some(format!("Created subpatch '{}'", name));
        self.commit_patch();
    }

    fn sync_subpatch_ports(&mut self, sub_id: SubPatchId) {
        let (inputs, outputs) = if let Some(sub) = self.patches().subpatch(sub_id) {
            (sub.input_count() as u8, sub.output_count() as u8)
        } else {
            return;
        };

        let ids: Vec<ModuleId> = self
            .patches()
            .root
            .all_modules()
            .filter(|m| m.kind == ModuleKind::SubPatch(sub_id))
            .map(|m| m.id)
            .collect();

        for id in ids {
            if let Some(m) = self.patches_mut().root.module_mut(id) {
                m.params = ModuleParams::SubPatch { inputs, outputs };
            }
            self.patches_mut().root.refit_module(id);
        }
    }

    fn modules_in_rect(&self, a: GridPos, b: GridPos) -> Vec<ModuleId> {
        let sel_min_x = a.x.min(b.x);
        let sel_max_x = a.x.max(b.x);
        let sel_min_y = a.y.min(b.y);
        let sel_max_y = a.y.max(b.y);

        self.patch()
            .all_modules()
            .filter_map(|m| {
                let pos = self.patch().module_position(m.id)?;
                let mod_min_x = pos.x;
                let mod_min_y = pos.y;
                let mod_max_x = pos.x + m.width() as u16 - 1;
                let mod_max_y = pos.y + m.height() as u16 - 1;

                let overlaps = mod_min_x <= sel_max_x
                    && mod_max_x >= sel_min_x
                    && mod_min_y <= sel_max_y
                    && mod_max_y >= sel_min_y;

                if overlaps {
                    Some(m.id)
                } else {
                    None
                }
            })
            .collect()
    }

    fn handle_edit_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        let Some(module) = self.patch().module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let defs = module.kind.param_defs();
        let has_special = module.kind.has_special_editor();
        let total_items = defs.len() + if has_special { 1 } else { 0 };
        let special_idx = defs.len();

        let Some(action) = lookup(bindings::edit_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::Down => {
                let new_idx = (param_idx + 1) % total_items;
                self.mode = Mode::Edit {
                    module_id,
                    param_idx: new_idx,
                };
            }
            Action::Up => {
                let new_idx = if param_idx == 0 {
                    total_items - 1
                } else {
                    param_idx - 1
                };
                self.mode = Mode::Edit {
                    module_id,
                    param_idx: new_idx,
                };
            }
            Action::Confirm => {
                if has_special && param_idx == special_idx {
                    match module.kind {
                        ModuleKind::Envelope => {
                            self.mode = Mode::EnvEdit {
                                module_id,
                                point_idx: 0,
                                editing: false,
                            };
                        }
                        ModuleKind::Probe => {
                            self.mode = Mode::ProbeEdit { module_id };
                        }
                        ModuleKind::Sample => {
                            self.mode = Mode::SampleView {
                                module_id,
                                zoom: 1.0,
                                offset: 0.0,
                            };
                        }
                        _ => {}
                    }
                }
            }
            Action::ValueDown => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if matches!(module.kind, ModuleKind::DelayTap(_)) && param_idx == 0 {
                        self.cycle_delay_tap_source(module_id, false);
                    } else if matches!(module.kind, ModuleKind::Sample) && param_idx == 0 {
                        self.cycle_sample_file(module_id, false);
                    } else {
                        let step = self.step_value();
                        if let Some(m) = self.patch_mut().module_mut(module_id) {
                            match &def.kind {
                                ParamKind::Float { min, .. } => {
                                    let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                                    m.params.set_float(param_idx, (cur - step).max(*min));
                                    m.params.set_connected(param_idx, false);
                                }
                                ParamKind::Time => {
                                    if let Some(t) = m.params.get_time_mut(param_idx) {
                                        t.adjust(false, false);
                                    }
                                    m.params.set_connected(param_idx, false);
                                }
                                ParamKind::Enum => {
                                    m.params.cycle_enum_prev(param_idx);
                                }
                                ParamKind::Toggle => {
                                    m.params.toggle(param_idx);
                                }
                                ParamKind::Int { min, .. } => {
                                    let cur = m.params.get_int(param_idx).unwrap_or(0);
                                    m.params.set_int(param_idx, (cur - 1).max(*min));
                                }
                                ParamKind::Input => {}
                            }
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::ValueUp => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if matches!(module.kind, ModuleKind::DelayTap(_)) && param_idx == 0 {
                        self.cycle_delay_tap_source(module_id, true);
                    } else if matches!(module.kind, ModuleKind::Sample) && param_idx == 0 {
                        self.cycle_sample_file(module_id, true);
                    } else {
                        let step = self.step_value();
                        if let Some(m) = self.patch_mut().module_mut(module_id) {
                            match &def.kind {
                                ParamKind::Float { max, .. } => {
                                    let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                                    m.params.set_float(param_idx, (cur + step).min(*max));
                                    m.params.set_connected(param_idx, false);
                                }
                                ParamKind::Time => {
                                    if let Some(t) = m.params.get_time_mut(param_idx) {
                                        t.adjust(true, false);
                                    }
                                    m.params.set_connected(param_idx, false);
                                }
                                ParamKind::Enum => {
                                    m.params.cycle_enum_next(param_idx);
                                }
                                ParamKind::Toggle => {
                                    m.params.toggle(param_idx);
                                }
                                ParamKind::Int { max, .. } => {
                                    let cur = m.params.get_int(param_idx).unwrap_or(0);
                                    m.params.set_int(param_idx, (cur + 1).min(*max));
                                }
                                ParamKind::Input => {}
                            }
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::ValueDownFast => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    let step = self.step_value() * 10.0;
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        match &def.kind {
                            ParamKind::Float { min, .. } => {
                                let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                                m.params.set_float(param_idx, (cur - step).max(*min));
                                m.params.set_connected(param_idx, false);
                            }
                            ParamKind::Time => {
                                if let Some(t) = m.params.get_time_mut(param_idx) {
                                    t.adjust(false, true);
                                }
                                m.params.set_connected(param_idx, false);
                            }
                            _ => {}
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::ValueUpFast => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    let step = self.step_value() * 10.0;
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        match &def.kind {
                            ParamKind::Float { max, .. } => {
                                let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                                m.params.set_float(param_idx, (cur + step).min(*max));
                                m.params.set_connected(param_idx, false);
                            }
                            ParamKind::Time => {
                                if let Some(t) = m.params.get_time_mut(param_idx) {
                                    t.adjust(true, true);
                                }
                                m.params.set_connected(param_idx, false);
                            }
                            _ => {}
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::CycleUnit => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if matches!(def.kind, ParamKind::Time) {
                        if let Some(m) = self.patch_mut().module_mut(module_id) {
                            if let Some(t) = m.params.get_time_mut(param_idx) {
                                t.unit = t.unit.next();
                            }
                        }
                        self.commit_patch();
                    }
                }
            }
            Action::TogglePort => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if !matches!(def.kind, ParamKind::Input) {
                        if let Some(m) = self.patch_mut().module_mut(module_id) {
                            m.params.toggle_connected(param_idx);
                        }
                        self.patch_mut().rebuild_channels();
                        self.commit_patch();
                    }
                }
            }
            Action::TypeValue => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if matches!(def.kind, ParamKind::Float { .. }) {
                        let current = module.params.get_float(param_idx).unwrap_or(0.0);
                        self.value_input = Input::new(format!("{}", current));
                        self.mode = Mode::ValueInput {
                            module_id,
                            param_idx,
                        };
                    }
                }
            }
            Action::CycleStep => {
                let cur_val = if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if matches!(def.kind, ParamKind::Float { .. }) {
                        module.params.get_float(param_idx)
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.step_size = (self.step_size + 1) % 5;
                if let Some(val) = cur_val {
                    let rounded = self.round_to_step(val);
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        m.params.set_float(param_idx, rounded);
                    }
                    self.commit_patch();
                }
                self.message = Some(format!("Step: {}", self.step_label()));
            }
            _ => {}
        }
    }

    fn handle_value_input_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Edit {
                    module_id,
                    param_idx,
                };
            }
            KeyCode::Enter => {
                if let Ok(val) = self.value_input.value().parse::<f32>() {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        m.params.set_float(param_idx, val);
                        m.params.set_connected(param_idx, false);
                    }
                    self.commit_patch();
                }
                self.mode = Mode::Edit {
                    module_id,
                    param_idx,
                };
            }
            KeyCode::Backspace => {
                self.value_input.handle(InputRequest::DeletePrevChar);
            }
            KeyCode::Delete => {
                self.value_input.handle(InputRequest::DeleteNextChar);
            }
            KeyCode::Left => {
                self.value_input.handle(InputRequest::GoToPrevChar);
            }
            KeyCode::Right => {
                self.value_input.handle(InputRequest::GoToNextChar);
            }
            KeyCode::Home => {
                self.value_input.handle(InputRequest::GoToStart);
            }
            KeyCode::End => {
                self.value_input.handle(InputRequest::GoToEnd);
            }
            KeyCode::Char(c) => {
                if c.is_ascii_digit() || c == '.' || c == '-' {
                    self.value_input.handle(InputRequest::InsertChar(c));
                }
            }
            _ => {}
        }
    }

    fn cycle_sample_file(&mut self, module_id: ModuleId, forward: bool) {
        let files = scan_wav_files();
        if files.is_empty() {
            return;
        }

        let current_idx = if let Some(m) = self.patch().module(module_id) {
            if let ModuleParams::Sample { file_idx, .. } = &m.params {
                *file_idx
            } else {
                0
            }
        } else {
            0
        };

        let new_idx = if forward {
            (current_idx + 1) % files.len()
        } else {
            if current_idx == 0 {
                files.len() - 1
            } else {
                current_idx - 1
            }
        };

        let new_name = files.get(new_idx).cloned().unwrap_or_default();
        let new_samples = files
            .get(new_idx)
            .map(|p| load_wav_samples(p))
            .unwrap_or_else(|| Arc::new(Vec::new()));

        if let Some(m) = self.patch_mut().module_mut(module_id) {
            if let ModuleParams::Sample {
                file_idx,
                file_name,
                samples,
                ..
            } = &mut m.params
            {
                *file_idx = new_idx;
                *file_name = new_name;
                *samples = new_samples;
            }
        }
    }

    fn cycle_delay_tap_source(&mut self, tap_id: ModuleId, forward: bool) {
        let delays: Vec<ModuleId> = self
            .patch()
            .all_modules()
            .filter(|m| m.kind == ModuleKind::Delay)
            .map(|m| m.id)
            .collect();

        if delays.is_empty() {
            return;
        }

        let current_delay = if let Some(m) = self.patch().module(tap_id) {
            if let ModuleKind::DelayTap(id) = m.kind {
                Some(id)
            } else {
                None
            }
        } else {
            None
        };

        let current_idx = current_delay
            .and_then(|id| delays.iter().position(|&d| d == id))
            .unwrap_or(0);

        let new_idx = if forward {
            (current_idx + 1) % delays.len()
        } else {
            if current_idx == 0 {
                delays.len() - 1
            } else {
                current_idx - 1
            }
        };

        if let Some(m) = self.patch_mut().module_mut(tap_id) {
            m.kind = ModuleKind::DelayTap(delays[new_idx]);
        }
    }

    fn handle_save_prompt_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.message = Some("Save cancelled".into());
            }
            KeyCode::Enter => {
                let path = PathBuf::from(self.save_input.value());
                let is_current_file = self.file_path.as_ref() == Some(&path);
                if !is_current_file && path.exists() {
                    self.mode = Mode::SaveConfirm;
                } else {
                    self.save_to_file(path);
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Backspace => {
                self.save_input.handle(InputRequest::DeletePrevChar);
            }
            KeyCode::Delete => {
                self.save_input.handle(InputRequest::DeleteNextChar);
            }
            KeyCode::Left => {
                self.save_input.handle(InputRequest::GoToPrevChar);
            }
            KeyCode::Right => {
                self.save_input.handle(InputRequest::GoToNextChar);
            }
            KeyCode::Home => {
                self.save_input.handle(InputRequest::GoToStart);
            }
            KeyCode::End => {
                self.save_input.handle(InputRequest::GoToEnd);
            }
            KeyCode::Char(c) => {
                self.save_input.handle(InputRequest::InsertChar(c));
            }
            _ => {}
        }
    }

    fn handle_save_confirm_key(&mut self, code: KeyCode) {
        let Some(action) = lookup(bindings::quit_confirm_bindings(), code) else {
            return;
        };
        match action {
            Action::Confirm => {
                let path = PathBuf::from(self.save_input.value());
                self.save_to_file(path);
                self.mode = Mode::Normal;
            }
            Action::Cancel => {
                self.mode = Mode::SavePrompt;
            }
            _ => {}
        }
    }

    fn handle_export_prompt_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.message = Some("Export cancelled".into());
            }
            KeyCode::Enter => {
                let path = PathBuf::from(self.export_input.value());
                if path.exists() {
                    self.mode = Mode::ExportConfirm;
                } else {
                    self.export_to_wav();
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Backspace => {
                self.export_input.handle(InputRequest::DeletePrevChar);
            }
            KeyCode::Delete => {
                self.export_input.handle(InputRequest::DeleteNextChar);
            }
            KeyCode::Left => {
                self.export_input.handle(InputRequest::GoToPrevChar);
            }
            KeyCode::Right => {
                self.export_input.handle(InputRequest::GoToNextChar);
            }
            KeyCode::Home => {
                self.export_input.handle(InputRequest::GoToStart);
            }
            KeyCode::End => {
                self.export_input.handle(InputRequest::GoToEnd);
            }
            KeyCode::Up => {
                self.export_loops = self.export_loops.saturating_add(1).min(100);
            }
            KeyCode::Down => {
                self.export_loops = self.export_loops.saturating_sub(1).max(1);
            }
            KeyCode::Char(c) => {
                self.export_input.handle(InputRequest::InsertChar(c));
            }
            _ => {}
        }
    }

    fn handle_export_confirm_key(&mut self, code: KeyCode) {
        let Some(action) = lookup(bindings::quit_confirm_bindings(), code) else {
            return;
        };
        match action {
            Action::Confirm => {
                self.export_to_wav();
                self.mode = Mode::Normal;
            }
            Action::Cancel => {
                self.mode = Mode::ExportPrompt;
            }
            _ => {}
        }
    }

    fn export_to_wav(&mut self) {
        use crate::Signal;

        let sample_rate = 44100usize;
        let scale = scale_from_idx(self.scale_idx());
        let track = Track::parse(self.track_text(), &scale).ok();
        let bars = track.as_ref().map(|t| t.bar_count() as f32).unwrap_or(1.0);
        let bpm = self.bpm;

        let seconds_per_beat = 60.0 / bpm;
        let beats_per_bar = 4.0;
        let duration = seconds_per_beat * beats_per_bar * bars * self.export_loops as f32;
        let total_samples = (duration * sample_rate as f32) as usize;

        let mut compiled = CompiledPatch::default();
        let ctx = CompileContext {
            sample_rate: sample_rate as f32,
            bpm,
            bars,
        };
        compile_patch(&mut compiled, &self.patches(), NUM_VOICES, &ctx);

        let mut track_state = TrackState::new(NUM_VOICES);
        track_state.clock.bpm(bpm).bars(bars);
        track_state.set_track(track);

        let mut signal = Signal::new(sample_rate);
        let mut samples = Vec::with_capacity(total_samples);

        for _ in 0..total_samples {
            track_state.update(&mut signal);
            let sample = compiled
                .process(&mut signal, &track_state, 0)
                .clamp(-1.0, 1.0);
            samples.push(sample);
            signal.advance();
        }

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        match hound::WavWriter::create(self.export_input.value(), spec) {
            Ok(mut writer) => {
                for sample in samples {
                    let _ = writer.write_sample(sample);
                    let _ = writer.write_sample(sample);
                }
                if writer.finalize().is_ok() {
                    self.message = Some(format!(
                        "Exported {} ({:.1}s, {} loops)",
                        self.export_input.value(),
                        duration,
                        self.export_loops
                    ));
                } else {
                    self.message = Some("Export failed: finalize error".into());
                }
            }
            Err(e) => {
                self.message = Some(format!("Export failed: {}", e));
            }
        }
    }

    fn save_to_file(&mut self, path: PathBuf) {
        let track_str = self.track_text();
        let track = if track_str.trim().is_empty() {
            None
        } else {
            Some(track_str)
        };
        let scale = scale_from_idx(self.scale_idx());
        let bars = Track::parse(track_str, &scale)
            .ok()
            .map(|t| t.bar_count() as f32)
            .unwrap_or(1.0);

        match persist::save_patchset(
            &path,
            &self.patches(),
            self.bpm,
            bars,
            self.scale_idx(),
            track,
        ) {
            Ok(()) => {
                self.file_path = Some(path.clone());
                self.dirty = false;
                self.message = Some(format!("Saved to {}", path.display()));
            }
            Err(e) => {
                self.message = Some(format!("Save failed: {}", e));
            }
        }
    }

    fn load_from_file(&mut self, path: PathBuf) {
        match persist::load_patchset(&path) {
            Ok(result) => {
                {
                    let inst = self.inst_mut();
                    inst.patches = result.patches;
                    inst.scale_idx = result.scale_idx;
                    if let Some(track_text) = result.track {
                        inst.track_text = track_text;
                    }
                }

                self.file_path = Some(path.clone());
                self.bpm = result.bpm;
                let _ = self.cmd_tx.send(AudioCommand::SetBpm(self.bpm));

                self.commit_patch();

                if result.missing_samples.is_empty() {
                    self.message = Some(format!("Loaded {}", path.display()));
                } else {
                    let missing: Vec<_> = result.missing_samples.iter().take(3).cloned().collect();
                    let more = if result.missing_samples.len() > 3 {
                        format!(" (+{} more)", result.missing_samples.len() - 3)
                    } else {
                        String::new()
                    };
                    self.message = Some(format!(
                        "Loaded {} - missing samples: {}{}",
                        path.display(),
                        missing.join(", "),
                        more
                    ));
                }
            }
            Err(e) => {
                self.message = Some(format!("Load failed: {}", e));
            }
        }
    }

    fn handle_adsr_edit_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        let Some(_) = self.patch().module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let Some(action) = lookup(bindings::edit_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::Down => {
                let new_idx = (param_idx + 1) % 2;
                self.mode = Mode::AdsrEdit {
                    module_id,
                    param_idx: new_idx,
                };
            }
            Action::Up => {
                let new_idx = if param_idx == 0 { 1 } else { param_idx - 1 };
                self.mode = Mode::AdsrEdit {
                    module_id,
                    param_idx: new_idx,
                };
            }
            Action::ValueUp => {
                if let Some(m) = self.patch_mut().module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr {
                        attack_ratio,
                        sustain,
                        ..
                    } = &mut m.params
                    {
                        match param_idx {
                            0 => *attack_ratio = (*attack_ratio + 0.05).min(1.0),
                            1 => *sustain = (*sustain + 0.05).min(1.0),
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            Action::ValueDown => {
                if let Some(m) = self.patch_mut().module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr {
                        attack_ratio,
                        sustain,
                        ..
                    } = &mut m.params
                    {
                        match param_idx {
                            0 => *attack_ratio = (*attack_ratio - 0.05).max(0.0),
                            1 => *sustain = (*sustain - 0.05).max(0.0),
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            Action::ValueUpFast => {
                if let Some(m) = self.patch_mut().module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr {
                        attack_ratio,
                        sustain,
                        ..
                    } = &mut m.params
                    {
                        match param_idx {
                            0 => *attack_ratio = 1.0,
                            1 => *sustain = 1.0,
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            Action::ValueDownFast => {
                if let Some(m) = self.patch_mut().module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr {
                        attack_ratio,
                        sustain,
                        ..
                    } = &mut m.params
                    {
                        match param_idx {
                            0 => *attack_ratio = 0.0,
                            1 => *sustain = 0.0,
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            _ => {}
        }
    }

    fn handle_env_edit_key(
        &mut self,
        code: KeyCode,
        module_id: ModuleId,
        point_idx: usize,
        editing: bool,
    ) {
        let Some(module) = self.patch().module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let num_points = module.params.env_points().map(|p| p.len()).unwrap_or(0);
        if num_points == 0 {
            self.mode = Mode::Normal;
            return;
        }

        if editing {
            let Some(action) = lookup(bindings::env_move_bindings(), code) else {
                return;
            };
            let step = self.step_value();
            match action {
                Action::Cancel | Action::Confirm => {
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx,
                        editing: false,
                    };
                }
                Action::Right => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, step);
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx: new_idx,
                        editing: true,
                    };
                    self.commit_patch();
                }
                Action::Left => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, -step);
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx: new_idx,
                        editing: true,
                    };
                    self.commit_patch();
                }
                Action::RightFast => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, step * 10.0);
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx: new_idx,
                        editing: true,
                    };
                    self.commit_patch();
                }
                Action::LeftFast => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, -step * 10.0);
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx: new_idx,
                        editing: true,
                    };
                    self.commit_patch();
                }
                Action::Up => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = (p.value + step).min(1.0);
                            }
                        }
                    }
                    self.commit_patch();
                }
                Action::Down => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = (p.value - step).max(-1.0);
                            }
                        }
                    }
                    self.commit_patch();
                }
                Action::UpFast => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = 1.0;
                            }
                        }
                    }
                    self.commit_patch();
                }
                Action::DownFast => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = -1.0;
                            }
                        }
                    }
                    self.commit_patch();
                }
                Action::CycleStep => {
                    self.step_size = (self.step_size + 1) % 5;
                    self.message = Some(format!("Step: {}", self.step_label()));
                }
                _ => {}
            }
        } else {
            let Some(action) = lookup(bindings::env_bindings(), code) else {
                return;
            };
            match action {
                Action::Cancel => {
                    self.mode = Mode::Normal;
                }
                Action::Right => {
                    let new_idx = (point_idx + 1) % num_points;
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx: new_idx,
                        editing: false,
                    };
                }
                Action::Left => {
                    let new_idx = if point_idx == 0 {
                        num_points - 1
                    } else {
                        point_idx - 1
                    };
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx: new_idx,
                        editing: false,
                    };
                }
                Action::Move => {
                    self.mode = Mode::EnvEdit {
                        module_id,
                        point_idx,
                        editing: true,
                    };
                }
                Action::ToggleCurve => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.curve = !p.curve;
                            }
                        }
                    }
                    self.commit_patch();
                }
                Action::AddPoint => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            let new_time = if points.is_empty() {
                                0.5
                            } else if point_idx + 1 < points.len() {
                                (points[point_idx].time + points[point_idx + 1].time) / 2.0
                            } else {
                                (points[point_idx].time + 1.0) / 2.0
                            };
                            let new_value = points.get(point_idx).map(|p| p.value).unwrap_or(0.5);
                            points.push(super::module::EnvPoint {
                                time: new_time,
                                value: new_value,
                                curve: false,
                            });
                            points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
                            let new_idx = points
                                .iter()
                                .position(|p| (p.time - new_time).abs() < 0.001)
                                .unwrap_or(point_idx);
                            self.mode = Mode::EnvEdit {
                                module_id,
                                point_idx: new_idx,
                                editing: false,
                            };
                        }
                    }
                    self.commit_patch();
                }
                Action::DeletePoint => {
                    if let Some(m) = self.patch_mut().module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if points.len() > 2 {
                                points.remove(point_idx);
                                let new_idx = point_idx.min(points.len() - 1);
                                self.mode = Mode::EnvEdit {
                                    module_id,
                                    point_idx: new_idx,
                                    editing: false,
                                };
                            } else {
                                self.message = Some("Need at least 2 points".into());
                            }
                        }
                    }
                    self.commit_patch();
                }
                Action::CycleStep => {
                    self.step_size = (self.step_size + 1) % 5;
                    self.message = Some(format!("Step: {}", self.step_label()));
                }
                _ => {}
            }
        }
    }

    fn adjust_env_point_time(
        &mut self,
        module_id: ModuleId,
        point_idx: usize,
        delta: f32,
    ) -> usize {
        let Some(m) = self.patch_mut().module_mut(module_id) else {
            return point_idx;
        };
        let Some(points) = m.params.env_points_mut() else {
            return point_idx;
        };
        let Some(p) = points.get_mut(point_idx) else {
            return point_idx;
        };

        let new_time = (p.time + delta).clamp(0.0, 1.0);
        p.time = new_time;

        points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        points
            .iter()
            .position(|p| (p.time - new_time).abs() < 1e-6)
            .unwrap_or(point_idx)
    }

    fn handle_probe_edit_key(&mut self, code: KeyCode, module_id: ModuleId) {
        let Some(_) = self.patch().module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let Some(action) = lookup(bindings::probe_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::ValueUp | Action::Right => {
                self.probe_len = (self.probe_len * 2).min(44100 * 2);
            }
            Action::ValueDown | Action::Left => {
                self.probe_len = (self.probe_len / 2).max(8);
            }
            Action::ValueUpFast => {
                self.probe_len = (self.probe_len * 4).min(44100 * 2);
            }
            Action::ValueDownFast => {
                self.probe_len = (self.probe_len / 4).max(8);
            }
            Action::Delete => {
                self.probe_len = 4410;
            }
            _ => {}
        }
    }

    fn handle_sample_view_key(
        &mut self,
        code: KeyCode,
        module_id: ModuleId,
        zoom: f32,
        offset: f32,
    ) {
        match code {
            KeyCode::Esc | KeyCode::Char('i') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let new_zoom = (zoom * 2.0).min(64.0);
                self.mode = Mode::SampleView {
                    module_id,
                    zoom: new_zoom,
                    offset: offset.min(1.0 - 1.0 / new_zoom),
                };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let new_zoom = (zoom / 2.0).max(1.0);
                self.mode = Mode::SampleView {
                    module_id,
                    zoom: new_zoom,
                    offset: offset.min(1.0 - 1.0 / new_zoom).max(0.0),
                };
            }
            KeyCode::Char('h') | KeyCode::Left => {
                let step = 0.1 / zoom;
                let new_offset = (offset - step).max(0.0);
                self.mode = Mode::SampleView {
                    module_id,
                    zoom,
                    offset: new_offset,
                };
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let step = 0.1 / zoom;
                let max_offset = (1.0 - 1.0 / zoom).max(0.0);
                let new_offset = (offset + step).min(max_offset);
                self.mode = Mode::SampleView {
                    module_id,
                    zoom,
                    offset: new_offset,
                };
            }
            KeyCode::Char('r') => {
                self.mode = Mode::SampleView {
                    module_id,
                    zoom: 1.0,
                    offset: 0.0,
                };
            }
            _ => {}
        }
    }

    fn handle_track_settings_key(&mut self, code: KeyCode, param_idx: usize) {
        let Some(action) = lookup(bindings::settings_bindings(), code) else {
            return;
        };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::Down => {
                let new_idx = (param_idx + 1) % 3;
                self.mode = Mode::TrackSettings { param_idx: new_idx };
            }
            Action::Up => {
                let new_idx = if param_idx == 0 { 2 } else { param_idx - 1 };
                self.mode = Mode::TrackSettings { param_idx: new_idx };
            }
            Action::ValueUp => match param_idx {
                0 => {
                    self.bpm = (self.bpm + 5.0).min(300.0);
                    let _ = self.cmd_tx.send(AudioCommand::SetBpm(self.bpm));
                }
                1 => {
                    let new_idx = (self.scale_idx() + 1) % SCALE_NAMES.len();
                    self.inst_mut().scale_idx = new_idx;
                    self.reparse_track();
                }
                2 => {
                    let _ = self.cmd_tx.send(AudioCommand::SetProbeVoice(
                        (self.probe_voice + 1) % NUM_VOICES,
                    ));
                    self.probe_voice = (self.probe_voice + 1) % NUM_VOICES;
                }
                _ => {}
            },
            Action::ValueDown => match param_idx {
                0 => {
                    self.bpm = (self.bpm - 5.0).max(20.0);
                    let _ = self.cmd_tx.send(AudioCommand::SetBpm(self.bpm));
                }
                1 => {
                    let cur = self.scale_idx();
                    let new_idx = if cur == 0 {
                        SCALE_NAMES.len() - 1
                    } else {
                        cur - 1
                    };
                    self.inst_mut().scale_idx = new_idx;
                    self.reparse_track();
                }
                2 => {
                    let v = if self.probe_voice == 0 {
                        NUM_VOICES - 1
                    } else {
                        self.probe_voice - 1
                    };
                    let _ = self.cmd_tx.send(AudioCommand::SetProbeVoice(v));
                    self.probe_voice = v;
                }
                _ => {}
            },
            Action::ValueUpFast => {
                if param_idx == 0 {
                    self.bpm = (self.bpm + 20.0).min(300.0);
                    let _ = self.cmd_tx.send(AudioCommand::SetBpm(self.bpm));
                }
            }
            Action::ValueDownFast => {
                if param_idx == 0 {
                    self.bpm = (self.bpm - 20.0).max(20.0);
                    let _ = self.cmd_tx.send(AudioCommand::SetBpm(self.bpm));
                }
            }
            _ => {}
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area());

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(22),
            ])
            .split(chunks[0]);

        let notes_area = main_chunks[0];
        let grid_area = main_chunks[1];
        let help_area = main_chunks[2];
        let status_area = chunks[1];

        let (moving_id, copy_previews, move_previews): (
            Option<ModuleId>,
            Vec<(Module, GridPos)>,
            Vec<(Module, GridPos)>,
        ) = match &self.mode {
            Mode::Move { module_id, .. } => {
                let preview = self
                    .patch()
                    .module(*module_id)
                    .cloned()
                    .map(|m| vec![(m, self.cursor())])
                    .unwrap_or_default();
                (Some(*module_id), vec![], preview)
            }
            Mode::Copy { module } => (None, vec![(module.clone(), self.cursor())], vec![]),
            Mode::CopySelection { modules, origin } => {
                let dx = self.cursor().x as i16 - origin.x as i16;
                let dy = self.cursor().y as i16 - origin.y as i16;
                let previews = modules
                    .iter()
                    .map(|(m, pos)| {
                        let new_x = (pos.x as i16 + dx).max(0) as u16;
                        let new_y = (pos.y as i16 + dy).max(0) as u16;
                        (m.clone(), GridPos::new(new_x, new_y))
                    })
                    .collect();
                (None, previews, vec![])
            }
            Mode::SelectMove {
                anchor,
                extent,
                move_origin,
            } => {
                let dx = self.cursor().x as i16 - move_origin.x as i16;
                let dy = self.cursor().y as i16 - move_origin.y as i16;
                let ids = self.modules_in_rect(*anchor, *extent);
                let previews: Vec<(Module, GridPos)> = ids
                    .iter()
                    .filter_map(|id| {
                        let m = self.patch().module(*id)?.clone();
                        let pos = self.patch().module_position(*id)?;
                        let new_x = (pos.x as i16 + dx).max(0) as u16;
                        let new_y = (pos.y as i16 + dy).max(0) as u16;
                        Some((m, GridPos::new(new_x, new_y)))
                    })
                    .collect();
                (None, vec![], previews)
            }
            _ => (None, vec![], vec![]),
        };

        let selection = match self.mode {
            Mode::Select { anchor } | Mode::MouseSelect { anchor } => Some((anchor, self.cursor())),
            Mode::SelectMove { anchor, extent, .. } => Some((anchor, extent)),
            _ => None,
        };

        self.grid_area = grid_area;

        let display_patch = self.patch();
        let subpatch_border = self
            .editing_subpatch()
            .and_then(|id| self.patches().subpatch(id).map(|s| s.color));

        if let Some(border_color) = subpatch_border {
            let border = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));
            let inner = border.inner(grid_area);
            f.render_widget(border, grid_area);

            let grid_widget = GridWidget::new(display_patch)
                .cursor(self.cursor())
                .view_center(self.view_center())
                .moving(moving_id)
                .copy_previews(copy_previews)
                .move_previews(move_previews)
                .selection(selection)
                .probe_values(self.probe_values())
                .meter_values(self.meter_values())
                .show_meters(self.show_meters);
            f.render_widget(grid_widget, inner);
        } else {
            let grid_widget = GridWidget::new(display_patch)
                .cursor(self.cursor())
                .view_center(self.view_center())
                .moving(moving_id)
                .copy_previews(copy_previews)
                .move_previews(move_previews)
                .selection(selection)
                .probe_values(self.probe_values())
                .meter_values(self.meter_values())
                .show_meters(self.show_meters);
            f.render_widget(grid_widget, grid_area);
        }

        let help_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        f.render_widget(help_block, help_area);

        let buf = f.buffer_mut();
        let now = Instant::now();
        let scroll_t = self.brand_scroll.animate_wrapped(now);

        let brand_rows = render_brand("BRAINWASH", 1);

        let brand_height = brand_rows.len().min(8);
        let brand_total_width = brand_rows
            .iter()
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0);
        let viewport_width = (help_area.width.saturating_sub(2)) as usize;
        let scroll_range = brand_total_width.saturating_sub(viewport_width);
        let scroll_offset = (scroll_t * scroll_range as f32).round() as usize;

        let brand_style = Style::default().fg(Color::Yellow);
        for (row_idx, row) in brand_rows.iter().take(brand_height).enumerate() {
            let y = help_area.y + row_idx as u16;
            if y >= help_area.y + help_area.height {
                break;
            }
            let chars: Vec<char> = row.chars().collect();
            for vp_x in 0..viewport_width {
                let char_idx = scroll_offset + vp_x;
                if char_idx < chars.len() {
                    let x = help_area.x + 1 + vp_x as u16;
                    if x < help_area.x + help_area.width {
                        buf[(x, y)].set_char(chars[char_idx]).set_style(brand_style);
                    }
                }
            }
        }

        let brand_divider_y = help_area.y + brand_height as u16;
        let divider_style = Style::default().fg(Color::Rgb(60, 60, 60));
        for x in (help_area.x + 1)..(help_area.x + help_area.width) {
            if brand_divider_y < help_area.y + help_area.height {
                buf[(x, brand_divider_y)]
                    .set_char('─')
                    .set_style(divider_style);
            }
        }

        let tabs_y = help_area.y + brand_height as u16 + 1;
        let tabs_x = help_area.x + 1;
        for i in 0..self.instruments.len().min(5) {
            let x = tabs_x + (i as u16 * 2);
            if x < help_area.x + help_area.width - 1 {
                let label = format!("{}", i + 1);
                let style = if i == self.current_instrument {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                buf[(x, tabs_y)]
                    .set_char(label.chars().next().unwrap())
                    .set_style(style);
            }
        }
        if self.instruments.len() < 5 {
            let plus_x = tabs_x + (self.instruments.len() as u16 * 2);
            if plus_x < help_area.x + help_area.width - 1 {
                buf[(plus_x, tabs_y)]
                    .set_char('+')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        let tabs_divider_y = help_area.y + brand_height as u16 + 2;
        for x in (help_area.x + 1)..(help_area.x + help_area.width) {
            if tabs_divider_y < help_area.y + help_area.height {
                buf[(x, tabs_divider_y)]
                    .set_char('─')
                    .set_style(divider_style);
            }
        }

        let header_height = brand_height as u16 + 3;

        let note_style = Style::default().fg(Color::Yellow);
        let note_active_style = Style::default().fg(Color::Black).bg(Color::Yellow);

        let scale = scale_from_idx(self.scale_idx());
        let mut scale_pitches: Vec<u8> = (-49..49i32)
            .map(|deg| scale.note(deg))
            .filter(|&p| p >= 12 && p <= 120)
            .map(|p| p as u8)
            .collect();
        scale_pitches.dedup();
        scale_pitches.reverse();

        let max_notes = notes_area.height as usize;
        let center_pitch = if self.active_pitches.is_empty() {
            60u8
        } else {
            let sum: u32 = self.active_pitches.iter().map(|&p| p as u32).sum();
            (sum / self.active_pitches.len() as u32) as u8
        };
        let center_idx = scale_pitches
            .iter()
            .position(|&p| p <= center_pitch)
            .unwrap_or(scale_pitches.len() / 2);
        let half = max_notes / 2;
        let start = center_idx.saturating_sub(half);
        let end = (start + max_notes).min(scale_pitches.len());
        let start = end.saturating_sub(max_notes);
        let visible_notes: Vec<u8> = scale_pitches[start..end].to_vec();

        for (i, &pitch) in visible_notes.iter().enumerate() {
            let y = notes_area.y + i as u16;
            let note_str = pitch_to_note(pitch);
            let is_active = self.active_pitches.contains(&pitch);
            let style = if is_active {
                note_active_style
            } else {
                note_style
            };
            for (j, c) in note_str.chars().enumerate() {
                let x = notes_area.x + j as u16;
                if x < notes_area.x + notes_area.width {
                    buf[(x, y)].set_char(c).set_style(style);
                }
            }
        }

        let help_inner = Rect::new(
            help_area.x + 1,
            help_area.y + header_height,
            help_area.width.saturating_sub(2),
            help_area.height.saturating_sub(header_height),
        );
        let bindings = match &self.mode {
            Mode::Normal => bindings::normal_bindings(),
            Mode::Palette if self.palette_searching => bindings::text_input_bindings(),
            Mode::Palette => bindings::palette_bindings(),
            Mode::Move { .. } | Mode::Copy { .. } | Mode::CopySelection { .. } => {
                bindings::move_bindings()
            }
            Mode::Select { .. } | Mode::MouseSelect { .. } | Mode::SelectMove { .. } => {
                bindings::select_bindings()
            }
            Mode::Edit { .. } | Mode::AdsrEdit { .. } => bindings::edit_bindings(),
            Mode::ProbeEdit { .. } | Mode::SampleView { .. } => bindings::probe_bindings(),
            Mode::EnvEdit { editing: true, .. } => bindings::env_move_bindings(),
            Mode::EnvEdit { .. } => bindings::env_bindings(),
            Mode::QuitConfirm | Mode::SaveConfirm | Mode::ExportConfirm => {
                bindings::quit_confirm_bindings()
            }
            Mode::SavePrompt | Mode::ExportPrompt | Mode::ValueInput { .. } => {
                bindings::text_input_bindings()
            }
            Mode::TrackSettings { .. } => bindings::settings_bindings(),
        };
        f.render_widget(HelpWidget::new(bindings, self.help_scroll), help_inner);

        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Palette => "PALETTE",
            Mode::Move { .. } => "MOVE",
            Mode::Copy { .. } | Mode::CopySelection { .. } => "COPY",
            Mode::Select { .. } | Mode::MouseSelect { .. } => "SELECT",
            Mode::SelectMove { .. } => "SEL-MOVE",
            Mode::Edit { .. } => "EDIT",
            Mode::ValueInput { .. } => "INPUT",
            Mode::AdsrEdit { .. } => "ADSR",
            Mode::EnvEdit { .. } => "ENV",
            Mode::ProbeEdit { .. } => "PROBE",
            Mode::SampleView { .. } => "SAMPLE",
            Mode::SavePrompt => "SAVE",
            Mode::SaveConfirm => "OVERWRITE?",
            Mode::QuitConfirm => "QUIT?",
            Mode::TrackSettings { .. } => "TRACK",
            Mode::ExportPrompt => "EXPORT",
            Mode::ExportConfirm => "OVERWRITE?",
        };
        let history: Vec<f32> = self.output_history.iter().copied().collect();
        let mut status = StatusWidget::new(self.cursor(), mode_str)
            .playing(self.playing)
            .output_history(&history);
        if let Some(ref msg) = self.message {
            status = status.message(msg);
        }
        f.render_widget(status, status_area);

        if self.mode == Mode::Palette {
            let palette_width = 32;
            let palette_height = 22;
            let palette_x = (f.area().width.saturating_sub(palette_width)) / 2;
            let palette_y = (f.area().height.saturating_sub(palette_height)) / 2;
            let palette_area = Rect::new(palette_x, palette_y, palette_width, palette_height);

            f.render_widget(Clear, palette_area);

            let palette_block = Block::default()
                .title(" Modules ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White));
            f.render_widget(palette_block, palette_area);

            let inner = Rect::new(
                palette_area.x + 1,
                palette_area.y + 1,
                palette_area.width.saturating_sub(2),
                palette_area.height.saturating_sub(2),
            );
            let palette = PaletteWidget::new()
                .selected_category(self.palette_category)
                .selected_module(self.palette_selections[self.palette_category])
                .filter(
                    &self.palette_filter,
                    self.filtered_modules(),
                    self.palette_filter_selection,
                    self.palette_searching,
                );
            f.render_widget(palette, inner);
        }

        if let Mode::Edit {
            module_id,
            param_idx,
        } = self.mode
        {
            if let Some(module) = self.patch().module(module_id) {
                let edit_width = 36;
                let extra = if module.kind.has_special_editor() {
                    2
                } else {
                    0
                };
                let edit_height = (module.kind.param_defs().len() + 8 + extra) as u16;
                let edit_x = (f.area().width.saturating_sub(edit_width)) / 2;
                let edit_y = (f.area().height.saturating_sub(edit_height)) / 2;
                let edit_area = Rect::new(edit_x, edit_y, edit_width, edit_height);

                f.render_widget(Clear, edit_area);

                let edit_block = Block::default()
                    .title(" Edit ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(module.kind.color()));
                f.render_widget(edit_block, edit_area);

                let inner = Rect::new(
                    edit_area.x + 1,
                    edit_area.y + 1,
                    edit_area.width.saturating_sub(2),
                    edit_area.height.saturating_sub(2),
                );
                let edit_widget =
                    EditWidget::new(module, param_idx, self.patch()).step_label(self.step_label());
                f.render_widget(edit_widget, inner);
            }
        }

        if let Mode::ValueInput {
            module_id,
            param_idx,
        } = self.mode
        {
            if let Some(module) = self.patch().module(module_id) {
                let defs = module.kind.param_defs();
                let param_name = defs.get(param_idx).map(|d| d.name).unwrap_or("Value");

                let input_width = 24u16;
                let input_height = 3u16;
                let input_x = (f.area().width.saturating_sub(input_width)) / 2;
                let input_y = (f.area().height.saturating_sub(input_height)) / 2;
                let input_area = Rect::new(input_x, input_y, input_width, input_height);

                f.render_widget(Clear, input_area);

                let input_block = Block::default()
                    .title(format!(" {} ", param_name))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                f.render_widget(input_block, input_area);

                let inner = Rect::new(
                    input_area.x + 1,
                    input_area.y + 1,
                    input_area.width.saturating_sub(2),
                    1,
                );
                let input_text = ratatui::widgets::Paragraph::new(self.value_input.value());
                f.render_widget(input_text, inner);
                f.set_cursor_position((inner.x + self.value_input.visual_cursor() as u16, inner.y));
            }
        }

        if let Mode::AdsrEdit {
            module_id,
            param_idx,
        } = self.mode
        {
            if let Some(module) = self.patch().module(module_id) {
                let env_width = grid_area.width.saturating_sub(4);
                let env_height = grid_area.height.saturating_sub(4);
                let env_x = grid_area.x + 2;
                let env_y = grid_area.y + 2;
                let env_area = Rect::new(env_x, env_y, env_width, env_height);

                f.render_widget(Clear, env_area);

                let env_block = Block::default()
                    .title(" ADSR ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(module.kind.color()));
                f.render_widget(env_block, env_area);

                let inner = Rect::new(
                    env_area.x + 1,
                    env_area.y + 1,
                    env_area.width.saturating_sub(2),
                    env_area.height.saturating_sub(2),
                );
                let adsr_widget = AdsrWidget::new(module, param_idx);
                f.render_widget(adsr_widget, inner);
            }
        }

        if let Mode::EnvEdit {
            module_id,
            point_idx,
            editing,
        } = self.mode
        {
            if let Some(module) = self.patch().module(module_id) {
                let env_width = grid_area.width.saturating_sub(4);
                let env_height = grid_area.height.saturating_sub(4);
                let env_x = grid_area.x + 2;
                let env_y = grid_area.y + 2;
                let env_area = Rect::new(env_x, env_y, env_width, env_height);

                f.render_widget(Clear, env_area);

                let title = if editing {
                    " Envelope [MOVE] "
                } else {
                    " Envelope "
                };
                let env_block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(module.kind.color()));
                f.render_widget(env_block, env_area);

                let inner = Rect::new(
                    env_area.x + 1,
                    env_area.y + 1,
                    env_area.width.saturating_sub(2),
                    env_area.height.saturating_sub(2),
                );
                let env_widget = EnvelopeWidget::new(module, point_idx, editing);
                f.render_widget(env_widget, inner);
            }
        }

        if let Mode::ProbeEdit { module_id, .. } = self.mode {
            if let Some(module) = self.patch().module(module_id) {
                let history: Vec<f32> = self
                    .inst()
                    .probe_histories
                    .get(&module_id)
                    .map(|h| h.iter().copied().collect())
                    .unwrap_or_default();
                let current = history.last().copied().unwrap_or(0.0);

                let (auto_min, auto_max) = if history.is_empty() {
                    (-1.0, 1.0)
                } else {
                    let min = history.iter().copied().fold(f32::INFINITY, f32::min);
                    let max = history.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                    let padding = (max - min).abs() * 0.1;
                    (min - padding, max + padding)
                };

                let probe_width = grid_area.width.saturating_sub(4);
                let probe_height = grid_area.height.saturating_sub(4);
                let probe_x = grid_area.x + 2;
                let probe_y = grid_area.y + 2;
                let probe_area = Rect::new(probe_x, probe_y, probe_width, probe_height);

                f.render_widget(Clear, probe_area);

                let probe_block = Block::default()
                    .title(" Probe ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(module.kind.color()));
                f.render_widget(probe_block, probe_area);

                let inner = Rect::new(
                    probe_area.x + 1,
                    probe_area.y + 1,
                    probe_area.width.saturating_sub(2),
                    probe_area.height.saturating_sub(2),
                );
                let probe_widget =
                    ProbeWidget::new(&history, auto_min, auto_max, self.probe_len, current);
                f.render_widget(probe_widget, inner);
            }
        }

        if let Mode::SampleView {
            module_id,
            zoom,
            offset,
        } = self.mode
        {
            if let Some(module) = self.patch().module(module_id) {
                if let ModuleParams::Sample { samples, .. } = &module.params {
                    let sample_width = grid_area.width.saturating_sub(4);
                    let sample_height = grid_area.height.saturating_sub(4);
                    let sample_x = grid_area.x + 2;
                    let sample_y = grid_area.y + 2;
                    let sample_area = Rect::new(sample_x, sample_y, sample_width, sample_height);

                    f.render_widget(Clear, sample_area);

                    let sample_block = Block::default()
                        .title(" Sample Waveform ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(module.kind.color()));
                    f.render_widget(sample_block, sample_area);

                    let inner = Rect::new(
                        sample_area.x + 1,
                        sample_area.y + 1,
                        sample_area.width.saturating_sub(2),
                        sample_area.height.saturating_sub(2),
                    );
                    let sample_widget = SampleWidget::new(samples, zoom, offset);
                    f.render_widget(sample_widget, inner);
                }
            }
        }

        if matches!(self.mode, Mode::SavePrompt | Mode::SaveConfirm) {
            let prompt_width = 50u16.min(f.area().width.saturating_sub(4));
            let prompt_height = 3u16;
            let prompt_x = (f.area().width.saturating_sub(prompt_width)) / 2;
            let prompt_y = (f.area().height.saturating_sub(prompt_height)) / 2;
            let prompt_area = Rect::new(prompt_x, prompt_y, prompt_width, prompt_height);

            f.render_widget(Clear, prompt_area);

            let title = if matches!(self.mode, Mode::SaveConfirm) {
                " Overwrite? (y/n) "
            } else {
                " Save As (Enter confirm, Esc cancel) "
            };
            let prompt_block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(prompt_block, prompt_area);

            let inner = Rect::new(
                prompt_area.x + 1,
                prompt_area.y + 1,
                prompt_area.width.saturating_sub(2),
                1,
            );
            let text = ratatui::widgets::Paragraph::new(self.save_input.value());
            f.render_widget(text, inner);
            f.set_cursor_position((inner.x + self.save_input.visual_cursor() as u16, inner.y));
        }

        if matches!(self.mode, Mode::ExportPrompt | Mode::ExportConfirm) {
            let prompt_width = 50u16.min(f.area().width.saturating_sub(4));
            let prompt_height = 5u16;
            let prompt_x = (f.area().width.saturating_sub(prompt_width)) / 2;
            let prompt_y = (f.area().height.saturating_sub(prompt_height)) / 2;
            let prompt_area = Rect::new(prompt_x, prompt_y, prompt_width, prompt_height);

            f.render_widget(Clear, prompt_area);

            let title = if matches!(self.mode, Mode::ExportConfirm) {
                " Overwrite? (y/n) "
            } else {
                " Export WAV (Enter confirm, Esc cancel) "
            };
            let prompt_block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            f.render_widget(prompt_block, prompt_area);

            let inner = Rect::new(
                prompt_area.x + 1,
                prompt_area.y + 1,
                prompt_area.width.saturating_sub(2),
                prompt_height.saturating_sub(2),
            );

            let loops_text = format!("Loops: {} (up/down)", self.export_loops);
            let file_line = ratatui::widgets::Paragraph::new(self.export_input.value());
            let loops_line = ratatui::widgets::Paragraph::new(loops_text)
                .style(Style::default().fg(Color::DarkGray));

            f.render_widget(file_line, Rect::new(inner.x, inner.y, inner.width, 1));
            f.render_widget(loops_line, Rect::new(inner.x, inner.y + 1, inner.width, 1));
            f.set_cursor_position((inner.x + self.export_input.visual_cursor() as u16, inner.y));
        }

        if let Mode::TrackSettings { param_idx } = self.mode {
            let width = 30u16;
            let height = 7u16;
            let x = (f.area().width.saturating_sub(width)) / 2;
            let y = (f.area().height.saturating_sub(height)) / 2;
            let area = Rect::new(x, y, width, height);

            f.render_widget(Clear, area);

            let block = Block::default()
                .title(" Settings ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(block, area);

            let label_style = Style::default().fg(Color::DarkGray);
            let value_style = Style::default().fg(Color::White);
            let selected_style = Style::default().fg(Color::Black).bg(Color::Yellow);

            let bpm_label = "BPM: ";
            let bpm_value = format!("{:.0}", self.bpm);
            let scale_label = "Scale: ";
            let scale_value = SCALE_NAMES[self.scale_idx()];
            let voice_label = "Probe Voice: ";
            let voice_value = format!("{}", self.probe_voice + 1);

            let bpm_style = if param_idx == 0 {
                selected_style
            } else {
                value_style
            };
            let scale_style = if param_idx == 1 {
                selected_style
            } else {
                value_style
            };
            let voice_style = if param_idx == 2 {
                selected_style
            } else {
                value_style
            };

            let inner_x = area.x + 2;
            let inner_y = area.y + 2;

            let buf = f.buffer_mut();
            for (i, c) in bpm_label.chars().enumerate() {
                let ix = i as u16;
                if inner_x + ix < area.x + area.width - 1 {
                    buf[(inner_x + ix, inner_y)]
                        .set_char(c)
                        .set_style(label_style);
                }
            }
            let bpm_x = inner_x + bpm_label.len() as u16;
            for (i, c) in bpm_value.chars().enumerate() {
                let ix = i as u16;
                if bpm_x + ix < area.x + area.width - 1 {
                    buf[(bpm_x + ix, inner_y)].set_char(c).set_style(bpm_style);
                }
            }

            let row2 = inner_y + 1;
            for (i, c) in scale_label.chars().enumerate() {
                let ix = i as u16;
                if inner_x + ix < area.x + area.width - 1 {
                    buf[(inner_x + ix, row2)].set_char(c).set_style(label_style);
                }
            }
            let scale_x = inner_x + scale_label.len() as u16;
            for (i, c) in scale_value.chars().enumerate() {
                let ix = i as u16;
                if scale_x + ix < area.x + area.width - 1 {
                    buf[(scale_x + ix, row2)].set_char(c).set_style(scale_style);
                }
            }

            let row3 = inner_y + 2;
            for (i, c) in voice_label.chars().enumerate() {
                let ix = i as u16;
                if inner_x + ix < area.x + area.width - 1 {
                    buf[(inner_x + ix, row3)].set_char(c).set_style(label_style);
                }
            }
            let voice_x = inner_x + voice_label.len() as u16;
            for (i, c) in voice_value.chars().enumerate() {
                let ix = i as u16;
                if voice_x + ix < area.x + area.width - 1 {
                    buf[(voice_x + ix, row3)].set_char(c).set_style(voice_style);
                }
            }
        }
    }
}

const NUM_VOICES: usize = 6;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let file_arg = std::env::args().nth(1).map(PathBuf::from);

    let (meter_tx, meter_rx) = meter_channel();
    let (output_tx, output_rx) = output_channel();
    let (cmd_tx, cmd_rx) = command_channel();

    let player = AudioPlayer::new()?;
    let sample_rate = player.config.sample_rate.0 as f32;
    let playing = Arc::new(Mutex::new(false));
    let playing_clone = Arc::clone(&playing);

    let stream = {
        use assert_no_alloc::assert_no_alloc;
        use cpal::traits::DeviceTrait;

        let mut engine = AudioEngine::new(NUM_VOICES, sample_rate, cmd_rx, meter_tx);
        let mut signal = Signal::new(sample_rate as usize);
        let channels = player.config.channels as usize;

        let mut output_counter = 0usize;
        player.device.build_output_stream(
            &player.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                engine.poll_commands();

                let is_playing = *playing_clone.lock().unwrap();
                if !is_playing {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    output_counter += data.len() / channels;
                    while output_counter >= OUTPUT_INTERVAL {
                        output_counter -= OUTPUT_INTERVAL;
                        let _ = output_tx.try_send(0.0);
                    }
                    return;
                }

                for frame in data.chunks_mut(channels) {
                    let sample = assert_no_alloc(|| engine.process(&mut signal).clamp(-1., 1.));

                    for channel_sample in frame.iter_mut() {
                        *channel_sample = sample;
                    }

                    output_counter += 1;
                    if output_counter >= OUTPUT_INTERVAL {
                        output_counter = 0;
                        let _ = output_tx.try_send(sample);
                    }

                    signal.advance();
                }
            },
            |err| eprintln!("Audio error: {}", err),
            None,
        )?
    };

    stream.play()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cmd_tx, meter_rx, output_rx);

    if let Some(path) = file_arg {
        app.load_from_file(path);
    } else {
        app.recompile_patch();
    }

    loop {
        *playing.lock().unwrap() = app.playing;

        terminal.draw(|f| app.ui(f))?;

        app.drain_meters();

        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            match &event {
                Event::Key(key) => {
                    app.handle_key(key.code, key.modifiers);
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse.kind, mouse.column, mouse.row);
                }
                _ => {}
            }
        }

        if let Some(request) = app.pending_request.take() {
            match request {
                AppRequest::EditTrack => {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                    let temp_path = std::env::temp_dir().join("brainwash_track.txt");
                    fs::write(&temp_path, app.track_text())?;

                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
                    let status = Command::new(&editor).arg(&temp_path).status();

                    if let Ok(s) = status {
                        if s.success() {
                            if let Ok(new_text) = fs::read_to_string(&temp_path) {
                                app.inst_mut().track_text = new_text;
                                app.reparse_track();
                            }
                        }
                    }
                    let _ = fs::remove_file(&temp_path);

                    enable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        EnterAlternateScreen,
                        EnableMouseCapture
                    )?;
                    terminal.clear()?;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
