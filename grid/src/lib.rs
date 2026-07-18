pub mod project;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    width: u16,
    height: u16,
}

impl Size {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub fn width(self) -> u16 {
        self.width
    }

    pub fn height(self) -> u16 {
        self.height
    }

    pub fn contains(self, position: Position) -> bool {
        position.x < self.width && position.y < self.height
    }
}

pub fn bounded_delta(min: u16, max: u16, delta: i16, size: u16) -> i16 {
    let lower = -(min as i16);
    let upper = size.saturating_sub(1).saturating_sub(max) as i16;
    delta.clamp(lower, upper)
}

impl Position {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    min: Position,
    max: Position,
}

impl Rect {
    pub fn from_points(a: Position, b: Position) -> Self {
        Self {
            min: Position::new(a.x.min(b.x), a.y.min(b.y)),
            max: Position::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn min(self) -> Position {
        self.min
    }

    pub fn max(self) -> Position {
        self.max
    }

    pub fn contains(self, position: Position) -> bool {
        position.x >= self.min.x
            && position.x <= self.max.x
            && position.y >= self.min.y
            && position.y <= self.max.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleId(u32);

impl ModuleId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    #[default]
    Right,
    Down,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_never_empty() {
        assert_eq!(Size::new(0, 0), Size::new(1, 1));
    }

    #[test]
    fn bounded_delta_keeps_a_span_inside_the_grid() {
        assert_eq!(bounded_delta(3, 5, -5, 8), -3);
        assert_eq!(bounded_delta(3, 5, 5, 8), 2);
    }

    #[test]
    fn rect_normalizes_points() {
        let rect = Rect::from_points(Position::new(4, 1), Position::new(2, 3));
        assert_eq!(rect.min(), Position::new(2, 1));
        assert_eq!(rect.max(), Position::new(4, 3));
    }
}
