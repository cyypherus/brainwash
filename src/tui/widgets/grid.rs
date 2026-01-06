use super::util::{CELL_HEIGHT, CELL_WIDTH, meter_char, meter_style, set_cell, set_str};
use crate::tui::grid::{Cell, GridPos};
use crate::tui::module::{Edge, Module, ModuleId, ModuleKind, StandardModule};
use crate::tui::patch::Patch;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::collections::HashMap;
use std::sync::LazyLock;

static EMPTY_METERS: LazyLock<HashMap<ModuleId, Vec<f32>>> = LazyLock::new(HashMap::new);
static EMPTY_PROBES: LazyLock<HashMap<ModuleId, f32>> = LazyLock::new(HashMap::new);

pub struct GridWidget<'a> {
    patch: &'a Patch,
    cursor: GridPos,
    view_center: GridPos,
    moving: Option<ModuleId>,
    copy_previews: Vec<(Module, GridPos)>,
    move_previews: Vec<(Module, GridPos)>,
    probe_values: &'a HashMap<ModuleId, f32>,
    meter_values: &'a HashMap<ModuleId, Vec<f32>>,
    show_meters: bool,
    selection: Option<(GridPos, GridPos)>,
    disabled_pulse: f32,
}

impl<'a> GridWidget<'a> {
    pub fn new(patch: &'a Patch) -> Self {
        Self {
            patch,
            cursor: GridPos::new(0, 0),
            view_center: GridPos::new(0, 0),
            moving: None,
            copy_previews: Vec::new(),
            move_previews: Vec::new(),
            probe_values: &EMPTY_PROBES,
            meter_values: &EMPTY_METERS,
            show_meters: false,
            selection: None,
            disabled_pulse: 0.0,
        }
    }

    pub fn disabled_pulse(mut self, value: f32) -> Self {
        self.disabled_pulse = value;
        self
    }

    pub fn cursor(mut self, pos: GridPos) -> Self {
        self.cursor = pos;
        self.view_center = pos;
        self
    }

    pub fn view_center(mut self, pos: GridPos) -> Self {
        self.view_center = pos;
        self
    }

    pub fn moving(mut self, id: Option<ModuleId>) -> Self {
        self.moving = id;
        self
    }

    pub fn copy_previews(mut self, previews: Vec<(Module, GridPos)>) -> Self {
        self.copy_previews = previews;
        self
    }

    pub fn move_previews(mut self, previews: Vec<(Module, GridPos)>) -> Self {
        self.move_previews = previews;
        self
    }

    pub fn selection(mut self, sel: Option<(GridPos, GridPos)>) -> Self {
        self.selection = sel;
        self
    }

    pub fn probe_values(mut self, values: &'a HashMap<ModuleId, f32>) -> Self {
        self.probe_values = values;
        self
    }

    pub fn meter_values(mut self, values: &'a HashMap<ModuleId, Vec<f32>>) -> Self {
        self.meter_values = values;
        self
    }

