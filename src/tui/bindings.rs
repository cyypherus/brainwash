use ratatui::crossterm::event::KeyCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Left,
    Down,
    Up,
    Right,
    LeftFast,
    DownFast,
    UpFast,
    RightFast,
    Place,
    Delete,
    Edit,
    Move,
    Rotate,
    TogglePlay,
    TrackEdit,
    Select,
    Save,
    SaveAs,
    Copy,
    Palette(usize),
    Confirm,
    Cancel,
    TogglePort,
    ValueDown,
    ValueUp,
    ValueDownFast,
    ValueUpFast,
    AddPoint,
    DeletePoint,
    ToggleCurve,
    TrackSettings,
    Undo,
    Redo,
    Search,
    MakeSubpatch,
    EditSubpatch,
    ExitSubpatch,
    ToggleMeters,
}

impl Action {
    pub fn hint(self) -> &'static str {
        match self {
            Action::Quit => "quit",
            Action::Left | Action::Down | Action::Up | Action::Right => "move",
            Action::LeftFast | Action::DownFast | Action::UpFast | Action::RightFast => "move fast",
            Action::Place => "place",
            Action::Delete => "delete",
            Action::Edit => "info",
            Action::Move => "grab",
            Action::Rotate => "rotate",
            Action::TogglePlay => "play",
            Action::TrackEdit => "track",
            Action::Select => "multi select",
            Action::Save => "save",
            Action::SaveAs => "save as",
            Action::Copy => "duplicate",
            Action::Palette(_) => "modules",
            Action::Confirm => "confirm",
            Action::Cancel => "cancel",
            Action::TogglePort => "port",
            Action::ValueDown | Action::ValueUp => "adjust",
            Action::ValueDownFast | Action::ValueUpFast => "adjust fast",
            Action::AddPoint => "add",
            Action::DeletePoint => "delete",
            Action::ToggleCurve => "curve",
            Action::TrackSettings => "settings",
            Action::Undo => "undo",
            Action::Redo => "redo",
            Action::Search => "search",
            Action::MakeSubpatch => "subpatch",
            Action::EditSubpatch | Action::ExitSubpatch => "sub",
            Action::ToggleMeters => "meters",
        }
    }
}

pub struct Binding {
    pub key: KeyCode,
    pub action: Action,
    pub hint: Option<&'static str>,
    pub group: Option<&'static str>,
    pub section: u8,
}

impl Binding {
    pub fn hint(&self) -> &'static str {
        self.hint.unwrap_or_else(|| self.action.hint())
    }
}

pub fn normal_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: None, group: Some("arrows"), section: 0 },

        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: None, group: None, section: 1 },
        Binding { key: KeyCode::Char('i'), action: Action::Edit, hint: None, group: None, section: 1 },
        Binding { key: KeyCode::Char('.'), action: Action::Delete, hint: None, group: None, section: 1 },
        Binding { key: KeyCode::Char('o'), action: Action::Rotate, hint: None, group: None, section: 1 },
        Binding { key: KeyCode::Char(','), action: Action::Select, hint: None, group: None, section: 1 },

        Binding { key: KeyCode::Char('u'), action: Action::Undo, hint: Some("undo/redo"), group: Some("undo"), section: 2 },
        Binding { key: KeyCode::Char('U'), action: Action::Redo, hint: Some("undo/redo"), group: Some("undo"), section: 2 },
        Binding { key: KeyCode::Char('y'), action: Action::Copy, hint: None, group: None, section: 2 },

        Binding { key: KeyCode::Char(' '), action: Action::TogglePlay, hint: None, group: None, section: 3 },
        Binding { key: KeyCode::Char('t'), action: Action::TrackEdit, hint: None, group: None, section: 3 },
        Binding { key: KeyCode::Char('s'), action: Action::TrackSettings, hint: None, group: None, section: 3 },
        Binding { key: KeyCode::Char('v'), action: Action::ToggleMeters, hint: None, group: None, section: 3 },

        Binding { key: KeyCode::Char('p'), action: Action::EditSubpatch, hint: None, group: None, section: 4 },

        Binding { key: KeyCode::Char('n'), action: Action::Palette(0), hint: Some("new module"), group: None, section: 5 },
        Binding { key: KeyCode::Char('7'), action: Action::Palette(0), hint: None, group: Some("palette"), section: 5 },
        Binding { key: KeyCode::Char('8'), action: Action::Palette(1), hint: None, group: Some("palette"), section: 5 },
        Binding { key: KeyCode::Char('9'), action: Action::Palette(2), hint: None, group: Some("palette"), section: 5 },
        Binding { key: KeyCode::Char('0'), action: Action::Palette(3), hint: None, group: Some("palette"), section: 5 },
        Binding { key: KeyCode::Char('-'), action: Action::Palette(4), hint: None, group: Some("palette"), section: 5 },
        Binding { key: KeyCode::Char('='), action: Action::Palette(5), hint: None, group: Some("palette"), section: 5 },
        Binding { key: KeyCode::Char('`'), action: Action::Palette(6), hint: None, group: Some("palette"), section: 5 },

        Binding { key: KeyCode::Char('w'), action: Action::Save, hint: None, group: None, section: 6 },
        Binding { key: KeyCode::Char('W'), action: Action::SaveAs, hint: None, group: None, section: 6 },
        Binding { key: KeyCode::Char('Q'), action: Action::Quit, hint: None, group: None, section: 6 },
    ]
}

