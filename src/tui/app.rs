use super::grid::GridPos;
use super::module::{ModuleCategory, ModuleId, ModuleKind, ParamKind};
use super::patch::Patch;
use super::engine::{CompiledPatch, compile_patch, TrackState};
use super::render::{AdsrWidget, EditWidget, EnvelopeWidget, GridWidget, HelpWidget, PaletteWidget, ProbeWidget, StatusWidget};
use super::persist;
use super::bindings::{self, Action, lookup};
use crate::live::AudioPlayer;
use crate::scale::cmin;
use crate::track::Track;
use crate::Signal;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use cpal::traits::StreamTrait;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
};
use tui_textarea::TextArea;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::{io, time::Duration};
use std::process::Command;
use std::fs;

#[derive(Clone, PartialEq)]
enum Mode {
    Normal,
    Palette,
    Move { module_id: ModuleId, origin: GridPos },
    Select { anchor: GridPos },
    SelectMove { anchor: GridPos, extent: GridPos, move_origin: GridPos },
    QuitConfirm,
    Edit { module_id: ModuleId, param_idx: usize },
    AdsrEdit { module_id: ModuleId, param_idx: usize },
    EnvEdit { module_id: ModuleId, point_idx: usize, editing: bool },
    ProbeEdit { module_id: ModuleId, param_idx: usize },
    SavePrompt,
}

enum AppRequest {
    EditTrack,
}

struct App<'a> {
    patch: Patch,
    cursor: GridPos,
    mode: Mode,
    pending_request: Option<AppRequest>,
    palette_category: usize,
    palette_selections: [usize; 7],
    palette_filter: String,
    palette_filter_selection: usize,
    palette_searching: bool,
    message: Option<String>,
    should_quit: bool,
    dirty: bool,
    audio_patch: Arc<Mutex<CompiledPatch>>,
    track_state: Arc<Mutex<TrackState>>,
    playing: bool,
    track_textarea: TextArea<'a>,
    file_path: Option<PathBuf>,
    file_textarea: TextArea<'a>,
    probe_min: f32,
    probe_max: f32,
    probe_len: usize,
}

impl<'a> App<'a> {
    fn new(audio_patch: Arc<Mutex<CompiledPatch>>, track_state: Arc<Mutex<TrackState>>) -> Self {
        let mut patch = Patch::new(20, 20);
        patch.add_module(ModuleKind::Output, GridPos::new(19, 19));

        let mut textarea = TextArea::new(vec!["(0/2/4/7)".into()]);
        textarea.set_cursor_line_style(Style::default());
        textarea.set_block(Block::default());

        let notation = "(0/2/4/7)";
        let scale = cmin();
        if let Ok(track) = Track::parse(notation, &scale) {
            track_state.lock().unwrap().set_track(Some(track));
        }

        let mut file_textarea = TextArea::new(vec!["patch.bw".into()]);
        file_textarea.set_cursor_line_style(Style::default());
        file_textarea.set_block(Block::default());

        Self {
            patch,
            cursor: GridPos::new(0, 0),
            mode: Mode::Normal,
            pending_request: None,
            palette_category: 0,
            palette_selections: [0; 7],
            palette_filter: String::new(),
            palette_filter_selection: 0,
            palette_searching: false,
            message: None,
            should_quit: false,
            dirty: false,
            audio_patch,
            track_state,
            playing: false,
            track_textarea: textarea,
            file_path: None,
            file_textarea,
            probe_min: -1.0,
            probe_max: 1.0,
            probe_len: 4410,
        }
    }

    fn reparse_track(&mut self) {
        let scale = cmin();
        let notation: String = self.track_textarea.lines().join("");
        match Track::parse(&notation, &scale) {
            Ok(track) => {
                let mut state = self.track_state.lock().unwrap();
                state.set_track(Some(track));
                self.message = Some("Track updated".into());
            }
            Err(e) => {
                self.message = Some(format!("Parse error: {}", e));
            }
        }
    }

    fn commit_patch(&mut self) {
        let num_voices = self.track_state.lock().unwrap().num_voices();
        let mut audio = self.audio_patch.lock().unwrap();
        compile_patch(&mut audio, &self.patch, num_voices);
        self.dirty = true;
    }

