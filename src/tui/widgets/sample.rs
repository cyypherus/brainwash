use super::util::{set_cell, set_str};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct SampleWidget<'a> {
    samples: &'a [f32],
    zoom: f32,
    offset: f32,
}

impl<'a> SampleWidget<'a> {
    pub fn new(samples: &'a [f32], zoom: f32, offset: f32) -> Self {
        Self {
            samples,
            zoom,
            offset,
        }
    }
}

impl Widget for SampleWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.samples.is_empty() || area.width < 5 || area.height < 3 {
            let label_style = Style::default().fg(Color::DarkGray);
            set_str(buf, area.x, area.y, "(no sample loaded)", label_style);
            return;
        }

        let (min, max) = self
            .samples
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), &v| {
                (min.min(v), max.max(v))
            });
        let padding = (max - min).abs() * 0.05;
        let chart_min = min - padding;
        let chart_max = max + padding;

        let header_height = 2u16;
        let header_area = Rect::new(area.x, area.y, area.width, header_height.min(area.height));
        let chart_area = Rect::new(
            area.x,
            area.y + header_height,
            area.width,
            area.height.saturating_sub(header_height),
        );

        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);

        let sample_count = self.samples.len();
        let duration_ms = (sample_count as f32 / 44.1) as usize;
        let duration_str = if duration_ms >= 1000 {
            format!("{:.2}s", duration_ms as f32 / 1000.0)
        } else {
            format!("{}ms", duration_ms)
        };
        let info = format!("{} samples ({})", sample_count, duration_str);
        let zoom_str = if self.zoom > 1.0 {
            format!("  Zoom: {:.0}x", self.zoom)
        } else {
            String::new()
        };
        set_str(buf, header_area.x, header_area.y, &info, value_style);
        set_str(
            buf,
            header_area.x + info.len() as u16,
            header_area.y,
            &zoom_str,
            label_style,
        );
        set_str(
            buf,
            header_area.x,
            header_area.y + 1,
            "jk zoom, hl pan, r reset",
            label_style,
        );

        if chart_area.width < 3 || chart_area.height < 3 {
            return;
        }

        let h = (chart_area.height - 1) as f32;
        let range = chart_max - chart_min;
        let axis_style = Style::default().fg(Color::DarkGray);
        let wave_color = Color::Rgb(100, 180, 255);
        let wave_style = Style::default().fg(wave_color);

        for y in 0..chart_area.height {
            set_cell(buf, chart_area.x, chart_area.y + y, '│', axis_style);
        }
        for x in 0..chart_area.width {
            set_cell(
                buf,
                chart_area.x + x,
                chart_area.y + chart_area.height - 1,
                '─',
                axis_style,
            );
        }
        set_cell(
            buf,
            chart_area.x,
            chart_area.y + chart_area.height - 1,
            '└',
            axis_style,
        );

        let chart_w = (chart_area.width - 1) as usize;
        let samples = self.samples;
        let samples_len = samples.len();

        let view_width = 1.0 / self.zoom;
        let view_start = self.offset;

        for screen_i in 0..chart_w {
            let x = 1 + screen_i as u16;
            let t0 = screen_i as f32 / chart_w.max(1) as f32;
            let t1 = (screen_i + 1) as f32 / chart_w.max(1) as f32;

            let sample_start = ((view_start + t0 * view_width) * samples_len as f32) as usize;
            let sample_end = ((view_start + t1 * view_width) * samples_len as f32) as usize;
            let sample_start = sample_start.min(samples_len.saturating_sub(1));
            let sample_end = sample_end.min(samples_len).max(sample_start + 1);

            let (min_val, max_val) = samples[sample_start..sample_end]
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), &v| {
                    (min.min(v), max.max(v))
                });

            let min_normalized = if range > 0.0 {
                ((chart_max - max_val) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let max_normalized = if range > 0.0 {
                ((chart_max - min_val) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };

            let top_y = (min_normalized * h) as u16;
            let bottom_y = (max_normalized * h) as u16;
            let top_y = top_y.min(chart_area.height.saturating_sub(2));
            let bottom_y = bottom_y.min(chart_area.height.saturating_sub(2));

            let screen_x = chart_area.x + x;

            for fill_y in top_y..=bottom_y {
                if fill_y < chart_area.height - 1 {
                    set_cell(buf, screen_x, chart_area.y + fill_y, '█', wave_style);
                }
            }
        }
    }
}
