use super::grid::GridPos;
use super::module::{ModuleCategory, ModuleId, ModuleKind, ParamKind};
use super::patch::Patch;
use super::engine::{CompiledPatch, compile_patch, TrackState};
use super::render::{EditWidget, GridWidget, HelpWidget, PaletteWidget, StatusWidget};
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
use std::{io, time::Duration};

#[derive(Clone, PartialEq)]
enum Mode {
    Normal,
    Palette,
    Move { module_id: ModuleId, origin: GridPos },
    Edit { module_id: ModuleId, param_idx: usize },
    TrackEdit,
}

struct App<'a> {
    patch: Patch,
    cursor: GridPos,
    mode: Mode,
    palette_category: usize,
    palette_selections: [usize; 7],
    message: Option<String>,
    should_quit: bool,
    audio_patch: Arc<Mutex<CompiledPatch>>,
    track_state: Arc<Mutex<TrackState>>,
    playing: bool,
    track_textarea: TextArea<'a>,
}

impl<'a> App<'a> {
    fn new(audio_patch: Arc<Mutex<CompiledPatch>>, track_state: Arc<Mutex<TrackState>>) -> Self {
        let mut patch = Patch::new(20, 20);
        patch.add_module(ModuleKind::Output, GridPos::new(19, 19));

        let mut textarea = TextArea::new(vec!["(0/2/4/7)".into()]);
        textarea.set_cursor_line_style(Style::default());
        textarea.set_block(Block::default());

        Self {
            patch,
            cursor: GridPos::new(0, 0),
            mode: Mode::Normal,
            palette_category: 0,
            palette_selections: [0; 7],
            message: None,
            should_quit: false,
            audio_patch,
            track_state,
            playing: false,
            track_textarea: textarea,
        }
    }

    fn reparse_track(&mut self) {
        let scale = cmin();
        let notation: String = self.track_textarea.lines().join("");
        match Track::parse(&notation, &scale) {
            Ok(track) => {
                let mut state = self.track_state.lock().unwrap();
                state.track = Some(track);
                self.message = Some("Track updated".into());
            }
            Err(e) => {
                self.message = Some(format!("Parse error: {}", e));
            }
        }
    }

    fn commit_patch(&mut self) {
        let compiled = compile_patch(&self.patch);
        let mut audio = self.audio_patch.lock().unwrap();
        *audio = compiled;
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
            Mode::Edit { module_id, param_idx } => self.handle_edit_key(code, module_id, param_idx),
            Mode::TrackEdit => self.handle_track_edit_key(code, modifiers),
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('h') | KeyCode::Left => self.move_cursor(-1, 0),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(0, 1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(0, -1),
            KeyCode::Char('l') | KeyCode::Right => self.move_cursor(1, 0),
            KeyCode::Char('H') => self.move_cursor(-4, 0),
            KeyCode::Char('J') => self.move_cursor(0, 4),
            KeyCode::Char('K') => self.move_cursor(0, -4),
            KeyCode::Char('L') => self.move_cursor(4, 0),
            KeyCode::Char('[') => self.cursor = GridPos::new(0, 0),
            KeyCode::Char(']') => {
                if let Some(out_id) = self.patch.output_id() {
                    if let Some(pos) = self.patch.module_position(out_id) {
                        self.cursor = pos;
                    }
                }
            }
            KeyCode::Char(' ') => {
                self.mode = Mode::Palette;
            }
            KeyCode::Char('m') | KeyCode::Enter => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    self.mode = Mode::Move { module_id: id, origin: self.cursor };
                    self.message = Some("Move mode - hjkl to move, Enter/m to place, Esc to cancel".into());
                }
            }
            KeyCode::Char('.') | KeyCode::Backspace => {
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
            KeyCode::Char('o') => {
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
            KeyCode::Char('u') => {
                if let Some(id) = self.patch.module_id_at(self.cursor) {
                    if let Some(m) = self.patch.module(id) {
                        let defs = m.kind.param_defs();
                        if !defs.is_empty() {
                            self.mode = Mode::Edit { module_id: id, param_idx: 0 };
                        } else {
                            self.message = Some("No params to edit".into());
                        }
                    }
                }
            }
            KeyCode::Char('7') => self.open_palette_category(0),
            KeyCode::Char('8') => self.open_palette_category(1),
            KeyCode::Char('9') => self.open_palette_category(2),
            KeyCode::Char('0') => self.open_palette_category(3),
            KeyCode::Char('-') => self.open_palette_category(4),
            KeyCode::Char('=') => self.open_palette_category(5),
            KeyCode::Char('p') => {
                self.playing = !self.playing;
                self.message = Some(if self.playing { "Playing".into() } else { "Paused".into() });
            }
            KeyCode::Char('t') => {
                self.mode = Mode::TrackEdit;
            }
            _ => {}
        }
    }