    fn move_cursor(&mut self, dx: i16, dy: i16) {
        let grid = self.patch.grid();
        let new_x = (self.cursor.x as i16 + dx).clamp(0, grid.width() as i16 - 1) as u16;
        let new_y = (self.cursor.y as i16 + dy).clamp(0, grid.height() as i16 - 1) as u16;
        self.cursor = GridPos::new(new_x, new_y);
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.message = None;

        match self.mode.clone() {
            Mode::Normal => self.handle_normal_key(code),
            Mode::Palette => self.handle_palette_key(code),
            Mode::Move { module_id, origin } => self.handle_move_key(code, module_id, origin),
            Mode::Select { anchor } => self.handle_select_key(code, anchor),
            Mode::SelectMove { anchor, extent, move_origin } => self.handle_select_move_key(code, anchor, extent, move_origin),
            Mode::Edit { module_id, param_idx } => self.handle_edit_key(code, module_id, param_idx),
            Mode::AdsrEdit { module_id, param_idx } => self.handle_adsr_edit_key(code, module_id, param_idx),
            Mode::EnvEdit { module_id, point_idx, editing } => self.handle_env_edit_key(code, module_id, point_idx, editing),
            Mode::ProbeEdit { module_id, param_idx } => self.handle_probe_edit_key(code, module_id, param_idx),
            Mode::SavePrompt => self.handle_save_prompt_key(code, modifiers),
            Mode::QuitConfirm => self.handle_quit_confirm_key(code),
        }
    }

