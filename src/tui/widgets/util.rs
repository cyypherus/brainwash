use ratatui::{
    buffer::Buffer,
    style::{Color, Style},
};

pub fn set_cell(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char(ch).set_style(style);
    }
}

pub fn set_str(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    for (i, ch) in s.chars().enumerate() {
        set_cell(buf, x + i as u16, y, ch, style);
    }
}

pub fn meter_char(val: f32) -> char {
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

pub fn meter_style(val: f32, base_color: Color) -> Style {
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

pub const CELL_WIDTH: u16 = 5;
pub const CELL_HEIGHT: u16 = 3;
