use super::grid::{Cell, GridPos};
use super::module::{Module, ModuleCategory, ModuleId, ModuleKind, ParamKind};
use super::patch::Patch;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

const CELL_WIDTH: u16 = 5;
const CELL_HEIGHT: u16 = 3;

fn set_cell(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char(ch).set_style(style);
    }
}

fn set_str(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    for (i, ch) in s.chars().enumerate() {
        set_cell(buf, x + i as u16, y, ch, style);
    }
}

pub struct GridWidget<'a> {
    patch: &'a Patch,
    cursor: GridPos,
    moving: Option<ModuleId>,
}

impl<'a> GridWidget<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self {
            patch,
            cursor: GridPos::new(0, 0),
            moving: None,
        }
    }

    pub fn cursor(mut self, pos: GridPos) -> Self {
        self.cursor = pos;
        self
    }

    pub fn moving(mut self, id: Option<ModuleId>) -> Self {
        self.moving = id;
        self
    }

    fn screen_pos(&self, grid_pos: GridPos, area: Rect) -> (u16, u16) {
        (area.x + grid_pos.x * CELL_WIDTH, area.y + grid_pos.y * CELL_HEIGHT)
    }

    fn render_empty(&self, buf: &mut Buffer, sx: u16, sy: u16, is_cursor: bool) {
        let dot_style = Style::default().fg(Color::Rgb(50, 50, 50));
        let bg = if is_cursor { Color::Rgb(40, 40, 50) } else { Color::Reset };
        
        for dy in 0..CELL_HEIGHT {
            for dx in 0..CELL_WIDTH {
                let ch = if dx == CELL_WIDTH / 2 && dy == CELL_HEIGHT / 2 { '·' } else { ' ' };
                let style = if is_cursor {
                    dot_style.bg(bg)
                } else {
                    dot_style
                };
                set_cell(buf, sx + dx, sy + dy, ch, style);
            }
        }
    }

    fn render_channel(&self, buf: &mut Buffer, sx: u16, sy: u16, is_cursor: bool, color: Color) {
        let style = if is_cursor {
            Style::default().fg(Color::White).bg(color)
        } else {
            Style::default().fg(color)
        };
        
        let cx = sx + CELL_WIDTH / 2;
        for dy in 0..CELL_HEIGHT {
            set_cell(buf, cx, sy + dy, '│', style);
        }
    }

    fn render_module(&self, buf: &mut Buffer, sx: u16, sy: u16, module: &Module, local_x: u8, local_y: u8, is_cursor: bool, is_moving: bool) {
        let kind = module.kind;
        let color = kind.color();
        let width = module.width();
        let height = module.height();
        let is_single_height = height == 1;
        let is_single_width = width == 1;
        
        let (border_style, text_style, inner_style) = if is_moving && is_cursor {
            (
                Style::default().fg(Color::White).bg(Color::Rgb(120, 120, 160)),
                Style::default().fg(Color::White).bg(Color::Rgb(120, 120, 160)).add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Rgb(120, 120, 160)),
            )
        } else if is_moving {
            (
                Style::default().fg(Color::White).bg(Color::Rgb(80, 80, 100)),
                Style::default().fg(Color::White).bg(Color::Rgb(80, 80, 100)).add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Rgb(80, 80, 100)),
            )
        } else if is_cursor {
            let cursor_bg = if color == Color::White { Color::Rgb(100, 100, 120) } else { color };
            (
                Style::default().fg(Color::Black).bg(cursor_bg),
                Style::default().fg(Color::Black).bg(cursor_bg).add_modifier(Modifier::BOLD),
                Style::default().bg(cursor_bg),
            )
        } else {
            (
                Style::default().fg(color),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
                Style::default(),
            )
        };

        let port_style = if is_moving && is_cursor {
            Style::default().fg(Color::White).bg(Color::Rgb(120, 120, 160))
        } else if is_moving {
            Style::default().fg(Color::White).bg(Color::Rgb(80, 80, 100))
        } else if is_cursor {
            let cursor_bg = if color == Color::White { Color::Rgb(100, 100, 120) } else { color };
            Style::default().fg(Color::White).bg(cursor_bg)
        } else {
            Style::default().fg(Color::Gray)
        };

        if is_cursor || is_moving {
            for dy in 0..CELL_HEIGHT {
                for dx in 0..CELL_WIDTH {
                    set_cell(buf, sx + dx, sy + dy, ' ', inner_style);
                }
            }
        }

        let is_top = local_y == 0;
        let is_bottom = local_y == height - 1;
        let is_left = local_x == 0;
        let is_right = local_x == width - 1;

        let (tl, tr, bl, br) = if is_single_width && is_single_height {
            ('╭', '╮', '╰', '╯')
        } else {
            ('┌', '┐', '└', '┘')
        };

        let top_str = if is_top {
            if is_left && is_right { format!("{}───{}", tl, tr) }
            else if is_left { format!("{}────", tl) }
            else if is_right { format!("────{}", tr) }
            else { "─────".to_string() }
        } else {
            if is_left && is_right { "│   │".to_string() }
            else if is_left { "│    ".to_string() }
            else if is_right { "    │".to_string() }
            else { "     ".to_string() }
        };

        let mid_str = if is_left && is_right { "│   │".to_string() }
            else if is_left { "│    ".to_string() }
            else if is_right { "    │".to_string() }
            else { "     ".to_string() };

        let bot_str = if is_bottom {
            if is_left && is_right { format!("{}───{}", bl, br) }
            else if is_left { format!("{}────", bl) }
            else if is_right { format!("────{}", br) }
            else { "─────".to_string() }
        } else {
            mid_str.clone()
        };

        set_str(buf, sx, sy, &top_str, border_style);
        set_str(buf, sx, sy + 1, &mid_str, border_style);
        set_str(buf, sx, sy + 2, &bot_str, border_style);

        if is_top && is_left {
            let name = kind.short_name();
            let name_x = sx + 1;
            for (i, ch) in name.chars().take(3).enumerate() {
                set_cell(buf, name_x + i as u16, sy + 1, ch, text_style);
            }
        }

        let cx = sx + CELL_WIDTH / 2;
        let cy = sy + CELL_HEIGHT / 2;

        let local_pos = match module.orientation {
            super::module::Orientation::Horizontal => local_y as usize,
            super::module::Orientation::Vertical => local_x as usize,
        };

        if !kind.is_routing() {
            let defs = kind.param_defs();
            let port_defs: Vec<_> = defs.iter().enumerate()
                .filter(|(_, d)| !matches!(d.kind, ParamKind::Enum { .. }))
                .collect();
            
            if let Some((param_idx, def)) = port_defs.get(local_pos) {
                let port_char = match def.kind {
                    ParamKind::Input => '●',
                    ParamKind::Float { .. } => {
                        if module.params.is_connected(*param_idx) { '●' } else { '✕' }
                    }
                    ParamKind::Enum { .. } => unreachable!(),
                };
                let label = def.name.chars().next().unwrap_or(' ');

                if module.has_input_top() {
                    set_cell(buf, cx - 1, sy, label, port_style);
                    set_cell(buf, cx, sy, port_char, port_style);
                }
                if module.has_input_left() {
                    set_cell(buf, sx, cy - 1, label, port_style);
                    set_cell(buf, sx, cy, port_char, port_style);
                }
            }
        }

        if kind.is_routing() {
            if is_top && is_left && module.has_input_top() {
                set_cell(buf, cx, sy, '●', port_style);
            }
            if is_left && module.has_input_left() {
                set_cell(buf, sx, cy, '●', port_style);
            }
        }

        if is_bottom && is_left && module.has_output_bottom() {
            set_cell(buf, cx, sy + CELL_HEIGHT - 1, '○', port_style);
        }
        if is_top && is_right && module.has_output_right() {
            set_cell(buf, sx + CELL_WIDTH - 1, cy, '○', port_style);
        }
    }

    fn render_cell(&self, buf: &mut Buffer, area: Rect, grid_pos: GridPos) {
        let (sx, sy) = self.screen_pos(grid_pos, area);
        
        if sx + CELL_WIDTH > area.x + area.width || sy + CELL_HEIGHT > area.y + area.height {
            return;
        }

        let is_cursor = grid_pos == self.cursor;
        let cell = self.patch.grid().get(grid_pos);

        match cell {
            Cell::Empty => {
                self.render_empty(buf, sx, sy, is_cursor);
            }
            Cell::Module { id, local_x, local_y } => {
                if let Some(module) = self.patch.module(id) {
                    let is_moving = self.moving == Some(id);
                    self.render_module(buf, sx, sy, module, local_x, local_y, is_cursor, is_moving);
                }
            }
            Cell::ChannelV { color } => {
                self.render_channel(buf, sx, sy, is_cursor, color);
            }
            Cell::ChannelH { color } => {
                let style = if is_cursor {
                    Style::default().fg(Color::White).bg(color)
                } else {
                    Style::default().fg(color)
                };
                let cy = sy + CELL_HEIGHT / 2;
                for dx in 0..CELL_WIDTH {
                    set_cell(buf, sx + dx, cy, '─', style);
                }
            }
            Cell::ChannelCorner { color, down_right } => {
                let style = if is_cursor {
                    Style::default().fg(Color::White).bg(color)
                } else {
                    Style::default().fg(color)
                };
                let cx = sx + CELL_WIDTH / 2;
                let cy = sy + CELL_HEIGHT / 2;
                
                if down_right {
                    for dx in 0..=CELL_WIDTH/2 {
                        set_cell(buf, sx + dx, cy, '─', style);
                    }
                    set_cell(buf, cx, cy, '┐', style);
                    for dy in (CELL_HEIGHT/2 + 1)..CELL_HEIGHT {
                        set_cell(buf, cx, sy + dy, '│', style);
                    }
                } else {
                    for dy in 0..=CELL_HEIGHT/2 {
                        set_cell(buf, cx, sy + dy, '│', style);
                    }
                    set_cell(buf, cx, cy, '└', style);
                    for dx in (CELL_WIDTH/2 + 1)..CELL_WIDTH {
                        set_cell(buf, sx + dx, cy, '─', style);
                    }
                }
            }
        }
    }
}

