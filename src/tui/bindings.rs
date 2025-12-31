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
    Home,
    End,
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
    Inspect,
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
}

pub struct Binding {
    pub key: KeyCode,
    pub action: Action,
    pub hint: &'static str,
}

pub fn normal_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('q'), action: Action::Quit, hint: "quit" },
        Binding { key: KeyCode::Esc, action: Action::Quit, hint: "quit" },
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "left" },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "left" },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "down" },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "down" },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "up" },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "up" },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "right" },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "right" },
        Binding { key: KeyCode::Char('H'), action: Action::LeftFast, hint: "left x4" },
        Binding { key: KeyCode::Char('J'), action: Action::DownFast, hint: "down x4" },
        Binding { key: KeyCode::Char('K'), action: Action::UpFast, hint: "up x4" },
        Binding { key: KeyCode::Char('L'), action: Action::RightFast, hint: "right x4" },
        Binding { key: KeyCode::Char('['), action: Action::Home, hint: "home" },
        Binding { key: KeyCode::Char(']'), action: Action::End, hint: "end" },
        Binding { key: KeyCode::Char(' '), action: Action::Place, hint: "place" },
        Binding { key: KeyCode::Char('.'), action: Action::Delete, hint: "delete" },
        Binding { key: KeyCode::Char('o'), action: Action::Rotate, hint: "rotate" },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: "move" },
        Binding { key: KeyCode::Char('u'), action: Action::Edit, hint: "edit" },
        Binding { key: KeyCode::Char('i'), action: Action::Inspect, hint: "inspect" },
        Binding { key: KeyCode::Char('p'), action: Action::TogglePlay, hint: "play" },
        Binding { key: KeyCode::Char('t'), action: Action::TrackEdit, hint: "track" },
        Binding { key: KeyCode::Char('v'), action: Action::Select, hint: "select" },
        Binding { key: KeyCode::Char('w'), action: Action::Save, hint: "save" },
        Binding { key: KeyCode::Char('W'), action: Action::SaveAs, hint: "save as" },
        Binding { key: KeyCode::Char('7'), action: Action::Palette(0), hint: "track" },
        Binding { key: KeyCode::Char('8'), action: Action::Palette(1), hint: "gen" },
        Binding { key: KeyCode::Char('9'), action: Action::Palette(2), hint: "env" },
        Binding { key: KeyCode::Char('0'), action: Action::Palette(3), hint: "fx" },
        Binding { key: KeyCode::Char('-'), action: Action::Palette(4), hint: "math" },
        Binding { key: KeyCode::Char('='), action: Action::Palette(5), hint: "route" },
    ]
}

pub fn move_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "cancel" },
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "left" },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "left" },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "down" },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "down" },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "up" },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "up" },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "right" },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "right" },
        Binding { key: KeyCode::Char('H'), action: Action::LeftFast, hint: "left x4" },
        Binding { key: KeyCode::Char('J'), action: Action::DownFast, hint: "down x4" },
        Binding { key: KeyCode::Char('K'), action: Action::UpFast, hint: "up x4" },
        Binding { key: KeyCode::Char('L'), action: Action::RightFast, hint: "right x4" },
        Binding { key: KeyCode::Char('m'), action: Action::Confirm, hint: "place" },
        Binding { key: KeyCode::Enter, action: Action::Confirm, hint: "place" },
        Binding { key: KeyCode::Char(' '), action: Action::Confirm, hint: "place" },
    ]
}

pub fn edit_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Char('q'), action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Enter, action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Char('u'), action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Tab, action: Action::Down, hint: "next" },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "next" },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "next" },
        Binding { key: KeyCode::BackTab, action: Action::Up, hint: "prev" },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "prev" },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "prev" },
        Binding { key: KeyCode::Char('h'), action: Action::ValueDown, hint: "-" },
        Binding { key: KeyCode::Left, action: Action::ValueDown, hint: "-" },
        Binding { key: KeyCode::Char('l'), action: Action::ValueUp, hint: "+" },
        Binding { key: KeyCode::Right, action: Action::ValueUp, hint: "+" },
        Binding { key: KeyCode::Char('H'), action: Action::ValueDownFast, hint: "-x10" },
        Binding { key: KeyCode::Char('L'), action: Action::ValueUpFast, hint: "+x10" },
        Binding { key: KeyCode::Char(';'), action: Action::TogglePort, hint: "port" },
    ]
}