    pub fn show_meters(mut self, show: bool) -> Self {
        self.show_meters = show;
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
        let bg = if is_cursor {
            Color::Rgb(40, 40, 50)
        } else {
            Color::Reset
        };

        for dy in 0..CELL_HEIGHT {
            for dx in 0..CELL_WIDTH {
                let ch = if dx == CELL_WIDTH / 2 && dy == CELL_HEIGHT / 2 {
                    '·'
                } else {
                    ' '
                };
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

    fn render_module(
        &self,
        buf: &mut Buffer,
        sx: u16,
        sy: u16,
        module: &Module,
        local_x: u8,
        local_y: u8,
        is_cursor: bool,
        is_moving: bool,
        probe_value: Option<f32>,
        meter_values: Option<&Vec<f32>>,
    ) {
        let color = if module.disabled {
            let r = (50.0 + self.disabled_pulse * 150.0) as u8;
            let g = (50.0 * (1.0 - self.disabled_pulse)) as u8;
            let b = (50.0 * (1.0 - self.disabled_pulse)) as u8;
            Color::Rgb(r, g, b)
        } else {
            module.color()
        };
        let width = module.width();
        let height = module.height();
        let is_single_height = height == 1;
        let is_single_width = width == 1;

        let (border_style, text_style, inner_style) = if is_moving && is_cursor {
            (
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(120, 120, 160)),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(120, 120, 160))
                    .add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Rgb(120, 120, 160)),
            )
        } else if is_moving {
            (
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(80, 80, 100)),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(80, 80, 100))
                    .add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Rgb(80, 80, 100)),
            )
        } else if is_cursor {
            let cursor_bg = if color == Color::White {
                Color::Rgb(100, 100, 120)
            } else {
                color
            };
            (
                Style::default().fg(Color::Black).bg(cursor_bg),
                Style::default()
                    .fg(Color::Black)
                    .bg(cursor_bg)
                    .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(120, 120, 160))
        } else if is_moving {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(80, 80, 100))
        } else if is_cursor {
            let cursor_bg = if color == Color::White {
                Color::Rgb(100, 100, 120)
            } else {
                color
            };
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
            if is_left && is_right {
                format!("{}───{}", tl, tr)
            } else if is_left {
                format!("{}────", tl)
            } else if is_right {
                format!("────{}", tr)
            } else {
                "─────".to_string()
            }
        } else if is_left && is_right {
            "│   │".to_string()
        } else if is_left {
            "│    ".to_string()
        } else if is_right {
            "    │".to_string()
        } else {
            "     ".to_string()
        };

        let mid_str = if is_left && is_right {
            "│   │".to_string()
        } else if is_left {
            "│    ".to_string()
        } else if is_right {
            "    │".to_string()
        } else {
            "     ".to_string()
        };

        let bot_str = if is_bottom {
            if is_left && is_right {
                format!("{}───{}", bl, br)
            } else if is_left {
                format!("{}────", bl)
            } else if is_right {
                format!("────{}", br)
            } else {
                "─────".to_string()
            }
        } else {
            mid_str.clone()
        };

        set_str(buf, sx, sy, &top_str, border_style);
        set_str(buf, sx, sy + 1, &mid_str, border_style);
        set_str(buf, sx, sy + 2, &bot_str, border_style);

        let cx = sx + CELL_WIDTH / 2;
        let cy = sy + CELL_HEIGHT / 2;

        let info = module.render_info();

        if is_top && is_left {
            if let Some(val) = probe_value {
                let val_str = format!("{:.1}", val);
                for (i, ch) in val_str.chars().take(3).enumerate() {
                    set_cell(buf, sx + 1 + i as u16, sy + 1, ch, text_style);
                }
            } else {
                for (i, ch) in info.name.chars().take(3).enumerate() {
                    set_cell(buf, sx + 1 + i as u16, sy + 1, ch, text_style);
                }
            }
        }

        if info.input_edges.len() == 1 {
            let edge = info.input_edges[0];
            let input_idx = match edge {
                Edge::Top => local_x as usize,
                Edge::Left => local_y as usize,
                _ => usize::MAX,
            };
            let on_edge = match edge {
                Edge::Top => local_y == 0,
                Edge::Left => local_x == 0,
                _ => false,
            };
            if on_edge && let Some(port) = info.input_ports.get(input_idx) {
                let port_char = if port.connected { '●' } else { '✕' };
                let meter_val = meter_values
                    .and_then(|m| m.get(input_idx).copied())
                    .unwrap_or(0.0);
                match edge {
                    Edge::Top => {
                        if self.show_meters {
                            let mc = meter_char(meter_val);
                            let ms = meter_style(meter_val, color);
                            set_cell(buf, cx - 1, sy, mc, ms);
                        } else if port.label != ' ' {
                            set_cell(buf, cx - 1, sy, port.label, port_style);
                        }
                        set_cell(buf, cx, sy, port_char, port_style);
                    }
                    Edge::Left => {
                        if self.show_meters {
                            let mc = meter_char(meter_val);
                            let ms = meter_style(meter_val, color);
                            set_cell(buf, sx, cy - 1, mc, ms);
                        } else if port.label != ' ' {
                            set_cell(buf, sx, cy - 1, port.label, port_style);
                        }
                        set_cell(buf, sx, cy, port_char, port_style);
                    }
                    _ => {}
                }
            }
        } else {
            for (input_port_idx, edge) in info.input_edges.iter().enumerate() {
                if let Some(port) = info.input_ports.get(input_port_idx) {
                    let port_char = if port.connected { '●' } else { '✕' };
                    match edge {
                        Edge::Top if local_y == 0 && local_x == 0 => {
                            set_cell(buf, cx, sy, port_char, port_style);
                        }
                        Edge::Left if local_x == 0 && local_y == 0 => {
                            set_cell(buf, sx, cy, port_char, port_style);
                        }
                        _ => {}
                    }
                }
            }
        }

        if info.output_edges.len() == 1 {
            let edge = info.output_edges[0];
            let output_idx = match edge {
                Edge::Bottom => local_x as usize,
                Edge::Right => local_y as usize,
                _ => usize::MAX,
            };
            let on_edge = match edge {
                Edge::Bottom => local_y == info.height - 1,
                Edge::Right => local_x == info.width - 1,
                _ => false,
            };
            if on_edge && output_idx < info.output_ports.len() {
                match edge {
                    Edge::Bottom => set_cell(buf, cx, sy + CELL_HEIGHT - 1, '○', port_style),
                    Edge::Right => set_cell(buf, sx + CELL_WIDTH - 1, cy, '○', port_style),
                    _ => {}
                }
            }
        } else {
            for (output_port_idx, edge) in info.output_edges.iter().enumerate() {
                if output_port_idx < info.output_ports.len() {
                    match edge {
                        Edge::Bottom if local_y == info.height - 1 && local_x == 0 => {
                            set_cell(buf, cx, sy + CELL_HEIGHT - 1, '○', port_style);
                        }
                        Edge::Right if local_x == info.width - 1 && local_y == 0 => {
                            set_cell(buf, sx + CELL_WIDTH - 1, cy, '○', port_style);
                        }
                        _ => {}
                    }
                }
            }
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

    fn render_cell(
        &self,
        buf: &mut Buffer,
        area: Rect,
        grid_pos: GridPos,
        viewport_origin: GridPos,
    ) {
        let (sx, sy) = self.screen_pos(grid_pos, viewport_origin, area);

        if sx < area.x
            || sy < area.y
            || sx + CELL_WIDTH > area.x + area.width
            || sy + CELL_HEIGHT > area.y + area.height
        {
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
            Cell::Module {
                id,
                local_x,
                local_y,
            } => {
                if let Some(module) = self.patch.module(id) {
                    let is_moving = self.moving == Some(id);
                    let probe_value = if module.kind == ModuleKind::Standard(StandardModule::Probe)
                    {
                        self.probe_values.get(&id).copied()
                    } else {
                        None
                    };
                    let meter = self.meter_values.get(&id);
                    self.render_module(
                        buf,
                        sx,
                        sy,
                        module,
                        local_x,
                        local_y,
                        is_cursor,
                        is_moving,
                        probe_value,
                        meter,
                    );
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
            Cell::ChannelCross { color_v, color_h } => {
                let cx = sx + CELL_WIDTH / 2;
                let cy = sy + CELL_HEIGHT / 2;
                let style_v = if is_cursor {
                    Style::default().fg(Color::White).bg(color_v)
                } else {
                    Style::default().fg(color_v)
                };
                let style_h = if is_cursor {
                    Style::default().fg(Color::White).bg(color_h)
                } else {
                    Style::default().fg(color_h)
                };
                for dy in 0..CELL_HEIGHT {
                    if sy + dy != cy {
                        set_cell(buf, cx, sy + dy, '│', style_v);
                    }
                }
                for dx in 0..CELL_WIDTH {
                    set_cell(buf, sx + dx, cy, '─', style_h);
                }
                set_cell(buf, cx, cy, '┼', style_v);
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
                    for dx in 0..=CELL_WIDTH / 2 {
                        set_cell(buf, sx + dx, cy, '─', style);
                    }
                    set_cell(buf, cx, cy, '┐', style);
                    for dy in (CELL_HEIGHT / 2 + 1)..CELL_HEIGHT {
                        set_cell(buf, cx, sy + dy, '│', style);
                    }
                } else {
                    for dy in 0..=CELL_HEIGHT / 2 {
                        set_cell(buf, cx, sy + dy, '│', style);
                    }
                    set_cell(buf, cx, cy, '└', style);
                    for dx in (CELL_WIDTH / 2 + 1)..CELL_WIDTH {
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

        let origin_x = if self.view_center.x < half_cols {
            0
        } else if self.view_center.x + half_cols >= grid.width() {
            grid.width().saturating_sub(visible_cols)
        } else {
            self.view_center.x - half_cols
        };

        let origin_y = if self.view_center.y < half_rows {
            0
        } else if self.view_center.y + half_rows >= grid.height() {
            grid.height().saturating_sub(visible_rows)
        } else {
            self.view_center.y - half_rows
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

        for (module, pos) in &self.copy_previews {
            let width = module.width();
            let height = module.height();
            for ly in 0..height {
                for lx in 0..width {
                    let gx = pos.x + lx as u16;
                    let gy = pos.y + ly as u16;
                    if gx >= origin_x && gy >= origin_y {
                        let (sx, sy) =
                            self.screen_pos(GridPos::new(gx, gy), viewport_origin, grid_area);
                        if sx < grid_area.x + grid_area.width && sy < grid_area.y + grid_area.height
                        {
                            let is_cursor = gx == self.cursor.x && gy == self.cursor.y;
                            self.render_module(
                                buf, sx, sy, module, lx, ly, is_cursor, true, None, None,
                            );
                        }
                    }
                }
            }
        }

        for (module, pos) in &self.move_previews {
            let width = module.width();
            let height = module.height();
            for ly in 0..height {
                for lx in 0..width {
                    let gx = pos.x + lx as u16;
                    let gy = pos.y + ly as u16;
                    if gx >= origin_x && gy >= origin_y {
                        let (sx, sy) =
                            self.screen_pos(GridPos::new(gx, gy), viewport_origin, grid_area);
                        if sx < grid_area.x + grid_area.width && sy < grid_area.y + grid_area.height
                        {
                            let is_cursor = gx == self.cursor.x && gy == self.cursor.y;
                            self.render_module(
                                buf, sx, sy, module, lx, ly, is_cursor, true, None, None,
                            );
                        }
                    }
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
