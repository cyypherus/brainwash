use super::util::set_str;
use crate::tui::bindings::{self, Binding};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct HelpWidget<'a> {
    bindings: &'a [Binding],
    scroll: usize,
}

impl<'a> HelpWidget<'a> {
    pub fn new(bindings: &'a [Binding], scroll: usize) -> Self {
        Self { bindings, scroll }
    }
}

impl Widget for HelpWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hints = bindings::hints(self.bindings);

        let key_style = Style::default().fg(Color::Cyan);
        let desc_style = Style::default().fg(Color::DarkGray);
        let scroll_style = Style::default().fg(Color::DarkGray);

        let mut total_lines = 0usize;
        let mut last_section_for_count = None;
        for (_, _, section) in hints.iter() {
            if last_section_for_count.is_some() && last_section_for_count != Some(*section) {
                total_lines += 1;
            }
            last_section_for_count = Some(*section);
            total_lines += 1;
        }

        let visible_height = area.height as usize;
        let can_scroll = total_lines > visible_height;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);

        let mut y = area.y;
        let mut line_idx = 0usize;
        let mut last_section = None;
        for (key, desc, section) in hints.iter() {
            if last_section.is_some() && last_section != Some(*section) {
                if line_idx >= scroll {
                    y += 1;
                }
                line_idx += 1;
            }
            last_section = Some(*section);

            if line_idx >= scroll {
                if y >= area.y + area.height {
                    break;
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
            line_idx += 1;
        }

        if can_scroll {
            if scroll > 0 {
                set_str(
                    buf,
                    area.x + area.width.saturating_sub(2),
                    area.y,
                    "▲",
                    scroll_style,
                );
            }
            if scroll + visible_height < total_lines {
                set_str(
                    buf,
                    area.x + area.width.saturating_sub(2),
                    area.y + area.height - 1,
                    "▼",
                    scroll_style,
                );
            }
        }
    }
}
