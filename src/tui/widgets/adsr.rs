use super::util::set_str;
use super::{ChartConfig, render_chart};
use crate::tui::module::{Module, ModuleParams};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

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