impl Widget for GridWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let grid = self.patch.grid();
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                self.render_cell(buf, area, GridPos::new(x, y));
            }
        }
    }
}

pub struct PaletteWidget {
    selected_category: usize,
    selected_module: usize,
}

impl PaletteWidget {
    pub fn new() -> Self {
        Self {
            selected_category: 0,
            selected_module: 0,
        }
    }

    pub fn selected_category(mut self, idx: usize) -> Self {
        self.selected_category = idx;
        self
    }

    pub fn selected_module(mut self, idx: usize) -> Self {
        self.selected_module = idx;
        self
    }
}

impl Widget for PaletteWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let categories = ModuleCategory::all();
        let mut y = area.y;

        for (cat_idx, cat) in categories.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }

            let is_selected_cat = cat_idx == self.selected_category;
            let cat_style = if is_selected_cat {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            set_str(buf, area.x, y, &format!(" {} ", cat.name()), cat_style);
            y += 1;

            if is_selected_cat {
                for (mod_idx, kind) in ModuleKind::by_category(*cat).iter().enumerate() {
                    if y >= area.y + area.height {
                        break;
                    }
                    let is_sel = mod_idx == self.selected_module;
                    let style = if is_sel {
                        Style::default().fg(Color::Black).bg(kind.color())
                    } else {
                        Style::default().fg(kind.color())
                    };
                    set_str(buf, area.x + 1, y, &format!(" {} ", kind.name()), style);
                    y += 1;
                }
            }
        }
    }
}

