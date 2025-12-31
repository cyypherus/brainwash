use super::grid::{Cell, GridPos};
use super::module::{Module, ModuleCategory, ModuleId, ModuleKind, ModuleParams, ParamKind};
use super::patch::Patch;
use crate::envelopes::{Envelope, EnvelopePoint, PointType};
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
    probe_values: &'a [f32],
    selection: Option<(GridPos, GridPos)>,
}

impl<'a> GridWidget<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self {
            patch,
            cursor: GridPos::new(0, 0),
            moving: None,
            probe_values: &[],
            selection: None,
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

    pub fn selection(mut self, sel: Option<(GridPos, GridPos)>) -> Self {
        self.selection = sel;
        self
    }

    pub fn probe_values(mut self, values: &'a [f32]) -> Self {
        self.probe_values = values;
        self
    }

    fn screen_pos(&self, grid_pos: GridPos, viewport_origin: GridPos, area: Rect) -> (u16, u16) {
        let rel_x = grid_pos.x as i32 - viewport_origin.x as i32;
        let rel_y = grid_pos.y as i32 - viewport_origin.y as i32;
        (
            (area.x as i32 + rel_x * CELL_WIDTH as i32) as u16,
            (area.y as i32 + rel_y * CELL_HEIGHT as i32) as u16,
        )
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

    fn render_module(&self, buf: &mut Buffer, sx: u16, sy: u16, module: &Module, local_x: u8, local_y: u8, is_cursor: bool, is_moving: bool, probe_value: Option<f32>) {
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
            if let Some(val) = probe_value {
                let val_str = format!("{:.1}", val);
                let name_x = sx + 1;
                for (i, ch) in val_str.chars().take(3).enumerate() {
                    set_cell(buf, name_x + i as u16, sy + 1, ch, text_style);
                }
            } else {
                let name = module.display_name();
                let name_x = sx + 1;
                for (i, ch) in name.chars().take(3).enumerate() {
                    set_cell(buf, name_x + i as u16, sy + 1, ch, text_style);
                }
            }
        }

        let cx = sx + CELL_WIDTH / 2;
        let cy = sy + CELL_HEIGHT / 2;

        let port_pos = match module.orientation {
            super::module::Orientation::Horizontal => local_y as usize,
            super::module::Orientation::Vertical => local_x as usize,
        };

        if !kind.is_routing() {
            let defs = kind.param_defs();
            let port_params: Vec<_> = defs.iter().enumerate()
                .filter(|(_, d)| !matches!(d.kind, ParamKind::Enum))
                .collect();
            
            if let Some(&(param_idx, def)) = port_params.get(port_pos) {
                let port_char = match def.kind {
                    ParamKind::Input => '●',
                    ParamKind::Float { .. } => {
                        if module.params.is_connected(param_idx) { '●' } else { '✕' }
                    }
                    ParamKind::Enum => unreachable!(),
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

    fn is_in_selection(&self, pos: GridPos) -> bool {
        if let Some((a, b)) = self.selection {
            let min_x = a.x.min(b.x);
            let max_x = a.x.max(b.x);
            let min_y = a.y.min(b.y);
            let max_y = a.y.max(b.y);
            pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y
        } else {
            false
        }
    }

    fn render_cell(&self, buf: &mut Buffer, area: Rect, grid_pos: GridPos, viewport_origin: GridPos) {
        let (sx, sy) = self.screen_pos(grid_pos, viewport_origin, area);
        
        if sx < area.x || sy < area.y || sx + CELL_WIDTH > area.x + area.width || sy + CELL_HEIGHT > area.y + area.height {
            return;
        }

        let is_cursor = grid_pos == self.cursor;
        let is_selected = self.is_in_selection(grid_pos);
        let cell = self.patch.grid().get(grid_pos);
        
        if is_selected && !is_cursor {
            let sel_style = Style::default().bg(Color::Rgb(60, 60, 80));
            for dy in 0..CELL_HEIGHT {
                for dx in 0..CELL_WIDTH {
                    set_cell(buf, sx + dx, sy + dy, ' ', sel_style);
                }
            }
        }

        match cell {
            Cell::Empty => {
                self.render_empty(buf, sx, sy, is_cursor);
            }
            Cell::Module { id, local_x, local_y } => {
                if let Some(module) = self.patch.module(id) {
                    let is_moving = self.moving == Some(id);
                    let probe_value = if module.kind == ModuleKind::Probe {
                        let probe_idx = self.patch.all_modules()
                            .filter(|m| m.kind == ModuleKind::Probe)
                            .position(|m| m.id == id);
                        probe_idx.and_then(|i| self.probe_values.get(i).copied())
                    } else {
                        None
                    };
                    self.render_module(buf, sx, sy, module, local_x, local_y, is_cursor, is_moving, probe_value);
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
        
        let gutter_left: u16 = 2;
        let gutter_top: u16 = 1;
        
        let grid_area = Rect::new(
            area.x + gutter_left,
            area.y + gutter_top,
            area.width.saturating_sub(gutter_left),
            area.height.saturating_sub(gutter_top),
        );
        
        let visible_cols = grid_area.width / CELL_WIDTH;
        let visible_rows = grid_area.height / CELL_HEIGHT;
        
        let half_cols = visible_cols / 2;
        let half_rows = visible_rows / 2;
        
        let origin_x = if self.cursor.x < half_cols {
            0
        } else if self.cursor.x + half_cols >= grid.width() {
            grid.width().saturating_sub(visible_cols)
        } else {
            self.cursor.x - half_cols
        };
        
        let origin_y = if self.cursor.y < half_rows {
            0
        } else if self.cursor.y + half_rows >= grid.height() {
            grid.height().saturating_sub(visible_rows)
        } else {
            self.cursor.y - half_rows
        };
        
        let viewport_origin = GridPos::new(origin_x, origin_y);
        
        for vy in 0..visible_rows {
            for vx in 0..visible_cols {
                let gx = origin_x + vx;
                let gy = origin_y + vy;
                if gx < grid.width() && gy < grid.height() {
                    self.render_cell(buf, grid_area, GridPos::new(gx, gy), viewport_origin);
                }
            }
        }
        
        let num_style = Style::default().fg(Color::Rgb(60, 60, 70));
        
        for vx in 0..visible_cols {
            let gx = origin_x + vx;
            if gx < grid.width() {
                let sx = grid_area.x + vx * CELL_WIDTH + CELL_WIDTH / 2;
                let label = format!("{}", gx);
                for (i, ch) in label.chars().enumerate() {
                    if sx + i as u16 <= area.x + area.width {
                        set_cell(buf, sx + i as u16, area.y, ch, num_style);
                    }
                }
            }
        }
        
        for vy in 0..visible_rows {
            let gy = origin_y + vy;
            if gy < grid.height() {
                let sy = grid_area.y + vy * CELL_HEIGHT + CELL_HEIGHT / 2;
                let label = format!("{:>2}", gy);
                for (i, ch) in label.chars().enumerate() {
                    set_cell(buf, area.x + i as u16, sy, ch, num_style);
                }
            }
        }
        
        let indicator_style = Style::default().fg(Color::Rgb(80, 80, 100));
        
        if origin_x > 0 {
            for vy in 0..visible_rows {
                let y = grid_area.y + vy * CELL_HEIGHT + CELL_HEIGHT / 2;
                if let Some(cell) = buf.cell_mut((grid_area.x, y)) {
                    cell.set_char('◂').set_style(indicator_style);
                }
            }
        }
        
        if origin_x + visible_cols < grid.width() {
            let x = grid_area.x + visible_cols * CELL_WIDTH - 1;
            for vy in 0..visible_rows {
                let y = grid_area.y + vy * CELL_HEIGHT + CELL_HEIGHT / 2;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('▸').set_style(indicator_style);
                }
            }
        }
        
        if origin_y > 0 {
            for vx in 0..visible_cols {
                let x = grid_area.x + vx * CELL_WIDTH + CELL_WIDTH / 2;
                if let Some(cell) = buf.cell_mut((x, grid_area.y)) {
                    cell.set_char('▴').set_style(indicator_style);
                }
            }
        }
        
        if origin_y + visible_rows < grid.height() {
            let y = grid_area.y + visible_rows * CELL_HEIGHT - 1;
            for vx in 0..visible_cols {
                let x = grid_area.x + vx * CELL_WIDTH + CELL_WIDTH / 2;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('▾').set_style(indicator_style);
                }
            }
        }
    }
}

pub struct PaletteWidget<'a> {
    selected_category: usize,
    selected_module: usize,
    filter: &'a str,
    filtered_modules: Vec<ModuleKind>,
    filter_selection: usize,
    searching: bool,
}

impl<'a> PaletteWidget<'a> {
    pub fn new() -> Self {
        Self {
            selected_category: 0,
            selected_module: 0,
            filter: "",
            filtered_modules: Vec::new(),
            filter_selection: 0,
            searching: false,
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

    pub fn filter(mut self, filter: &'a str, modules: Vec<ModuleKind>, selection: usize, searching: bool) -> Self {
        self.filter = filter;
        self.filtered_modules = modules;
        self.filter_selection = selection;
        self.searching = searching;
        self
    }
}

impl Widget for PaletteWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hint_style = Style::default().fg(Color::DarkGray);
        let hint_y = area.y + area.height.saturating_sub(1);

        if self.searching {
            let mut y = area.y;
            let filter_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            set_str(buf, area.x, y, &format!(" /{} ", self.filter), filter_style);
            y += 1;

            for (idx, kind) in self.filtered_modules.iter().enumerate() {
                if y >= area.y + area.height.saturating_sub(1) {
                    break;
                }
                let is_sel = idx == self.filter_selection;
                let style = if is_sel {
                    Style::default().fg(Color::Black).bg(kind.color())
                } else {
                    Style::default().fg(kind.color())
                };
                set_str(buf, area.x + 1, y, &format!(" {} ", kind.name()), style);
                y += 1;
            }

            set_str(buf, area.x, hint_y, " esc clear", hint_style);
            return;
        }

        let categories = ModuleCategory::all();
        let mut y = area.y;

        for (cat_idx, cat) in categories.iter().enumerate() {
            if y >= area.y + area.height.saturating_sub(1) {
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
                    if y >= area.y + area.height.saturating_sub(1) {
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

        set_str(buf, area.x, hint_y, " hjkl / search", hint_style);
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
        let groups: &[&[(&str, &str)]] = &[
            &[
                ("hjkl", "move"),
                ("Space", "add"),
                (".", "delete"),
                ("m", "move"),
                ("u", "edit"),
                ("o", "rotate"),
                ("v", "select"),
            ],
            &[
                ("t", "track"),
                ("p", "play"),
            ],
            &[
                ("s/S", "save"),
                ("q", "quit"),
            ],
        ];

        let key_style = Style::default().fg(Color::Cyan);
        let desc_style = Style::default().fg(Color::DarkGray);

        let mut y = area.y;
        for (group_idx, group) in groups.iter().enumerate() {
            if group_idx > 0 {
                y += 1;
            }
            for (key, desc) in *group {
                if y >= area.y + area.height {
                    return;
                }
                set_str(buf, area.x, y, key, key_style);
                set_str(buf, area.x + key.len() as u16 + 1, y, desc, desc_style);
                y += 1;
            }
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
            let is_selected = i == self.selected_param;
            let is_connected = self.module.params.is_connected(i);

            let style = if is_selected { selected_style } else { label_style };
            
            let port_str = match def.kind {
                ParamKind::Input => "● ",
                ParamKind::Float { .. } => if is_connected { "● " } else { "✕ " },
                ParamKind::Enum => "  ",
            };
            set_str(buf, area.x, y, port_str, style);
            set_str(buf, area.x + 2, y, def.name, style);

            let val_x = area.x + 8;
            let v_style = if is_selected { selected_style } else { value_style };

            match &def.kind {
                ParamKind::Input => {
                    set_str(buf, val_x, y, "(input)", label_style);
                }
                ParamKind::Enum => {
                    let val_str = self.module.params.enum_display().unwrap_or("?");
                    set_str(buf, val_x, y, val_str, v_style);
                    if is_selected {
                        set_str(buf, val_x + val_str.len() as u16 + 1, y, "<hl>", label_style);
                    }
                }
                ParamKind::Float { step, .. } => {
                    let val = self.module.params.get_float(i).unwrap_or(0.0);
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
            }

            y += 1;
        }
    }
}

pub struct AdsrWidget<'a> {
    module: &'a Module,
    selected_param: usize,
}

impl<'a> AdsrWidget<'a> {
    pub fn new(module: &'a Module, selected_param: usize) -> Self {
        Self { module, selected_param }
    }

    fn draw_curve(&self, buf: &mut Buffer, area: Rect, attack_ratio: f32, sustain: f32) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let w = area.width as f32;
        let h = (area.height - 1) as f32;

        let attack_x = (attack_ratio * w) as u16;

        let curve_style = Style::default().fg(Color::Rgb(255, 200, 100));
        let selected_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);

        for x in 0..area.width {
            let t = x as f32 / w;
            let y_val = if t < attack_ratio {
                if attack_ratio == 0.0 { sustain } else { t / attack_ratio }
            } else {
                let decay_progress = if attack_ratio >= 1.0 { 0.0 } else { (t - attack_ratio) / (1.0 - attack_ratio) };
                1.0 - decay_progress * (1.0 - sustain)
            };

            let y = ((1.0 - y_val) * h) as u16;
            let screen_x = area.x + x;
            let screen_y = area.y + y;

            if screen_y < area.y + area.height {
                set_cell(buf, screen_x, screen_y, '█', curve_style);
            }
        }

        let labels = ["A", "S"];
        let positions = [
            attack_x / 2,
            attack_x + (area.width - attack_x) / 2,
        ];

        for (i, (label, pos)) in labels.iter().zip(positions.iter()).enumerate() {
            let style = if i == self.selected_param { selected_style } else { label_style };
            let x = area.x + (*pos).min(area.width.saturating_sub(1));
            set_str(buf, x, area.y + area.height - 1, label, style);
        }
    }
}

impl<'a> Widget for AdsrWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (attack_ratio, sustain) = match &self.module.params {
            ModuleParams::Adsr { attack_ratio, sustain, .. } => (*attack_ratio, *sustain),
            _ => (0.5, 0.7),
        };

        let param_area = Rect::new(area.x, area.y, area.width, 3);
        let curve_area = Rect::new(area.x, area.y + 3, area.width, area.height.saturating_sub(3));

        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

        let params = [
            ("Atk Ratio", attack_ratio, ""),
            ("Sustain", sustain, ""),
        ];

        for (i, (name, val, suffix)) in params.iter().enumerate() {
            let y = param_area.y + i as u16;
            let is_selected = i == self.selected_param;
            let style = if is_selected { selected_style } else { label_style };
            let v_style = if is_selected { selected_style } else { value_style };

            set_str(buf, param_area.x, y, name, style);
            let val_str = format!("{:.2}{}", val, suffix);
            set_str(buf, param_area.x + 10, y, &val_str, v_style);

            if is_selected {
                set_str(buf, param_area.x + 10 + val_str.len() as u16 + 1, y, "<hl>", label_style);
            }
        }

        self.draw_curve(buf, curve_area, attack_ratio, sustain);
    }
}

pub struct EnvelopeWidget<'a> {
    module: &'a Module,
    selected_point: usize,
    editing: bool,
}

