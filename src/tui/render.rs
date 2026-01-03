use super::bindings;
use super::grid::{Cell, GridPos};
use super::module::{Edge, Module, ModuleCategory, ModuleId, ModuleKind, ModuleParams, ParamKind};
use super::patch::Patch;
use crate::envelopes::{Envelope, EnvelopePoint, PointType};
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

const CELL_WIDTH: u16 = 5;
const CELL_HEIGHT: u16 = 3;

struct ChartConfig {
    color: Color,
    min: f32,
    max: f32,
    show_axes: bool,
    show_zero: bool,
    show_fill: bool,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            color: Color::Rgb(100, 220, 220),
            min: 0.0,
            max: 1.0,
            show_axes: true,
            show_zero: false,
            show_fill: true,
        }
    }
}

fn render_chart<F>(buf: &mut Buffer, area: Rect, config: &ChartConfig, sample_fn: F)
where
    F: Fn(f32) -> f32,
{
    if area.width < 5 || area.height < 3 {
        return;
    }

    let h = (area.height - 1) as f32;
    let range = config.max - config.min;
    let axis_style = Style::default().fg(Color::DarkGray);

    if config.show_axes {
        for y in 0..area.height {
            set_cell(buf, area.x, area.y + y, '│', axis_style);
        }
        for x in 0..area.width {
            set_cell(buf, area.x + x, area.y + area.height - 1, '─', axis_style);
        }
        set_cell(buf, area.x, area.y + area.height - 1, '└', axis_style);
    }

    if config.show_zero && config.min <= 0.0 && config.max >= 0.0 && range > 0.0 {
        let zero_y = ((config.max / range) * h) as u16;
        if zero_y < area.height - 1 {
            let zero_style = Style::default().fg(Color::Rgb(60, 60, 60));
            for x in 1..area.width {
                set_cell(buf, area.x + x, area.y + zero_y, '·', zero_style);
            }
        }
    }

    let curve_style = Style::default().fg(config.color);
    let fill_color = match config.color {
        Color::Rgb(r, g, b) => Color::Rgb(r / 3, g / 3, b / 3),
        _ => Color::Rgb(40, 80, 80),
    };
    let fill_style = Style::default().fg(fill_color);

    let chart_start = if config.show_axes { 1 } else { 0 };
    let chart_w = (area.width - chart_start) as usize;

    for screen_i in 0..chart_w {
        let x = chart_start + screen_i as u16;
        let t = screen_i as f32 / chart_w.max(1) as f32;
        let val = sample_fn(t);

        let normalized = if range > 0.0 {
            ((config.max - val) / range).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let y = (normalized * h) as u16;
        let screen_x = area.x + x;
        let line_y = y.min(area.height.saturating_sub(1));

        if config.show_fill {
            for fill_y in (line_y + 1)..(area.height - 1) {
                set_cell(buf, screen_x, area.y + fill_y, '·', fill_style);
            }
        }

        set_cell(buf, screen_x, area.y + line_y, '·', curve_style);
    }

    if config.show_axes {
        let max_str = format!("{:.1}", config.max);
        let min_str = format!("{:.1}", config.min);
        set_str(buf, area.x + 1, area.y, &max_str, axis_style);
        set_str(
            buf,
            area.x + 1,
            area.y + area.height - 2,
            &min_str,
            axis_style,
        );
    }
}

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

fn meter_char(val: f32) -> char {
    let v = val.abs();
    if v < 0.1 {
        ' '
    } else if v < 0.4 {
        '▁'
    } else if v < 0.7 {
        '▃'
    } else {
        '▅'
    }
}

fn meter_style(val: f32, base_color: Color) -> Style {
    let v = val.abs();
    let brightness = (v * 0.8).min(0.8) + 0.2;
    let (r, g, b) = match base_color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::White => (255, 255, 255),
        Color::Gray => (128, 128, 128),
        _ => (200, 200, 200),
    };
    let r = (r as f32 * brightness) as u8;
    let g = (g as f32 * brightness) as u8;
    let b = (b as f32 * brightness) as u8;
    Style::default().fg(Color::Rgb(r, g, b))
}

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
        }
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
        let kind = module.kind;
        let color = kind.color();
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
        } else {
            if is_left && is_right {
                "│   │".to_string()
            } else if is_left {
                "│    ".to_string()
            } else if is_right {
                "    │".to_string()
            } else {
                "     ".to_string()
            }
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
            if on_edge {
                if let Some(port) = info.input_ports.get(input_idx) {
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
                    let probe_value = if module.kind == ModuleKind::Probe {
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

    pub fn filter(
        mut self,
        filter: &'a str,
        modules: Vec<ModuleKind>,
        selection: usize,
        searching: bool,
    ) -> Self {
        self.filter = filter;
        self.filtered_modules = modules;
        self.filter_selection = selection;
        self.searching = searching;
        self
    }
}

impl Widget for PaletteWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let desc_style = Style::default().fg(Color::Gray).bg(Color::Rgb(40, 40, 40));
        let desc_y = area.y + area.height.saturating_sub(1);
        let max_w = area.width as usize;

        if self.searching {
            let mut y = area.y;
            let filter_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            set_str(buf, area.x, y, &format!(" /{} ", self.filter), filter_style);
            y += 1;

            for (idx, kind) in self.filtered_modules.iter().enumerate() {
                if y >= desc_y {
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

            if let Some(selected_kind) = self.filtered_modules.get(self.filter_selection) {
                let desc: String = selected_kind.description().chars().take(max_w).collect();
                for x in 0..area.width {
                    set_cell(buf, area.x + x, desc_y, ' ', desc_style);
                }
                set_str(buf, area.x, desc_y, &desc, desc_style);
            }

            return;
        }

        let categories = ModuleCategory::all();
        let mut y = area.y;

        for (cat_idx, cat) in categories.iter().enumerate() {
            if y >= desc_y {
                break;
            }

            let is_selected_cat = cat_idx == self.selected_category;
            let cat_style = if is_selected_cat {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            set_str(buf, area.x, y, &format!(" {} ", cat.name()), cat_style);
            y += 1;

            if is_selected_cat {
                let mods = ModuleKind::by_category(*cat);
                for (mod_idx, kind) in mods.iter().enumerate() {
                    if y >= desc_y {
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
                if let Some(selected_kind) = mods.get(self.selected_module) {
                    let desc: String = selected_kind.description().chars().take(max_w).collect();
                    for x in 0..area.width {
                        set_cell(buf, area.x + x, desc_y, ' ', desc_style);
                    }
                    set_str(buf, area.x, desc_y, &desc, desc_style);
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
    output_history: &'a [f32],
}

impl<'a> StatusWidget<'a> {
    pub fn new(cursor: GridPos, mode: &'a str) -> Self {
        Self {
            cursor,
            mode,
            message: None,
            playing: false,
            output_history: &[],
        }
    }

    pub fn message(mut self, msg: &'a str) -> Self {
        self.message = Some(msg);
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    pub fn output_history(mut self, history: &'a [f32]) -> Self {
        self.output_history = history;
        self
    }
}

impl Widget for StatusWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let pos_style = Style::default().fg(Color::DarkGray);
        let msg_style = Style::default().fg(Color::White);

        set_str(buf, area.x, area.y, &format!("[{}]", self.mode), mode_style);

        let pos_str = format!(" ({},{})", self.cursor.x, self.cursor.y);
        set_str(
            buf,
            area.x + self.mode.len() as u16 + 2,
            area.y,
            &pos_str,
            pos_style,
        );

        let mut wave_start = area.x + self.mode.len() as u16 + 2 + pos_str.len() as u16 + 1;
        if self.playing {
            let play_style = Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD);
            set_str(buf, wave_start, area.y, "▶", play_style);
            wave_start += 2;
        }

        let msg_len = self.message.map(|m| m.len() as u16 + 2).unwrap_or(0);
        let wave_end = area.x + area.width.saturating_sub(msg_len);

        if wave_end > wave_start && !self.output_history.is_empty() {
            let wave_width = (wave_end - wave_start) as usize;
            for i in 0..wave_width {
                let t = i as f32 / wave_width as f32;
                let idx = ((t * self.output_history.len() as f32) as usize)
                    .min(self.output_history.len().saturating_sub(1));
                let val = self.output_history[idx].abs();
                if val < 0.02 {
                    continue;
                }
                let color = if val > 0.15 {
                    Color::Yellow
                } else {
                    Color::Rgb(60, 60, 60)
                };
                set_cell(
                    buf,
                    wave_start + i as u16,
                    area.y,
                    '•',
                    Style::default().fg(color),
                );
            }
        }

        if let Some(msg) = self.message {
            let x = area.x + area.width.saturating_sub(msg.len() as u16 + 1);
            set_str(buf, x, area.y, msg, msg_style);
        }
    }
}

pub struct HelpWidget {
    bindings: &'static [bindings::Binding],
}

impl HelpWidget {
    pub fn new(bindings: &'static [bindings::Binding]) -> Self {
        Self { bindings }
    }
}

impl Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hints = bindings::hints(self.bindings);

        let key_style = Style::default().fg(Color::Cyan);
        let desc_style = Style::default().fg(Color::DarkGray);

        let mut y = area.y;
        let mut last_section = None;
        for (key, desc, section) in hints.iter() {
            if last_section.is_some() && last_section != Some(*section) {
                y += 1;
            }
            last_section = Some(*section);
            if y >= area.y + area.height {
                return;
            }
            set_str(buf, area.x, y, key, key_style);
            set_str(
                buf,
                area.x + key.chars().count() as u16 + 1,
                y,
                desc,
                desc_style,
            );
            y += 1;
        }
    }
}

pub struct EditWidget<'a> {
    module: &'a Module,
    selected_param: usize,
    patch: &'a Patch,
}

impl<'a> EditWidget<'a> {
    pub fn new(module: &'a Module, selected_param: usize, patch: &'a Patch) -> Self {
        Self {
            module,
            selected_param,
            patch,
        }
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

        let desc = self.module.kind.description();
        let desc_style = Style::default().fg(Color::DarkGray);
        set_str(buf, area.x, area.y + 1, desc, desc_style);

        let mut y = area.y + 3;

        for (i, def) in defs.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }
            let is_selected = i == self.selected_param;
            let is_connected = self.module.params.is_connected(i);

            let style = if is_selected {
                selected_style
            } else {
                label_style
            };

            let port_str = match def.kind {
                ParamKind::Input => "● ",
                ParamKind::Float { .. } => {
                    if is_connected {
                        "● "
                    } else {
                        "✕ "
                    }
                }
                ParamKind::Enum | ParamKind::Toggle => "  ",
            };
            set_str(buf, area.x, y, port_str, style);
            set_str(buf, area.x + 2, y, def.name, style);

            let val_x = area.x + 8;
            let v_style = if is_selected {
                selected_style
            } else {
                value_style
            };

            match &def.kind {
                ParamKind::Input => {
                    set_str(buf, val_x, y, "(input)", label_style);
                }
                ParamKind::Enum => {
                    let val_str = if let ModuleKind::DelayTap(delay_id) = self.module.kind {
                        if i == 0 {
                            let is_valid_delay = self
                                .patch
                                .module(delay_id)
                                .map(|m| m.kind == ModuleKind::Delay)
                                .unwrap_or(false);
                            if is_valid_delay {
                                if let Some(pos) = self.patch.module_position(delay_id) {
                                    format!("Delay @{},{}", pos.x, pos.y)
                                } else {
                                    "Delay (?)".to_string()
                                }
                            } else {
                                "Invalid".to_string()
                            }
                        } else {
                            self.module.params.enum_display().unwrap_or("?").to_string()
                        }
                    } else {
                        self.module.params.enum_display().unwrap_or("?").to_string()
                    };
                    set_str(buf, val_x, y, &val_str, v_style);
                    if is_selected {
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl>",
                            label_style,
                        );
                    }
                }
                ParamKind::Toggle => {
                    let val = self.module.params.get_toggle(i);
                    let val_str = if val { "on" } else { "off" };
                    set_str(buf, val_x, y, val_str, v_style);
                    if is_selected {
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl>",
                            label_style,
                        );
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
                        set_str(
                            buf,
                            val_x + val_str.len() as u16 + 1,
                            y,
                            "<hl> ;",
                            label_style,
                        );
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
        Self {
            module,
            selected_param,
        }
    }

    fn draw_curve(&self, buf: &mut Buffer, area: Rect, attack_ratio: f32, sustain: f32) {
        let config = ChartConfig {
            color: Color::Rgb(255, 200, 100),
            min: 0.0,
            max: 1.0,
            show_axes: true,
            show_zero: false,
            show_fill: true,
        };

        render_chart(buf, area, &config, |t| {
            if t < attack_ratio {
                if attack_ratio == 0.0 {
                    sustain
                } else {
                    t / attack_ratio
                }
            } else {
                let decay_progress = if attack_ratio >= 1.0 {
                    0.0
                } else {
                    (t - attack_ratio) / (1.0 - attack_ratio)
                };
                1.0 - decay_progress * (1.0 - sustain)
            }
        });
    }
}

impl<'a> Widget for AdsrWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (attack_ratio, sustain) = match &self.module.params {
            ModuleParams::Adsr {
                attack_ratio,
                sustain,
                ..
            } => (*attack_ratio, *sustain),
            _ => (0.5, 0.7),
        };

        let param_area = Rect::new(area.x, area.y, area.width, 3);
        let curve_area = Rect::new(
            area.x,
            area.y + 3,
            area.width,
            area.height.saturating_sub(3),
        );

        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let params = [("Atk Ratio", attack_ratio, ""), ("Sustain", sustain, "")];

        for (i, (name, val, suffix)) in params.iter().enumerate() {
            let y = param_area.y + i as u16;
            let is_selected = i == self.selected_param;
            let style = if is_selected {
                selected_style
            } else {
                label_style
            };
            let v_style = if is_selected {
                selected_style
            } else {
                value_style
            };

            set_str(buf, param_area.x, y, name, style);
            let val_str = format!("{:.2}{}", val, suffix);
            set_str(buf, param_area.x + 10, y, &val_str, v_style);

            if is_selected {
                set_str(
                    buf,
                    param_area.x + 10 + val_str.len() as u16 + 1,
                    y,
                    "<hl>",
                    label_style,
                );
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
        Self {
            module,
            selected_point,
            editing,
        }
    }
}

impl Widget for EnvelopeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Some(points) = self.module.params.env_points() else {
            return;
        };
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
        let selected_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(255, 200, 100));
        let editing_style = Style::default().fg(Color::Black).bg(Color::Yellow);
        let curve_point_style = Style::default().fg(Color::Cyan);

        let mut x_offset = 0u16;
        for (i, p) in points.iter().enumerate() {
            let is_sel = i == self.selected_point;
            let style = if is_sel {
                if self.editing {
                    editing_style
                } else {
                    selected_style
                }
            } else {
                value_style
            };
            let type_ch = if p.curve { '~' } else { '/' };
            let type_style = if is_sel {
                style
            } else if p.curve {
                curve_point_style
            } else {
                label_style
            };

            if x_offset < list_area.width {
                set_cell(
                    buf,
                    list_area.x + x_offset,
                    list_area.y,
                    type_ch,
                    type_style,
                );
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

        let env = Envelope::new(
            points
                .iter()
                .map(|p| EnvelopePoint {
                    time: p.time,
                    value: p.value,
                    point_type: if p.curve {
                        PointType::Curve
                    } else {
                        PointType::Linear
                    },
                })
                .collect(),
        );

        let config = ChartConfig {
            color: Color::Rgb(255, 200, 100),
            min: -1.0,
            max: 1.0,
            show_axes: true,
            show_zero: true,
            show_fill: false,
        };
        render_chart(buf, curve_area, &config, |t| env.output(t));

        let point_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        let selected_point_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);
        let editing_point_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

        let w = (curve_area.width - 2) as f32;
        let h = (curve_area.height - 2) as f32;
        for (i, p) in points.iter().enumerate() {
            let px = 1 + (p.time * w) as u16;
            let normalized = (1.0 - p.value) / 2.0;
            let py = (normalized * h) as u16;
            let screen_x = curve_area.x + px.min(curve_area.width - 1);
            let screen_y = curve_area.y + py.min(curve_area.height - 2);
            let is_sel = i == self.selected_point;
            let style = if is_sel {
                if self.editing {
                    editing_point_style
                } else {
                    selected_point_style
                }
            } else {
                point_style
            };
            set_cell(buf, screen_x, screen_y, '●', style);
        }
    }
}

pub struct ProbeWidget<'a> {
    history: &'a [f32],
    min: f32,
    max: f32,
    len: usize,
    current: f32,
    selected_param: usize,
}

impl<'a> ProbeWidget<'a> {
    pub fn new(
        history: &'a [f32],
        min: f32,
        max: f32,
        len: usize,
        current: f32,
        selected_param: usize,
    ) -> Self {
        Self {
            history,
            min,
            max,
            len,
            current,
            selected_param,
        }
    }
}

impl Widget for ProbeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let header_height = 4u16;
        let header_area = Rect::new(area.x, area.y, area.width, header_height.min(area.height));
        let chart_area = Rect::new(
            area.x,
            area.y + header_height,
            area.width,
            area.height.saturating_sub(header_height),
        );

        let title_style = Style::default()
            .fg(Color::Rgb(100, 220, 220))
            .add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);
        let selected_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(100, 220, 220));

        set_str(buf, header_area.x, header_area.y, "Probe", title_style);

        let current_str = format!("Value: {:.4}", self.current);
        set_str(
            buf,
            header_area.x,
            header_area.y + 1,
            &current_str,
            value_style,
        );

        let min_style = if self.selected_param == 0 {
            selected_style
        } else {
            label_style
        };
        let max_style = if self.selected_param == 1 {
            selected_style
        } else {
            label_style
        };
        let len_style = if self.selected_param == 2 {
            selected_style
        } else {
            label_style
        };

        let min_str = format!("Min: {:.2}", self.min);
        let max_str = format!("Max: {:.2}", self.max);
        let len_str = format!("Len: {}", self.len);
        set_str(buf, header_area.x, header_area.y + 2, &min_str, min_style);
        let mut x = header_area.x + min_str.len() as u16 + 2;
        set_str(buf, x, header_area.y + 2, &max_str, max_style);
        x += max_str.len() as u16 + 2;
        set_str(buf, x, header_area.y + 2, &len_str, len_style);
        set_str(
            buf,
            header_area.x,
            header_area.y + 3,
            "hl select, jk adjust, r reset, c clear",
            label_style,
        );

        let start = self.history.len().saturating_sub(self.len);
        let samples: &[f32] = &self.history[start..];

        if samples.is_empty() {
            return;
        }

        let config = ChartConfig {
            color: Color::Rgb(100, 220, 220),
            min: self.min,
            max: self.max,
            show_axes: true,
            show_zero: true,
            show_fill: true,
        };

        let samples_len = samples.len();
        render_chart(buf, chart_area, &config, |t| {
            let sample_idx = ((t * samples_len as f32) as usize).min(samples_len.saturating_sub(1));
            samples[sample_idx]
        });
    }
}