pub struct StatusWidget<'a> {
    cursor: GridPos,
    mode: &'a str,
    message: Option<&'a str>,
    playing: bool,
}

impl<'a> StatusWidget<'a> {
    pub fn new(cursor: GridPos, mode: &'a str) -> Self {
        Self { cursor, mode, message: None, playing: false }
    }

    pub fn message(mut self, msg: &'a str) -> Self {
        self.message = Some(msg);
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }
}

impl Widget for StatusWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let pos_style = Style::default().fg(Color::DarkGray);
        let msg_style = Style::default().fg(Color::White);

        set_str(buf, area.x, area.y, &format!("[{}]", self.mode), mode_style);
        
        let pos_str = format!(" ({},{})", self.cursor.x, self.cursor.y);
        set_str(buf, area.x + self.mode.len() as u16 + 2, area.y, &pos_str, pos_style);

        let play_x = area.x + self.mode.len() as u16 + 2 + pos_str.len() as u16 + 1;
        if self.playing {
            let play_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
            set_str(buf, play_x, area.y, "▶", play_style);
        }

        if let Some(msg) = self.message {
            let x = area.x + area.width.saturating_sub(msg.len() as u16 + 1);
            set_str(buf, x, area.y, msg, msg_style);
        }
    }
}

pub struct HelpWidget;