    fn open_palette_category(&mut self, cat: usize) {
        self.palette_category = cat;
        self.mode = Mode::Palette;
    }

    fn handle_palette_key(&mut self, code: KeyCode) {
        let categories = ModuleCategory::all();
        let current_cat = categories[self.palette_category];
        let modules = ModuleKind::by_category(current_cat);
        let palette_module = self.palette_selections[self.palette_category];

        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
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
            KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => {
                self.palette_category = (self.palette_category + 1) % categories.len();
            }
            KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
                self.palette_category = if self.palette_category == 0 {
                    categories.len() - 1
                } else {
                    self.palette_category - 1
                };
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
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
        match code {
            KeyCode::Esc => {
                self.cursor = origin;
                self.mode = Mode::Normal;
                self.message = Some("Move cancelled".into());
            }
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('m') => {
                if self.patch.move_module(module_id, self.cursor) {
                    self.mode = Mode::Normal;
                    self.message = Some("Moved".into());
                    self.commit_patch();
                } else {
                    self.message = Some("Can't place here".into());
                }
            }
            KeyCode::Char('h') | KeyCode::Left => self.move_cursor(-1, 0),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(0, 1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(0, -1),
            KeyCode::Char('l') | KeyCode::Right => self.move_cursor(1, 0),
            KeyCode::Char('H') => self.move_cursor(-4, 0),
            KeyCode::Char('J') => self.move_cursor(0, 4),
            KeyCode::Char('K') => self.move_cursor(0, -4),
            KeyCode::Char('L') => self.move_cursor(4, 0),
            _ => {}
        }
    }

    fn handle_edit_key(&mut self, code: KeyCode, module_id: ModuleId, param_idx: usize) {
        let Some(module) = self.patch.module(module_id) else {
            self.mode = Mode::Normal;
            return;
        };

        let defs = module.kind.param_defs();
        let total_params = defs.len();

        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('u') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Tab | KeyCode::Char('j') | KeyCode::Down => {
                let new_idx = (param_idx + 1) % total_params;
                self.mode = Mode::Edit { module_id, param_idx: new_idx };
            }
            KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
                let new_idx = if param_idx == 0 { total_params - 1 } else { param_idx - 1 };
                self.mode = Mode::Edit { module_id, param_idx: new_idx };
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        match &def.kind {
                            ParamKind::Float { min, step, .. } => {
                                m.params.floats[param_idx] = (m.params.floats[param_idx] - step).max(*min);
                                m.params.set_connected(param_idx, false);
                            }
                            ParamKind::Enum { options } => {
                                let cur = m.params.floats[param_idx] as usize;
                                m.params.floats[param_idx] = if cur == 0 { (options.len() - 1) as f32 } else { (cur - 1) as f32 };
                            }
                            ParamKind::Input => {}
                        }
                    }
                    self.commit_patch();
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        match &def.kind {
                            ParamKind::Float { max, step, .. } => {
                                m.params.floats[param_idx] = (m.params.floats[param_idx] + step).min(*max);
                                m.params.set_connected(param_idx, false);
                            }
                            ParamKind::Enum { options } => {
                                let cur = m.params.floats[param_idx] as usize;
                                m.params.floats[param_idx] = ((cur + 1) % options.len()) as f32;
                            }
                            ParamKind::Input => {}
                        }
                    }
                    self.commit_patch();
                }
            }
            KeyCode::Char('H') => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let ParamKind::Float { min, step, .. } = &def.kind {
                            m.params.floats[param_idx] = (m.params.floats[param_idx] - step * 10.0).max(*min);
                            m.params.set_connected(param_idx, false);
                        }
                    }
                    self.commit_patch();
                }
            }
            KeyCode::Char('L') => {
                if param_idx < defs.len() {
                    let def = &defs[param_idx];
                    if let Some(m) = self.patch.module_mut(module_id) {
                        if let ParamKind::Float { max, step, .. } = &def.kind {
                            m.params.floats[param_idx] = (m.params.floats[param_idx] + step * 10.0).min(*max);
                            m.params.set_connected(param_idx, false);
                        }
                    }
                    self.commit_patch();
                }
            }
            KeyCode::Char(';') => {
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

    fn handle_track_edit_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if code == KeyCode::Esc {
            self.message = Some("Track edit cancelled".into());
            self.mode = Mode::Normal;
        } else if code == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
            self.reparse_track();
            self.mode = Mode::Normal;
        }
    }

    fn handle_track_event(&mut self, event: &Event) {
        if let Event::Key(_) = event {
            self.track_textarea.input(event.clone());
        }
    }

    fn ui(&self, f: &mut Frame) {
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

        let grid_widget = GridWidget::new(&self.patch)
            .cursor(self.cursor)
            .moving(moving_id);
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
            Mode::Palette => "SELECT",
            Mode::Move { .. } => "MOVE",
            Mode::Edit { .. } => "EDIT",
            Mode::TrackEdit => "TRACK",
        };
        let mut status = StatusWidget::new(self.cursor, mode_str).playing(self.playing);
        if let Some(ref msg) = self.message {
            status = status.message(msg);
        }
        f.render_widget(status, status_area);

        if self.mode == Mode::Palette {
            let palette_width = 18;
            let palette_height = 16;
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
                .selected_module(self.palette_selections[self.palette_category]);
            f.render_widget(palette, inner);
        }

        if let Mode::Edit { module_id, param_idx } = self.mode {
            if let Some(module) = self.patch.module(module_id) {
                let edit_width = 24;
                let edit_height = (module.kind.param_defs().len() + 4) as u16;
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

        if self.mode == Mode::TrackEdit {
            let track_width = 50.min(f.area().width.saturating_sub(4));
            let track_height = 10.min(f.area().height.saturating_sub(4));
            let track_x = (f.area().width.saturating_sub(track_width)) / 2;
            let track_y = (f.area().height.saturating_sub(track_height)) / 2;
            let track_area = Rect::new(track_x, track_y, track_width, track_height);

            f.render_widget(Clear, track_area);

            let track_block = Block::default()
                .title(" Track (Ctrl-s save, Esc cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(track_block, track_area);

            let inner = Rect::new(
                track_area.x + 1,
                track_area.y + 1,
                track_area.width.saturating_sub(2),
                track_area.height.saturating_sub(2),
            );

            f.render_widget(&self.track_textarea, inner);
        }
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let audio_patch = Arc::new(Mutex::new(CompiledPatch::default()));
    let audio_patch_clone = Arc::clone(&audio_patch);
    let track_state = Arc::new(Mutex::new(TrackState::default()));
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
                        patch.process(&mut signal_lock, track.current_freq, track.current_gate).clamp(-1., 1.)
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

    loop {
        *playing.lock().unwrap() = app.playing;
        
        terminal.draw(|f| app.ui(f))?;

        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Key(key) = &event {
                if app.mode == Mode::TrackEdit {
                    app.handle_track_event(&event);
                }
                app.handle_key(key.code, key.modifiers);
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
