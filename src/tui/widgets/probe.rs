use super::util::set_str;
use super::{ChartConfig, render_chart};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

pub struct ProbeWidget<'a> {
    history: &'a [f32],
    min: f32,
    max: f32,
    len: usize,
    current: f32,
}

impl<'a> ProbeWidget<'a> {
    pub fn new(history: &'a [f32], min: f32, max: f32, len: usize, current: f32) -> Self {
        Self {
            history,
            min,
            max,
            len,
            current,
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

        let min_str = format!("Min: {:.2}", self.min);
        let max_str = format!("Max: {:.2}", self.max);
        let len_ms = self.len as f32 / 44.1;
        let len_str = if len_ms >= 1000.0 {
            format!("Len: {:.1}s", len_ms / 1000.0)
        } else if len_ms >= 10.0 {
            format!("Len: {:.0}ms", len_ms)
        } else {
            format!("Len: {:.2}ms", len_ms)
        };
        set_str(buf, header_area.x, header_area.y + 2, &min_str, label_style);
        let mut x = header_area.x + min_str.len() as u16 + 2;
        set_str(buf, x, header_area.y + 2, &max_str, label_style);
        x += max_str.len() as u16 + 2;
        set_str(buf, x, header_area.y + 2, &len_str, selected_style);

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