impl Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let keys = [
            ("hjkl", "move"),
            ("Space", "add"),
            ("m", "grab"),
            ("o", "rotate"),
            ("u", "edit"),
            (".", "delete"),
            ("t", "track"),
            ("p", "play"),
            ("q", "quit"),
        ];

        let key_style = Style::default().fg(Color::Cyan);
        let desc_style = Style::default().fg(Color::DarkGray);

        for (i, (key, desc)) in keys.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            set_str(buf, area.x, y, key, key_style);
            set_str(buf, area.x + key.len() as u16 + 1, y, desc, desc_style);
        }
    }
}

pub struct EditWidget<'a> {
    module: &'a Module,
    selected_param: usize,
}

impl<'a> EditWidget<'a> {
    pub fn new(module: &'a Module, selected_param: usize) -> Self {
        Self { module, selected_param }
    }
}

impl Widget for EditWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let defs = self.module.kind.param_defs();
        let color = self.module.kind.color();

        let title_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_style = Style::default().fg(Color::Black).bg(color);

        let title = self.module.kind.name();
        set_str(buf, area.x, area.y, title, title_style);

        let mut y = area.y + 2;

        for (i, def) in defs.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }
            let val = self.module.params.floats[i];
            let is_selected = i == self.selected_param;
            let is_connected = self.module.params.is_connected(i);

            let style = if is_selected { selected_style } else { label_style };
            
            let port_str = match def.kind {
                ParamKind::Input => "● ",
                ParamKind::Float { .. } => if is_connected { "● " } else { "✕ " },
                ParamKind::Enum { .. } => "  ",
            };
            set_str(buf, area.x, y, port_str, style);
            set_str(buf, area.x + 2, y, def.name, style);

            let val_x = area.x + 8;
            let v_style = if is_selected { selected_style } else { value_style };

            match &def.kind {
                ParamKind::Input => {
                    set_str(buf, val_x, y, "(input)", label_style);
                }
                ParamKind::Float { step, .. } => {
                    let val_str = if *step >= 1.0 {
                        format!("{:.0}", val)
                    } else if *step >= 0.1 {
                        format!("{:.1}", val)
                    } else {
                        format!("{:.2}", val)
                    };
                    set_str(buf, val_x, y, &val_str, v_style);
                    if is_selected {
                        set_str(buf, val_x + val_str.len() as u16 + 1, y, "<hl> ;", label_style);
                    }
                }
                ParamKind::Enum { options } => {
                    let idx = (val as usize).min(options.len().saturating_sub(1));
                    let val_str = options[idx];
                    set_str(buf, val_x, y, val_str, v_style);
                    if is_selected {
                        set_str(buf, val_x + val_str.len() as u16 + 1, y, "<hl>", label_style);
                    }
                }
            }

            y += 1;
        }
    }
}
