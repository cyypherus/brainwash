use ratatui::crossterm::event::KeyCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::tui::bindings::{Action, Binding};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub normal: HashMap<String, KeyList>,
    pub palette: HashMap<String, KeyList>,
    pub edit: HashMap<String, KeyList>,
    pub env: HashMap<String, KeyList>,
    pub env_move: HashMap<String, KeyList>,
    pub select: HashMap<String, KeyList>,
    pub settings: HashMap<String, KeyList>,
    pub probe: HashMap<String, KeyList>,
    pub sample: HashMap<String, KeyList>,
    #[serde(rename = "move")]
    pub move_mode: HashMap<String, KeyList>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum KeyList {
    Single(String),
    Multiple(Vec<String>),
}

impl KeyList {
    pub fn keys(&self) -> Vec<&str> {
        match self {
            KeyList::Single(s) => vec![s.as_str()],
            KeyList::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    dirs_path().map(|p| p.join("bindings.toml"))
}

fn dirs_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("brainwash"))
}

pub fn load_config() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    if !path.exists() {
        return Config::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to parse {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Failed to read {}: {}", path.display(), e);
            Config::default()
        }
    }
}

pub fn parse_key(s: &str) -> Option<KeyCode> {
    match s.to_lowercase().as_str() {
        "esc" | "escape" => Some(KeyCode::Esc),
        "enter" | "return" | "ret" => Some(KeyCode::Enter),
        "tab" => Some(KeyCode::Tab),
        "backtab" | "s-tab" => Some(KeyCode::BackTab),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "space" => Some(KeyCode::Char(' ')),
        "backspace" => Some(KeyCode::Backspace),
        "delete" | "del" => Some(KeyCode::Delete),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" => Some(KeyCode::PageUp),
        "pagedown" => Some(KeyCode::PageDown),
        _ => {
            let chars: Vec<char> = s.chars().collect();
            if chars.len() == 1 {
                Some(KeyCode::Char(chars[0]))
            } else {
                None
            }
        }
    }
}

pub fn parse_action(s: &str) -> Option<Action> {
    match s {
        "quit" => Some(Action::Quit),
        "left" => Some(Action::Left),
        "down" => Some(Action::Down),
        "up" => Some(Action::Up),
        "right" => Some(Action::Right),
        "left_fast" => Some(Action::LeftFast),
        "down_fast" => Some(Action::DownFast),
        "up_fast" => Some(Action::UpFast),
        "right_fast" => Some(Action::RightFast),
        "place" => Some(Action::Place),
        "delete" => Some(Action::Delete),
        "edit" => Some(Action::Edit),
        "move" => Some(Action::Move),
        "rotate" => Some(Action::Rotate),
        "toggle_play" => Some(Action::TogglePlay),
        "track_edit" => Some(Action::TrackEdit),
        "select" => Some(Action::Select),
        "save" => Some(Action::Save),
        "save_as" => Some(Action::SaveAs),
        "copy" => Some(Action::Copy),
        "confirm" => Some(Action::Confirm),
        "cancel" => Some(Action::Cancel),
        "toggle_port" => Some(Action::TogglePort),
        "value_down" => Some(Action::ValueDown),
        "value_up" => Some(Action::ValueUp),
        "value_down_fast" => Some(Action::ValueDownFast),
        "value_up_fast" => Some(Action::ValueUpFast),
        "cycle_unit" => Some(Action::CycleUnit),
        "add_point" => Some(Action::AddPoint),
        "delete_point" => Some(Action::DeletePoint),
        "toggle_curve" => Some(Action::ToggleCurve),
        "track_settings" => Some(Action::TrackSettings),
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "search" => Some(Action::Search),
        "edit_subpatch" => Some(Action::EditSubpatch),
        "exit_subpatch" => Some(Action::ExitSubpatch),
        "toggle_meters" => Some(Action::ToggleMeters),
        "open_palette" => Some(Action::OpenPalette),
        "export" => Some(Action::Export),
        "type_value" => Some(Action::TypeValue),
        "cycle_step" => Some(Action::CycleStep),
        "new_instrument" => Some(Action::NewInstrument),
        "help_scroll_up" => Some(Action::HelpScrollUp),
        "help_scroll_down" => Some(Action::HelpScrollDown),
        s if s.starts_with("palette_") => s[8..].parse().ok().map(Action::Palette),
        s if s.starts_with("instrument_") => s[11..].parse().ok().map(Action::Instrument),
        _ => None,
    }
}

pub fn apply_overrides(bindings: &mut Vec<Binding>, overrides: &HashMap<String, KeyList>) {
    for (action_str, keys) in overrides {
        let Some(action) = parse_action(action_str) else {
            continue;
        };
        bindings.retain(|b| !matches_action(b.action, action));
        for key_str in keys.keys() {
            if let Some(key) = parse_key(key_str) {
                bindings.push(Binding {
                    key,
                    action,
                    hint: None,
                    group: None,
                    section: 0,
                });
            }
        }
    }
}

fn matches_action(a: Action, b: Action) -> bool {
    std::mem::discriminant(&a) == std::mem::discriminant(&b)
}

pub struct Bindings {
    pub normal: Vec<Binding>,
    pub palette: Vec<Binding>,
    pub move_mode: Vec<Binding>,
    pub select: Vec<Binding>,
    pub edit: Vec<Binding>,
    pub env: Vec<Binding>,
    pub env_move: Vec<Binding>,
    pub settings: Vec<Binding>,
    pub probe: Vec<Binding>,
    pub sample: Vec<Binding>,
    pub quit_confirm: Vec<Binding>,
    pub text_input: Vec<Binding>,
}

impl Bindings {
    pub fn load() -> Self {
        use crate::tui::bindings;
        let config = load_config();

        let mut normal: Vec<Binding> = bindings::normal_bindings().to_vec();
        let mut palette: Vec<Binding> = bindings::palette_bindings().to_vec();
        let mut move_mode: Vec<Binding> = bindings::move_bindings().to_vec();
        let mut select: Vec<Binding> = bindings::select_bindings().to_vec();
        let mut edit: Vec<Binding> = bindings::edit_bindings().to_vec();
        let mut env: Vec<Binding> = bindings::env_bindings().to_vec();
        let mut env_move: Vec<Binding> = bindings::env_move_bindings().to_vec();
        let mut settings: Vec<Binding> = bindings::settings_bindings().to_vec();
        let mut probe: Vec<Binding> = bindings::probe_bindings().to_vec();
        let mut sample: Vec<Binding> = bindings::sample_bindings().to_vec();
        let quit_confirm: Vec<Binding> = bindings::quit_confirm_bindings().to_vec();
        let text_input: Vec<Binding> = bindings::text_input_bindings().to_vec();

        apply_overrides(&mut normal, &config.normal);
        apply_overrides(&mut palette, &config.palette);
        apply_overrides(&mut move_mode, &config.move_mode);
        apply_overrides(&mut select, &config.select);
        apply_overrides(&mut edit, &config.edit);
        apply_overrides(&mut env, &config.env);
        apply_overrides(&mut env_move, &config.env_move);
        apply_overrides(&mut settings, &config.settings);
        apply_overrides(&mut probe, &config.probe);
        apply_overrides(&mut sample, &config.sample);

        Self {
            normal,
            palette,
            move_mode,
            select,
            edit,
            env,
            env_move,
            settings,
            probe,
            sample,
            quit_confirm,
            text_input,
        }
    }
}
