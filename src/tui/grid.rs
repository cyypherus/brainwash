use super::module::ModuleId;
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: u16,
    pub y: u16,
}

impl GridPos {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub fn down(self) -> Self {
        Self::new(self.x, self.y + 1)
    }

    pub fn right(self) -> Self {
        Self::new(self.x + 1, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Down,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Module {
        id: ModuleId,
        local_x: u8,
        local_y: u8,
    },
    ChannelV {
        color: Color,
    },
    ChannelH {
        color: Color,
    },
    ChannelCross {
        color_v: Color,
        color_h: Color,
    },
    ChannelCorner {
        color: Color,
        down_right: bool,
    },
}

impl Cell {
    pub fn is_empty(&self) -> bool {
        matches!(self, Cell::Empty)
    }

    pub fn is_channel(&self) -> bool {
        matches!(
            self,
            Cell::ChannelV { .. }
                | Cell::ChannelH { .. }
                | Cell::ChannelCross { .. }
                | Cell::ChannelCorner { .. }
        )
    }
}

#[derive(Clone)]
pub struct Grid {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::Empty; (width as usize) * (height as usize)],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    fn index(&self, pos: GridPos) -> Option<usize> {
        if pos.x < self.width && pos.y < self.height {
            Some((pos.y as usize) * (self.width as usize) + (pos.x as usize))
        } else {
            None
        }
    }

    pub fn get(&self, pos: GridPos) -> Cell {
        self.index(pos)
            .map(|i| self.cells[i])
            .unwrap_or(Cell::Empty)
    }

    pub fn set(&mut self, pos: GridPos, cell: Cell) {
        if let Some(i) = self.index(pos) {
            self.cells[i] = cell;
        }
    }

    pub fn in_bounds(&self, pos: GridPos) -> bool {
        pos.x < self.width && pos.y < self.height
    }

    pub fn place_module(&mut self, id: ModuleId, pos: GridPos, width: u8, height: u8) {
        for dy in 0..height {
            for dx in 0..width {
                let p = GridPos::new(pos.x + dx as u16, pos.y + dy as u16);
                if self.in_bounds(p) {
                    self.set(
                        p,
                        Cell::Module {
                            id,
                            local_x: dx,
                            local_y: dy,
                        },
                    );
                }
            }
        }
    }

    pub fn remove_module(&mut self, id: ModuleId) {
        for cell in &mut self.cells {
            if let Cell::Module { id: cid, .. } = cell
                && *cid == id
            {
                *cell = Cell::Empty;
            }
        }
    }

    pub fn clear_all(&mut self) {
        self.cells.fill(Cell::Empty);
    }

    pub fn clear_channels(&mut self) {
        for cell in &mut self.cells {
            if cell.is_channel() {
                *cell = Cell::Empty;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_bounds() {
        let grid = Grid::new(10, 8);
        assert!(grid.in_bounds(GridPos::new(0, 0)));
        assert!(grid.in_bounds(GridPos::new(9, 7)));
        assert!(!grid.in_bounds(GridPos::new(10, 0)));
    }

    #[test]
    fn test_place_module() {
        let mut grid = Grid::new(10, 8);
        let id = ModuleId(0);
        grid.place_module(id, GridPos::new(2, 2), 1, 2);
        assert_eq!(
            grid.get(GridPos::new(2, 2)),
            Cell::Module {
                id,
                local_x: 0,
                local_y: 0
            }
        );
        assert_eq!(
            grid.get(GridPos::new(2, 3)),
            Cell::Module {
                id,
                local_x: 0,
                local_y: 1
            }
        );
    }
}
