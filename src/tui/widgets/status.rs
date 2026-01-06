use super::util::{set_cell, set_str};
use crate::tui::grid::GridPos;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

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
