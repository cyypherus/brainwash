mod util;

mod adsr;
mod edit;
mod envelope;
mod grid;
mod help;
mod palette;
mod probe;
mod sample;
mod status;

pub use util::{set_cell, set_str};

pub use adsr::AdsrWidget;
pub use edit::EditWidget;
pub use envelope::EnvelopeWidget;
pub use grid::GridWidget;
pub use help::HelpWidget;
pub use palette::PaletteWidget;
pub use probe::ProbeWidget;
pub use sample::SampleWidget;
pub use status::StatusWidget;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

pub struct ChartConfig {
    pub color: Color,
    pub min: f32,
    pub max: f32,
    pub show_axes: bool,
    pub show_zero: bool,
    pub show_fill: bool,
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

pub fn render_chart<F>(buf: &mut Buffer, area: Rect, config: &ChartConfig, sample_fn: F)
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
