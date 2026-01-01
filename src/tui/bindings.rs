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

pub struct Binding {
    pub key: KeyCode,
    pub action: Action,
    pub hint: &'static str,
    pub group: Option<&'static str>,
}

pub fn normal_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "move", group: Some("arrows") },

        Binding { key: KeyCode::Char('.'), action: Action::Delete, hint: "delete", group: None },
        Binding { key: KeyCode::Char('o'), action: Action::Rotate, hint: "rotate", group: None },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: "grab", group: None },
        Binding { key: KeyCode::Char('i'), action: Action::Edit, hint: "info", group: None },
        Binding { key: KeyCode::Char('u'), action: Action::Undo, hint: "undo", group: None },
        Binding { key: KeyCode::Char('U'), action: Action::Redo, hint: "redo", group: None },
        Binding { key: KeyCode::Char('y'), action: Action::Copy, hint: "yank", group: None },
        Binding { key: KeyCode::Char(' '), action: Action::TogglePlay, hint: "play", group: None },
        Binding { key: KeyCode::Char('t'), action: Action::TrackEdit, hint: "track", group: None },
        Binding { key: KeyCode::Char(','), action: Action::Select, hint: "select", group: None },
        Binding { key: KeyCode::Char('w'), action: Action::Save, hint: "save", group: None },
        Binding { key: KeyCode::Char('W'), action: Action::SaveAs, hint: "save as", group: None },
        Binding { key: KeyCode::Char('s'), action: Action::TrackSettings, hint: "settings", group: None },
        Binding { key: KeyCode::Char('Q'), action: Action::Quit, hint: "quit", group: None },
        Binding { key: KeyCode::Char('p'), action: Action::EditSubpatch, hint: "enter sub", group: None },
        Binding { key: KeyCode::Esc, action: Action::ExitSubpatch, hint: "exit sub", group: None },
        Binding { key: KeyCode::Char('n'), action: Action::Palette(0), hint: "menu", group: None },
        Binding { key: KeyCode::Char('7'), action: Action::Palette(0), hint: "track", group: None },
        Binding { key: KeyCode::Char('8'), action: Action::Palette(1), hint: "gen", group: None },
        Binding { key: KeyCode::Char('9'), action: Action::Palette(2), hint: "env", group: None },
        Binding { key: KeyCode::Char('0'), action: Action::Palette(3), hint: "fx", group: None },
        Binding { key: KeyCode::Char('-'), action: Action::Palette(4), hint: "math", group: None },
        Binding { key: KeyCode::Char('='), action: Action::Palette(5), hint: "route", group: None },
        Binding { key: KeyCode::Char('`'), action: Action::Palette(6), hint: "sub", group: None },
        Binding { key: KeyCode::Char('v'), action: Action::ToggleMeters, hint: "meters", group: None },
    ]
}

pub fn palette_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: "place", group: Some("confirm") },
        Binding { key: KeyCode::Char('/'), action: Action::Search, hint: "search", group: None },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "cancel", group: Some("cancel") },
        Binding { key: KeyCode::Char('q'), action: Action::Cancel, hint: "cancel", group: Some("cancel") },
        Binding { key: KeyCode::Char(' '), action: Action::Cancel, hint: "cancel", group: Some("cancel") },
    ]
}

pub fn move_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Char('m'), action: Action::Confirm, hint: "place", group: Some("confirm") },
        Binding { key: KeyCode::Char('y'), action: Action::Confirm, hint: "place", group: Some("confirm") },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: "place", group: Some("confirm") },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "cancel", group: None },
    ]
}

pub fn edit_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "param", group: Some("jk") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "param", group: Some("jk") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "param", group: Some("ud") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "param", group: Some("ud") },
        Binding { key: KeyCode::Char('h'), action: Action::ValueDown, hint: "adjust", group: Some("hl") },
        Binding { key: KeyCode::Char('l'), action: Action::ValueUp, hint: "adjust", group: Some("hl") },
        Binding { key: KeyCode::Left, action: Action::ValueDown, hint: "adjust", group: Some("lr") },
        Binding { key: KeyCode::Right, action: Action::ValueUp, hint: "adjust", group: Some("lr") },
        Binding { key: KeyCode::Char(';'), action: Action::TogglePort, hint: "port", group: None },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Char('i'), action: Action::Cancel, hint: "done", group: Some("done") },
    ]
}