pub fn palette_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: Some("place"), group: Some("place"), section: 0 },
        Binding { key: KeyCode::Char('n'), action: Action::Confirm, hint: Some("place"), group: Some("place"), section: 0 },
        Binding { key: KeyCode::Char('/'), action: Action::Search, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: None, group: Some("cancel"), section: 0 },
        Binding { key: KeyCode::Char(' '), action: Action::Cancel, hint: None, group: Some("cancel"), section: 0 },
    ]
}

pub fn move_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Char('m'), action: Action::Confirm, hint: Some("place"), group: Some("place"), section: 0 },
        Binding { key: KeyCode::Char('y'), action: Action::Confirm, hint: Some("place"), group: Some("place"), section: 0 },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: Some("place"), group: Some("place"), section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: None, group: None, section: 0 },
    ]
}

pub fn edit_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: Some("param"), group: Some("jk"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: Some("param"), group: Some("jk"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: Some("param"), group: Some("ud"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: Some("param"), group: Some("ud"), section: 0 },
        Binding { key: KeyCode::Char('h'), action: Action::ValueDown, hint: None, group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::ValueUp, hint: None, group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::ValueDown, hint: None, group: Some("lr"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::ValueUp, hint: None, group: Some("lr"), section: 0 },
        Binding { key: KeyCode::Char(';'), action: Action::TogglePort, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Char('i'), action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
    ]
}

pub fn env_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: Some("point"), group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: Some("point"), group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: Some("point"), group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: Some("point"), group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: Some("move"), group: None, section: 0 },
        Binding { key: KeyCode::Char(' '), action: Action::AddPoint, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Char('.'), action: Action::DeletePoint, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Char('c'), action: Action::ToggleCurve, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Char('i'), action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
    ]
}

pub fn env_move_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Char('m'), action: Action::Confirm, hint: Some("done"), group: Some("done"), section: 0 },
    ]
}

pub fn select_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: None, group: Some("hjkl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: None, group: Some("arrows"), section: 0 },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: None, group: Some("grab"), section: 0 },
        Binding { key: KeyCode::Enter, action: Action::Move, hint: None, group: Some("grab"), section: 0 },
        Binding { key: KeyCode::Char('.'), action: Action::Delete, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Char('y'), action: Action::Copy, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Char('p'), action: Action::MakeSubpatch, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: None, group: Some("cancel"), section: 0 },
        Binding { key: KeyCode::Char(','), action: Action::Cancel, hint: None, group: Some("cancel"), section: 0 },
    ]
}

pub fn quit_confirm_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('y'), action: Action::Confirm, hint: Some("yes"), group: Some("yes"), section: 0 },
        Binding { key: KeyCode::Char('Y'), action: Action::Confirm, hint: Some("yes"), group: Some("yes"), section: 0 },
        Binding { key: KeyCode::Char('n'), action: Action::Cancel, hint: Some("no"), group: Some("no"), section: 0 },
        Binding { key: KeyCode::Char('N'), action: Action::Cancel, hint: Some("no"), group: Some("no"), section: 0 },
    ]
}