pub fn env_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Char('q'), action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Char('u'), action: Action::Cancel, hint: "done" },
        Binding { key: KeyCode::Tab, action: Action::Right, hint: "next" },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "next" },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "next" },
        Binding { key: KeyCode::BackTab, action: Action::Left, hint: "prev" },
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "prev" },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "prev" },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: "move" },
        Binding { key: KeyCode::Char(' '), action: Action::AddPoint, hint: "add" },
        Binding { key: KeyCode::Char('.'), action: Action::DeletePoint, hint: "del" },
        Binding { key: KeyCode::Char('c'), action: Action::ToggleCurve, hint: "curve" },
    ]
}

pub fn select_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "cancel" },
        Binding { key: KeyCode::Char('h'), action: Action::Left, hint: "left" },
        Binding { key: KeyCode::Left, action: Action::Left, hint: "left" },
        Binding { key: KeyCode::Char('j'), action: Action::Down, hint: "down" },
        Binding { key: KeyCode::Down, action: Action::Down, hint: "down" },
        Binding { key: KeyCode::Char('k'), action: Action::Up, hint: "up" },
        Binding { key: KeyCode::Up, action: Action::Up, hint: "up" },
        Binding { key: KeyCode::Char('l'), action: Action::Right, hint: "right" },
        Binding { key: KeyCode::Right, action: Action::Right, hint: "right" },
        Binding { key: KeyCode::Char('H'), action: Action::LeftFast, hint: "left x4" },
        Binding { key: KeyCode::Char('J'), action: Action::DownFast, hint: "down x4" },
        Binding { key: KeyCode::Char('K'), action: Action::UpFast, hint: "up x4" },
        Binding { key: KeyCode::Char('L'), action: Action::RightFast, hint: "right x4" },
        Binding { key: KeyCode::Char('m'), action: Action::Move, hint: "move" },
        Binding { key: KeyCode::Enter, action: Action::Move, hint: "move" },
        Binding { key: KeyCode::Char('x'), action: Action::Delete, hint: "delete" },
    ]
}

pub fn quit_confirm_bindings() -> &'static [Binding] {
    &[
        Binding { key: KeyCode::Char('y'), action: Action::Confirm, hint: "yes" },
        Binding { key: KeyCode::Char('Y'), action: Action::Confirm, hint: "yes" },
        Binding { key: KeyCode::Char('n'), action: Action::Cancel, hint: "no" },
        Binding { key: KeyCode::Char('N'), action: Action::Cancel, hint: "no" },
        Binding { key: KeyCode::Esc, action: Action::Cancel, hint: "no" },
    ]
}

pub fn lookup(bindings: &[Binding], key: KeyCode) -> Option<Action> {
    bindings.iter().find(|b| b.key == key).map(|b| b.action)
}

pub fn hints(bindings: &[Binding]) -> Vec<(&'static str, &'static str)> {
    let mut seen = std::collections::HashSet::new();
    bindings.iter()
        .filter(|b| seen.insert(b.action))
        .map(|b| (key_str(b.key), b.hint))
        .collect()
}

pub fn key_str(key: KeyCode) -> &'static str {
    match key {
        KeyCode::Char('h') => "h",
        KeyCode::Char('j') => "j",
        KeyCode::Char('k') => "k",
        KeyCode::Char('l') => "l",
        KeyCode::Char('H') => "H",
        KeyCode::Char('J') => "J",
        KeyCode::Char('K') => "K",
        KeyCode::Char('L') => "L",
        KeyCode::Char('q') => "q",
        KeyCode::Char('u') => "u",
        KeyCode::Char('i') => "i",
        KeyCode::Char('m') => "m",
        KeyCode::Char('o') => "o",
        KeyCode::Char('p') => "p",
        KeyCode::Char('t') => "t",
        KeyCode::Char('v') => "v",
        KeyCode::Char('w') => "w",
        KeyCode::Char('W') => "W",
        KeyCode::Char('c') => "c",
        KeyCode::Char('y') => "y",
        KeyCode::Char('n') => "n",
        KeyCode::Char(' ') => "space",
        KeyCode::Char('.') => ".",
        KeyCode::Char(';') => ";",
        KeyCode::Char('[') => "[",
        KeyCode::Char(']') => "]",
        KeyCode::Char('7') => "7",
        KeyCode::Char('8') => "8",
        KeyCode::Char('9') => "9",
        KeyCode::Char('0') => "0",
        KeyCode::Char('-') => "-",
        KeyCode::Char('=') => "=",
        KeyCode::Enter => "ret",
        KeyCode::Esc => "esc",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "S-tab",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        _ => "?",
    }
}