pub fn env_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "point", group: Some("hl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "point", group: Some("hl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "point", group: Some("arrows") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "point", group: Some("arrows") },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: "move", group: None },
        Binding { key: KeyCode::Char(' '), action: Action::AddPoint, hint: "add", group: None },
        Binding { key: KeyCode::Char('.'), action: Action::DeletePoint, hint: "delete", group: None },
        Binding { key: KeyCode::Char('c'), action: Action::ToggleCurve, hint: "curve", group: None },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Char('i'), action: Action::Cancel, hint: "done", group: Some("done") },
    ]
}

pub fn env_move_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Char('m'), action: Action::Confirm, hint: "done", group: Some("done") },
    ]
}

pub fn select_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "move", group: Some("hjkl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "move", group: Some("arrows") },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: "grab", group: Some("grab") },
        Binding { key: KeyCode::Enter, action: Action::Move, hint: "grab", group: Some("grab") },
        Binding { key: KeyCode::Char('.'), action: Action::Delete, hint: "delete", group: Some("del") },
        Binding { key: KeyCode::Char('x'), action: Action::Delete, hint: "delete", group: Some("del") },
        Binding { key: KeyCode::Char('y'), action: Action::Copy, hint: "yank", group: None },
        Binding { key: KeyCode::Char('p'), action: Action::MakeSubpatch, hint: "subpatch", group: None },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "cancel", group: Some("cancel") },
        Binding { key: KeyCode::Char(','), action: Action::Cancel, hint: "cancel", group: Some("cancel") },
    ]
}

pub fn quit_confirm_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('y'), action: Action::Confirm, hint: "yes", group: Some("yes") },
        Binding { key: KeyCode::Char('Y'), action: Action::Confirm, hint: "yes", group: Some("yes") },
        Binding { key: KeyCode::Char('n'), action: Action::Cancel, hint: "no", group: Some("no") },
        Binding { key: KeyCode::Char('N'), action: Action::Cancel, hint: "no", group: Some("no") },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "no", group: Some("no") },
    ]
}

pub fn settings_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "param", group: Some("jk") },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "param", group: Some("jk") },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "param", group: Some("ud") },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "param", group: Some("ud") },
        Binding { key: KeyCode::Char('h'), action: Action::ValueDown, hint: "adjust", group: Some("hl") },
        Binding { key: KeyCode::Char('l'), action: Action::ValueUp, hint: "adjust", group: Some("hl") },
        Binding { key: KeyCode::Left, action: Action::ValueDown, hint: "adjust", group: Some("lr") },
        Binding { key: KeyCode::Right, action: Action::ValueUp, hint: "adjust", group: Some("lr") },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Char('s'), action: Action::Cancel, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Enter, action: Action::Cancel, hint: "done", group: Some("done") },
    ]
}

pub fn text_input_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: "confirm", group: None },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "cancel", group: None },
    ]
}

pub fn probe_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "param", group: Some("hl") },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "param", group: Some("hl") },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "param", group: Some("lr") },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "param", group: Some("lr") },
        Binding { key: KeyCode::Char('j'), action: Action::ValueDown, hint: "adjust", group: Some("jk") },
        Binding { key: KeyCode::Char('k'), action: Action::ValueUp, hint: "adjust", group: Some("jk") },
        Binding { key: KeyCode::Down, action: Action::ValueDown, hint: "adjust", group: Some("ud") },
        Binding { key: KeyCode::Up, action: Action::ValueUp, hint: "adjust", group: Some("ud") },
        Binding { key: KeyCode::Char('r'), action: Action::Delete, hint: "reset", group: None },
        Binding { key: KeyCode::Char('c'), action: Action::ToggleCurve, hint: "cycle", group: None },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done", group: Some("done") },
        Binding { key: KeyCode::Char('i'), action: Action::Cancel, hint: "done", group: Some("done") },
    ]
}

pub fn lookup(bindings: &[Binding], key: KeyCode) -> Option<Action> {
    bindings.iter().find(|b| b.key == key).map(|b| b.action)
}

pub fn hints(bindings: &[Binding]) -> Vec<(String, &'static str)> {
    let mut result: Vec<(String, &'static str, Option<&'static str>)> = Vec::new();
    for b in bindings {
        if let Some(group) = b.group {
            if let Some(existing) = result.iter_mut().find(|(_, _, g)| *g == Some(group)) {
                existing.0.push('/');
                existing.0.push_str(key_str(b.key));
                continue;
            }
        }
        if result.iter().any(|(_, h, g)| *h == b.hint && g.is_none() && b.group.is_none()) {
            continue;
        }
        result.push((key_str(b.key).to_string(), b.hint, b.group));
    }
    result.into_iter().map(|(k, h, _)| (k, h)).collect()
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
