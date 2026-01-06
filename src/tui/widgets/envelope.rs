use super::util::set_cell;
use super::{ChartConfig, render_chart};
use crate::envelopes::{Envelope, EnvelopePoint, PointType};
use crate::tui::module::Module;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

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

        let chart_w = (curve_area.width - 1) as f32;
        let chart_h = (curve_area.height - 1) as f32;
        for (i, p) in points.iter().enumerate() {
            let px = 1 + (p.time * chart_w) as u16;
            let normalized = (1.0 - p.value) / 2.0;
            let py = (normalized * chart_h) as u16;
            let screen_x = curve_area.x + px.min(curve_area.width - 1);
            let screen_y = curve_area.y + py.min(curve_area.height - 1);
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