pub fn settings_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: Some("param"), group: Some("jk"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: Some("param"), group: Some("jk"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::Down, hint: Some("param"), group: Some("ud"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::Up, hint: Some("param"), group: Some("ud"), section: 0 },
        Binding { key: KeyCode::Char('h'), action: Action::ValueDown, hint: None, group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::ValueUp, hint: None, group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::ValueDown, hint: None, group: Some("lr"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::ValueUp, hint: None, group: Some("lr"), section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Char('s'), action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Enter, action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
    ]
}

pub fn text_input_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: None, group: None, section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: None, group: None, section: 0 },
    ]
}

pub fn probe_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: Some("param"), group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: Some("param"), group: Some("hl"), section: 0 },
        Binding { key: KeyCode::Left, action: Action::Left, hint: Some("param"), group: Some("lr"), section: 0 },
        Binding { key: KeyCode::Right, action: Action::Right, hint: Some("param"), group: Some("lr"), section: 0 },
        Binding { key: KeyCode::Char('j'), action: Action::ValueDown, hint: None, group: Some("jk"), section: 0 },
        Binding { key: KeyCode::Char('k'), action: Action::ValueUp, hint: None, group: Some("jk"), section: 0 },
        Binding { key: KeyCode::Down, action: Action::ValueDown, hint: None, group: Some("ud"), section: 0 },
        Binding { key: KeyCode::Up, action: Action::ValueUp, hint: None, group: Some("ud"), section: 0 },
        Binding { key: KeyCode::Char('r'), action: Action::Delete, hint: Some("reset"), group: None, section: 0 },
        Binding { key: KeyCode::Char('c'), action: Action::ToggleCurve, hint: Some("cycle"), group: None, section: 0 },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
        Binding { key: KeyCode::Char('i'), action: Action::Cancel, hint: Some("done"), group: Some("done"), section: 0 },
    ]
}

pub fn lookup(bindings: &[Binding], key: KeyCode) -> Option<Action> {
    bindings.iter().find(|b| b.key == key).map(|b| b.action)
}

pub fn hints(bindings: &[Binding]) -> Vec<(String, &'static str, u8)> {
    let mut result: Vec<(String, &'static str, Option<&'static str>, u8)> = Vec::new();
    for b in bindings {
        let hint = b.hint();
        if let Some(group) = b.group {
            if let Some(existing) = result.iter_mut().find(|(_, _, g, _)| *g == Some(group)) {
                existing.0.push('/');
                existing.0.push_str(key_str(b.key));
                continue;
            }
        }
        if result.iter().any(|(_, h, g, _)| *h == hint && g.is_none() && b.group.is_none()) {
            continue;
        }
        result.push((key_str(b.key).to_string(), hint, b.group, b.section));
    }
    result.into_iter().map(|(k, h, _, s)| (k, h, s)).collect()
}

pub fn key_str(key: KeyCode) -> &'static str {
    match key {
        KeyCode::Char(c) => match c {
            'h' => "h",
            'j' => "j",
            'k' => "k",
            'l' => "l",
            'H' => "H",
            'J' => "J",
            'K' => "K",
            'L' => "L",
            'q' => "q",
            'Q' => "Q",
            'u' => "u",
            'U' => "U",
            'y' => "y",
            'Y' => "Y",
            'i' => "i",
            'm' => "m",
            'n' => "n",
            'o' => "o",
            'p' => "p",
            'r' => "r",
            't' => "t",
            's' => "s",
            'S' => "S",
            'e' => "e",
            'c' => "c",
            'x' => "x",
            'v' => "v",
            '`' => "`",
            ',' => ",",
            'w' => "w",
            'W' => "W",
            ' ' => "space",
            '.' => ".",
            ';' => ";",
            '/' => "/",
            '7' => "7",
            '8' => "8",
            '9' => "9",
            '0' => "0",
            '-' => "-",
            '=' => "=",
            _ => "?",
        },
        KeyCode::Enter => "ret",
        KeyCode::Esc => "esc",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "S-tab",
        KeyCode::Left => "←",
        KeyCode::Right => "→",
        KeyCode::Up => "↑",
        KeyCode::Down => "↓",
        _ => "?",
    }
}