    fn handle_quit_confirm_key(&mut self, code: KeyCode) {
        let Some(action) = lookup(bindings::quit_confirm_bindings(), code) else { return };
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
        let Some(action) = lookup(bindings::normal_bindings(), code) else { return };
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
            Action::Home => self.cursor = GridPos::new(0, 0),
            Action::End => {
                if let Some(out_id) = self.patch.output_id() {
                    if let Some(pos) = self.patch.module_position(out_id) {
                        self.cursor = pos;
                    }
                }
            }
            Action::Place => {
                self.mode = Mode::Palette;
            }
            Action::Move => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    self.mode = Mode::Move { module_id: id, origin: self.cursor };
                }
            }
            Action::Delete => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    if let Some(m) = self.patch.module(id) {
                        if m.kind == ModuleKind::Output {
                            self.message = Some("Cannot delete output".into());
                        } else if self.patch.remove_module(id) {
                            self.message = Some("Deleted".into());
                            self.commit_patch();
                        }
                    }
                }
            }
            Action::Rotate => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    if let Some(m) = self.patch.module(id) {
                        if m.kind.is_routing() {
                            self.message = Some("Cannot rotate routing modules".into());
                        } else if self.patch.rotate_module(id) {
                            self.message = Some("Rotated".into());
                            self.commit_patch();
                        } else {
                            self.message = Some("No room to rotate".into());
                        }
                    }
                }
            }
            Action::Edit => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    if let Some(m) = self.patch.module(id) {
                        if m.kind == ModuleKind::Adsr {
                            self.mode = Mode::AdsrEdit { module_id: id, param_idx: 0 };
                        } else if m.kind == ModuleKind::Envelope {
                            self.mode = Mode::EnvEdit { module_id: id, point_idx: 0, editing: false };
                        } else if m.kind == ModuleKind::Probe {
                            self.mode = Mode::ProbeEdit { module_id: id, param_idx: 0 };
                        } else {
                            let defs = m.kind.param_defs();
                            if !defs.is_empty() {
                                self.mode = Mode::Edit { module_id: id, param_idx: 0 };
                            } else {
                                self.message = Some("No params to edit".into());
                            }
                        }
                    }
                }
            }
            Action::Inspect => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    if let Some(m) = self.patch.module(id) {
                        self.open_palette_for_module(m.kind);
                    }
                }
            }
            Action::Palette(cat) => self.open_palette_category(cat),
            Action::TogglePlay => {
                self.playing = !self.playing;
                self.message = Some(if self.playing { "Playing".into() } else { "Paused".into() });
            }
            Action::TrackEdit => {
                self.pending_request = Some(AppRequest::EditTrack);
            }
            Action::Select => {
                self.mode = Mode::Select { anchor: self.cursor };
            }
            Action::Save => {
                if let Some(ref path) = self.file_path {
                    self.save_to_file(path.clone());
                } else {
                    self.file_textarea = TextArea::new(vec!["patch.bw".into()]);
                    self.file_textarea.set_cursor_line_style(Style::default());
                    self.mode = Mode::SavePrompt;
                }
            }
            Action::SaveAs => {
                let default = self.file_path.as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "patch.bw".into());
                self.file_textarea = TextArea::new(vec![default]);
                self.file_textarea.set_cursor_line_style(Style::default());
                self.mode = Mode::SavePrompt;
            }
            _ => {}
        }
    }

    fn open_palette_category(&mut self, cat: usize) {
        self.palette_category = cat;
        self.mode = Mode::Palette;
    }

    fn open_palette_for_module(&mut self, kind: ModuleKind) {
        let cat = kind.category();
        let cats = ModuleCategory::all();
        if let Some(cat_idx) = cats.iter().position(|&c| c == cat) {
            let modules_in_cat = ModuleKind::by_category(cat);
            if let Some(sel_idx) = modules_in_cat.iter().position(|&k| k == kind) {
                self.palette_category = cat_idx;
                self.palette_selections[cat_idx] = sel_idx;
                self.mode = Mode::Palette;
            }
        }
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
                KeyCode::Enter | KeyCode::Char('/') => {
                    if let Some(kind) = filtered.get(self.palette_filter_selection) {
                        if *kind == ModuleKind::Output && self.patch.output_id().is_some() {
                            self.message = Some("Output exists".into());
                        } else if self.patch.add_module(*kind, self.cursor).is_some() {
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
                KeyCode::Char(' ') => {
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
            return;
        }

        let categories = ModuleCategory::all();
        let current_cat = categories[self.palette_category];
        let modules = ModuleKind::by_category(current_cat);
        let palette_module = self.palette_selections[self.palette_category];

        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('/') => {
                self.palette_filter.clear();
                self.palette_filter_selection = 0;
                self.palette_searching = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if palette_module + 1 < modules.len() {
                    self.palette_selections[self.palette_category] += 1;
                } else if self.palette_category + 1 < categories.len() {
                    self.palette_category += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if palette_module > 0 {
                    self.palette_selections[self.palette_category] -= 1;
                } else if self.palette_category > 0 {
                    self.palette_category -= 1;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.palette_category = (self.palette_category + 1) % categories.len();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.palette_category = if self.palette_category == 0 {
                    categories.len() - 1
                } else {
                    self.palette_category - 1
                };
            }
            KeyCode::Char(' ') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter | KeyCode::Char('i') => {
                if let Some(kind) = modules.get(palette_module) {
                    if *kind == ModuleKind::Output && self.patch.output_id().is_some() {
                        self.message = Some("Output exists".into());
                    } else if self.patch.add_module(*kind, self.cursor).is_some() {
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

    fn handle_move_key(&mut self, code: KeyCode, module_id: ModuleId, origin: GridPos) {
        let Some(action) = lookup(bindings::move_bindings(), code) else { return };
        match action {
            Action::Cancel => {
                self.cursor = origin;
                self.mode = Mode::Normal;
                self.message = Some("Move cancelled".into());
            }
            Action::Confirm => {
                if self.patch.move_module(module_id, self.cursor) {
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

    fn handle_select_key(&mut self, code: KeyCode, anchor: GridPos) {
        let Some(action) = lookup(bindings::select_bindings(), code) else { return };
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
                    extent: self.cursor,
                    move_origin: self.cursor,
                };
            }
            Action::Delete => {
                let ids = self.modules_in_rect(anchor, self.cursor);
                let count = ids.len();
                for id in ids {
                    self.patch.remove_module(id);
                }
                self.mode = Mode::Normal;
                self.message = Some(format!("Deleted {} modules", count));
                self.commit_patch();
            }
            _ => {}
        }
    }

    fn handle_select_move_key(&mut self, code: KeyCode, anchor: GridPos, extent: GridPos, move_origin: GridPos) {
        let Some(action) = lookup(bindings::move_bindings(), code) else { return };
        match action {
            Action::Cancel => {
                self.cursor = move_origin;
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
                let dx = self.cursor.x as i16 - move_origin.x as i16;
                let dy = self.cursor.y as i16 - move_origin.y as i16;
                let ids = self.modules_in_rect(anchor, extent);
                let moves: Vec<_> = ids.iter()
                    .filter_map(|id| {
                        let pos = self.patch.module_position(*id)?;
                        let new_x = (pos.x as i16 + dx).max(0) as u16;
                        let new_y = (pos.y as i16 + dy).max(0) as u16;
                        Some((*id, GridPos::new(new_x, new_y)))
                    })
                    .collect();
                let moved = self.patch.move_modules(&moves);
                self.mode = Mode::Normal;
                self.message = Some(format!("Moved {} modules", moved));
                self.commit_patch();
            }
            _ => {}
        }
    }

    fn modules_in_rect(&self, a: GridPos, b: GridPos) -> Vec<ModuleId> {
        let sel_min_x = a.x.min(b.x);
        let sel_max_x = a.x.max(b.x);
        let sel_min_y = a.y.min(b.y);
        let sel_max_y = a.y.max(b.y);
        
        self.patch.all_modules()
            .filter_map(|m| {
                let pos = self.patch.module_position(m.id)?;
                let mod_min_x = pos.x;
                let mod_min_y = pos.y;
                let mod_max_x = pos.x + m.width() as u16 - 1;
                let mod_max_y = pos.y + m.height() as u16 - 1;
                
                let overlaps = mod_min_x <= sel_max_x && mod_max_x >= sel_min_x
                    && mod_min_y <= sel_max_y && mod_max_y >= sel_min_y;
                
                if overlaps {
                    Some(m.id)
                } else {
                    None
                }
            })
            .collect()
    }

    fn handle_edit_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        let Some(module) = self.patch.module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let defs = module.kind.param_defs();
        let total_params = defs.len();

        let Some(action) = lookup(bindings::edit_bindings(), code) else { return };
        match action {
            Action::Cancel => {
                self.mode = Mode::Normal;
            }
            Action::Down => {
                let new_idx = (param_idx + 1) % total_params;
                self.mode = Mode::Edit { module_id, param_idx: new_idx };
            }
            Action::Up => {
                let new_idx = if param_idx == 0 { total_params - 1 } else { param_idx - 1 };
                self.mode = Mode::Edit { module_id, param_idx: new_idx };
            }
            Action::ValueDown => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        match &def.kind {
                            ParamKind::Float { min, step, .. } => {
                                let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                                m.params.set_float(param_idx, (cur - step).max(*min));
                                m.params.set_connected(param_idx, false);
                            }
                            ParamKind::Enum => {
                                m.params.cycle_enum_prev();
                            }
                            ParamKind::Input => {}
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::ValueUp => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        match &def.kind {
                            ParamKind::Float { max, step, .. } => {
                                let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                                m.params.set_float(param_idx, (cur + step).min(*max));
                                m.params.set_connected(param_idx, false);
                            }
                            ParamKind::Enum => {
                                m.params.cycle_enum_next();
                            }
                            ParamKind::Input => {}
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::ValueDownFast => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let ParamKind::Float { min, step, .. } = &def.kind {
                            let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                            m.params.set_float(param_idx, (cur - step * 10.0).max(*min));
                            m.params.set_connected(param_idx, false);
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::ValueUpFast => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let ParamKind::Float { max, step, .. } = &def.kind {
                            let cur = m.params.get_float(param_idx).unwrap_or(0.0);
                            m.params.set_float(param_idx, (cur + step * 10.0).min(*max));
                            m.params.set_connected(param_idx, false);
                        }
                    }
                    self.commit_patch();
                }
            }
            Action::TogglePort => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if !matches!(def.kind, ParamKind::Input) {
                        if let Some(m) = self.patch.module_mut(module_id) {
                            m.params.toggle_connected(param_idx);
                        }
                        self.commit_patch();
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_save_prompt_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.message = Some("Save cancelled".into());
            }
            KeyCode::Enter => {
                let path = PathBuf::from(self.file_textarea.lines().join(""));
                self.save_to_file(path);
                self.mode = Mode::Normal;
            }
            _ => {
                self.file_textarea.input(Event::Key(event::KeyEvent::new(code, modifiers)));
            }
        }
    }

    fn save_to_file(&mut self, path: PathBuf) {
        let track_text: String = self.track_textarea.lines().join("\n");
        let track = if track_text.trim().is_empty() { None } else { Some(track_text.as_str()) };
        let state = self.track_state.lock().unwrap();
        let bpm = state.clock.current_bpm();
        let bars = state.clock.current_bars();
        drop(state);

        match persist::save_patch(&path, &self.patch, bpm, bars, track) {
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
        match persist::load_patch(&path) {
            Ok((patch, bpm, bars, track)) => {
                self.patch = patch;
                self.file_path = Some(path.clone());
                
                {
                    let mut state = self.track_state.lock().unwrap();
                    state.clock.bpm(bpm).bars(bars);
                }

                if let Some(track_text) = track {
                    self.track_textarea = TextArea::new(track_text.lines().map(|s| s.to_string()).collect());
                    self.track_textarea.set_cursor_line_style(Style::default());
                    self.track_textarea.set_block(Block::default());
                    self.reparse_track();
                }

                self.commit_patch();
                self.message = Some(format!("Loaded {}", path.display()));
            }
            Err(e) => {
                self.message = Some(format!("Load failed: {}", e));
            }
        }
    }

    fn handle_adsr_edit_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        let Some(_) = self.patch.module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        match code {
            KeyCode::Esc | KeyCode::Char('u') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let new_idx = (param_idx + 1) % 2;
                self.mode = Mode::AdsrEdit { module_id, param_idx: new_idx };
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let new_idx = if param_idx == 0 { 1 } else { param_idx - 1 };
                self.mode = Mode::AdsrEdit { module_id, param_idx: new_idx };
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(m) = self.patch.module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr { attack_ratio, sustain, .. } = &mut m.params {
                        match param_idx {
                            0 => *attack_ratio = (*attack_ratio + 0.05).min(1.0),
                            1 => *sustain = (*sustain + 0.05).min(1.0),
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(m) = self.patch.module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr { attack_ratio, sustain, .. } = &mut m.params {
                        match param_idx {
                            0 => *attack_ratio = (*attack_ratio - 0.05).max(0.0),
                            1 => *sustain = (*sustain - 0.05).max(0.0),
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            KeyCode::Char('L') => {
                if let Some(m) = self.patch.module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr { attack_ratio, sustain, .. } = &mut m.params {
                        match param_idx {
                            0 => *attack_ratio = 1.0,
                            1 => *sustain = 1.0,
                            _ => {}
                        }
                    }
                }
                self.commit_patch();
            }
            KeyCode::Char('H') => {
                if let Some(m) = self.patch.module_mut(module_id) {
                    if let super::module::ModuleParams::Adsr { attack_ratio, sustain, .. } = &mut m.params {
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

    fn handle_env_edit_key(&mut self, code: KeyCode, module_id: ModuleId, point_idx: usize, editing: bool) {
        let Some(module) = self.patch.module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let num_points = module.params.env_points().map(|p| p.len()).unwrap_or(0);
        if num_points == 0 {
            self.mode = Mode::Normal;
            return;
        }

        if editing {
            match code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('m') => {
                    self.mode = Mode::EnvEdit { module_id, point_idx, editing: false };
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, 0.01);
                    self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: true };
                    self.commit_patch();
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, -0.01);
                    self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: true };
                    self.commit_patch();
                }
                KeyCode::Char('L') => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, 0.1);
                    self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: true };
                    self.commit_patch();
                }
                KeyCode::Char('H') => {
                    let new_idx = self.adjust_env_point_time(module_id, point_idx, -0.1);
                    self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: true };
                    self.commit_patch();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = (p.value + 0.05).min(1.0);
                            }
                        }
                    }
                    self.commit_patch();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = (p.value - 0.05).max(0.0);
                            }
                        }
                    }
                    self.commit_patch();
                }
                KeyCode::Char('K') => {
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = 1.0;
                            }
                        }
                    }
                    self.commit_patch();
                }
                KeyCode::Char('J') => {
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.value = 0.0;
                            }
                        }
                    }
                    self.commit_patch();
                }
                _ => {}
            }
        } else {
            match code {
                KeyCode::Esc | KeyCode::Char('u') => {
                    self.mode = Mode::Normal;
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    let new_idx = (point_idx + 1) % num_points;
                    self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: false };
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    let new_idx = if point_idx == 0 { num_points - 1 } else { point_idx - 1 };
                    self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: false };
                }
                KeyCode::Char('m') | KeyCode::Enter => {
                    self.mode = Mode::EnvEdit { module_id, point_idx, editing: true };
                }
                KeyCode::Char('c') => {
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if let Some(p) = points.get_mut(point_idx) {
                                p.curve = !p.curve;
                            }
                        }
                    }
                    self.commit_patch();
                }
                KeyCode::Char(' ') => {
                    if let Some(m) = self.patch.module_mut(module_id) {
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
                            let new_idx = points.iter().position(|p| (p.time - new_time).abs() < 0.001).unwrap_or(point_idx);
                            self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: false };
                        }
                    }
                    self.commit_patch();
                }
                KeyCode::Char('.') => {
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let Some(points) = m.params.env_points_mut() {
                            if points.len() > 2 {
                                points.remove(point_idx);
                                let new_idx = point_idx.min(points.len() - 1);
                                self.mode = Mode::EnvEdit { module_id, point_idx: new_idx, editing: false };
                            } else {
                                self.message = Some("Need at least 2 points".into());
                            }
                        }
                    }
                    self.commit_patch();
                }
                _ => {}
            }
        }
    }

    fn adjust_env_point_time(&mut self, module_id: ModuleId, point_idx: usize, delta: f32) -> usize {
        let Some(m) = self.patch.module_mut(module_id) else { return point_idx };
        let Some(points) = m.params.env_points_mut() else { return point_idx };
        let Some(p) = points.get_mut(point_idx) else { return point_idx };
        
        let new_time = (p.time + delta).clamp(0.0, 1.0);
        p.time = new_time;
        
        points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        
        points.iter()
            .position(|p| (p.time - new_time).abs() < 1e-6)
            .unwrap_or(point_idx)
    }

    fn handle_probe_edit_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        let Some(_) = self.patch.module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        match code {
            KeyCode::Esc | KeyCode::Char('u') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let new_idx = (param_idx + 1) % 3;
                self.mode = Mode::ProbeEdit { module_id, param_idx: new_idx };
            }
            KeyCode::Char('h') | KeyCode::Left => {
                let new_idx = if param_idx == 0 { 2 } else { param_idx - 1 };
                self.mode = Mode::ProbeEdit { module_id, param_idx: new_idx };
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match param_idx {
                    0 => self.probe_min -= 0.1,
                    1 => self.probe_max += 0.1,
                    2 => self.probe_len = (self.probe_len * 2).min(44100 * 2),
                    _ => {}
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match param_idx {
                    0 => self.probe_min += 0.1,
                    1 => self.probe_max -= 0.1,
                    2 => self.probe_len = (self.probe_len / 2).max(8),
                    _ => {}
                }
            }
            KeyCode::Char('K') => {
                match param_idx {
                    0 => self.probe_min -= 1.0,
                    1 => self.probe_max += 1.0,
                    2 => self.probe_len = (self.probe_len * 4).min(44100 * 2),
                    _ => {}
                }
            }
            KeyCode::Char('J') => {
                match param_idx {
                    0 => self.probe_min += 1.0,
                    1 => self.probe_max -= 1.0,
                    2 => self.probe_len = (self.probe_len / 4).max(8),
                    _ => {}
                }
            }
            KeyCode::Char('r') => {
                self.probe_min = -1.0;
                self.probe_max = 1.0;
                self.probe_len = 4410;
            }
            KeyCode::Char('c') => {
                let probe_idx = self.patch.all_modules()
                    .filter(|m| m.kind == ModuleKind::Probe)
                    .position(|m| m.id == module_id);
                if let Some(idx) = probe_idx {
                    self.audio_patch.lock().unwrap().clear_probe_history(idx);
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
            .constraints([Constraint::Min(0), Constraint::Length(16)])
            .split(chunks[0]);

        let grid_area = main_chunks[0];
        let help_area = main_chunks[1];
        let status_area = chunks[1];

        let moving_id = if let Mode::Move { module_id, .. } = self.mode {
            Some(module_id)
        } else {
            None
        };

        let selection = match self.mode {
            Mode::Select { anchor } => Some((anchor, self.cursor)),
            Mode::SelectMove { anchor, extent, .. } => Some((anchor, extent)),
            _ => None,
        };

        let audio_patch = self.audio_patch.lock().unwrap();
        let probe_values: Vec<f32> = (0..16)
            .filter_map(|i| audio_patch.probe_history(i).and_then(|h| h.back().copied()))
            .collect();
        drop(audio_patch);
        let grid_widget = GridWidget::new(&self.patch)
            .cursor(self.cursor)
            .moving(moving_id)
            .selection(selection)
            .probe_values(&probe_values);
        f.render_widget(grid_widget, grid_area);

        let help_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        f.render_widget(help_block, help_area);

        let help_inner = Rect::new(
            help_area.x + 2,
            help_area.y + 1,
            help_area.width.saturating_sub(3),
            help_area.height.saturating_sub(1),
        );
        f.render_widget(HelpWidget, help_inner);

        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Palette => "PALETTE",
            Mode::Move { .. } => "MOVE",
            Mode::Select { .. } => "SELECT",
            Mode::SelectMove { .. } => "SEL-MOVE",
            Mode::Edit { .. } => "EDIT",
            Mode::AdsrEdit { .. } => "ADSR",
            Mode::EnvEdit { .. } => "ENV",
            Mode::ProbeEdit { .. } => "PROBE",
            Mode::SavePrompt => "SAVE",
            Mode::QuitConfirm => "QUIT?",
        };
        let mut status = StatusWidget::new(self.cursor, mode_str).playing(self.playing);
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
                .filter(&self.palette_filter, self.filtered_modules(), self.palette_filter_selection, self.palette_searching);
            f.render_widget(palette, inner);
        }

        if let Mode::Edit { module_id, param_idx } = self.mode {
            if let Some(module) = self.patch.module(module_id) {
                let edit_width = 36;
                let edit_height = (module.kind.param_defs().len() + 6) as u16;
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
                let edit_widget = EditWidget::new(module, param_idx);
                f.render_widget(edit_widget, inner);
            }
        }

        if let Mode::AdsrEdit { module_id, param_idx } = self.mode {
            if let Some(module) = self.patch.module(module_id) {
                let env_width = f.area().width.saturating_sub(4);
                let env_height = f.area().height.saturating_sub(6);
                let env_x = 2;
                let env_y = 2;
                let env_area = Rect::new(env_x, env_y, env_width, env_height);

                f.render_widget(Clear, env_area);

                let env_block = Block::default()
                    .title(" ADSR (jk select, hl adjust, Esc close) ")
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

        if let Mode::EnvEdit { module_id, point_idx, editing } = self.mode {
            if let Some(module) = self.patch.module(module_id) {
                let env_width = f.area().width.saturating_sub(4);
                let env_height = f.area().height.saturating_sub(6);
                let env_x = 2;
                let env_y = 2;
                let env_area = Rect::new(env_x, env_y, env_width, env_height);

                f.render_widget(Clear, env_area);

                let title = if editing {
                    " Envelope [MOVE] (hjkl move, Esc done) "
                } else {
                    " Envelope (hl select, m move, c curve, Space add, . del) "
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

        if let Mode::ProbeEdit { module_id, param_idx } = self.mode {
            if let Some(module) = self.patch.module(module_id) {
                let probe_idx = self.patch.all_modules()
                    .filter(|m| m.kind == ModuleKind::Probe)
                    .position(|m| m.id == module_id);
                
                let audio_patch = self.audio_patch.lock().unwrap();
                let history: Vec<f32> = probe_idx
                    .and_then(|i| audio_patch.probe_history(i))
                    .map(|h| h.iter().copied().collect())
                    .unwrap_or_default();
                let current = history.last().copied().unwrap_or(0.0);
                drop(audio_patch);

                let probe_width = f.area().width.saturating_sub(4);
                let probe_height = f.area().height.saturating_sub(6);
                let probe_x = 2;
                let probe_y = 2;
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
                let probe_widget = ProbeWidget::new(&history, self.probe_min, self.probe_max, self.probe_len, current, param_idx);
                f.render_widget(probe_widget, inner);
            }
        }

        if matches!(self.mode, Mode::SavePrompt) {
            let prompt_width = 50u16.min(f.area().width.saturating_sub(4));
            let prompt_height = 3u16;
            let prompt_x = (f.area().width.saturating_sub(prompt_width)) / 2;
            let prompt_y = (f.area().height.saturating_sub(prompt_height)) / 2;
            let prompt_area = Rect::new(prompt_x, prompt_y, prompt_width, prompt_height);

            f.render_widget(Clear, prompt_area);

            let prompt_block = Block::default()
                .title(" Save As (Enter confirm, Esc cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(prompt_block, prompt_area);

            let inner = Rect::new(
                prompt_area.x + 1,
                prompt_area.y + 1,
                prompt_area.width.saturating_sub(2),
                prompt_area.height.saturating_sub(2),
            );
            f.render_widget(&self.file_textarea, inner);
        }
    }
}

const NUM_VOICES: usize = 6;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let file_arg = std::env::args().nth(1).map(PathBuf::from);
    
    let audio_patch = Arc::new(Mutex::new(CompiledPatch::default()));
    let audio_patch_clone = Arc::clone(&audio_patch);
    let track_state = Arc::new(Mutex::new(TrackState::new(NUM_VOICES)));
    let track_state_clone = Arc::clone(&track_state);
    
    let player = AudioPlayer::new()?;
    let playing = Arc::new(Mutex::new(false));
    let playing_clone = Arc::clone(&playing);
    
    let stream = {
        use cpal::traits::DeviceTrait;
        use assert_no_alloc::assert_no_alloc;
        
        let signal = Arc::new(Mutex::new(Signal::new(player.config.sample_rate.0 as usize)));
        let channels = player.config.channels as usize;
        
        player.device.build_output_stream(
            &player.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let is_playing = *playing_clone.lock().unwrap();
                if !is_playing {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }
                
                let mut signal_lock = signal.lock().unwrap();
                let mut patch = audio_patch_clone.lock().unwrap();
                let mut track = track_state_clone.lock().unwrap();
                
                for frame in data.chunks_mut(channels) {
                    track.update(&mut signal_lock);
                    let sample = assert_no_alloc(|| {
                        patch.process(&mut signal_lock, &track).clamp(-1., 1.)
                    });
                    
                    for channel_sample in frame.iter_mut() {
                        *channel_sample = sample;
                    }
                    
                    signal_lock.advance();
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

    let mut app = App::new(audio_patch, track_state);

    if let Some(path) = file_arg {
        app.load_from_file(path);
    }

    loop {
        *playing.lock().unwrap() = app.playing;
        
        terminal.draw(|f| app.ui(f))?;

        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Key(key) = &event {
                app.handle_key(key.code, key.modifiers);
            }
        }

        if let Some(request) = app.pending_request.take() {
            match request {
                AppRequest::EditTrack => {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    
                    let track_text: String = app.track_textarea.lines().join("\n");
                    let temp_path = std::env::temp_dir().join("brainwash_track.txt");
                    fs::write(&temp_path, &track_text)?;
                    
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
                    let status = Command::new(&editor).arg(&temp_path).status();
                    
                    if let Ok(s) = status {
                        if s.success() {
                            if let Ok(new_text) = fs::read_to_string(&temp_path) {
                                app.track_textarea = TextArea::new(new_text.lines().map(|s| s.to_string()).collect());
                                app.track_textarea.set_cursor_line_style(Style::default());
                                app.track_textarea.set_block(Block::default());
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