impl<'a> EnvelopeWidget<'a> {
    pub fn new(module: &'a Module, selected_point: usize, editing: bool) -> Self {
        Self { module, selected_point, editing }
    }
}

impl Widget for EnvelopeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Some(points) = self.module.params.env_points() else { return };
        if points.is_empty() || area.width < 10 || area.height < 5 {
            return;
        }

        let list_height = 2u16;
        let list_area = Rect::new(area.x, area.y, area.width, list_height.min(area.height));
        let curve_area = Rect::new(
            area.x,
            area.y + list_height,
            area.width,
            area.height.saturating_sub(list_height),
        );

        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_style = Style::default().fg(Color::Black).bg(Color::Rgb(255, 200, 100));
        let editing_style = Style::default().fg(Color::Black).bg(Color::Yellow);
        let curve_point_style = Style::default().fg(Color::Cyan);

        let mut x_offset = 0u16;
        for (i, p) in points.iter().enumerate() {
            let is_sel = i == self.selected_point;
            let style = if is_sel { 
                if self.editing { editing_style } else { selected_style }
            } else { 
                value_style 
            };
            let type_ch = if p.curve { '~' } else { '/' };
            let type_style = if is_sel { style } else if p.curve { curve_point_style } else { label_style };
            
            if x_offset < list_area.width {
                set_cell(buf, list_area.x + x_offset, list_area.y, type_ch, type_style);
                x_offset += 1;
            }
            let info = format!("{:.2},{:.2}", p.time, p.value);
            for ch in info.chars() {
                if x_offset < list_area.width {
                    set_cell(buf, list_area.x + x_offset, list_area.y, ch, style);
                    x_offset += 1;
                }
            }
            if x_offset < list_area.width {
                set_cell(buf, list_area.x + x_offset, list_area.y, ' ', label_style);
                x_offset += 1;
            }
        }

        if curve_area.width < 5 || curve_area.height < 3 {
            return;
        }

        let w = (curve_area.width - 1) as f32;
        let h = (curve_area.height - 1) as f32;

        let curve_style = Style::default().fg(Color::Rgb(255, 200, 100));
        let point_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        let selected_point_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let editing_point_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

        let env = Envelope::new(
            points.iter()
                .map(|p| EnvelopePoint {
                    time: p.time,
                    value: p.value,
                    point_type: if p.curve { PointType::Curve } else { PointType::Linear },
                })
                .collect()
        );

        for x in 0..curve_area.width {
            let t = x as f32 / w;
            let val = env.output(t);
            let y = ((1.0 - val) * h) as u16;
            let screen_x = curve_area.x + x;
            let screen_y = curve_area.y + y;
            if screen_y < curve_area.y + curve_area.height {
                set_cell(buf, screen_x, screen_y, '·', curve_style);
            }
        }

        for (i, p) in points.iter().enumerate() {
            let px = (p.time * w) as u16;
            let py = ((1.0 - p.value) * h) as u16;
            let screen_x = curve_area.x + px.min(curve_area.width - 1);
            let screen_y = curve_area.y + py.min(curve_area.height - 1);
            let is_sel = i == self.selected_point;
            let style = if is_sel {
                if self.editing { editing_point_style } else { selected_point_style }
            } else {
                point_style
            };
            set_cell(buf, screen_x, screen_y, '●', style);
        }
    }
}


