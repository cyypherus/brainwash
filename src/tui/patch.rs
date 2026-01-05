use super::grid::{Cell, Grid, GridPos};
use super::module::{Module, ModuleId, ModuleKind, StandardModule, SubPatchId, SubpatchModule};
use ratatui::style::Color;
use std::collections::HashMap;

#[derive(Clone)]
pub struct SubPatchDef {
    pub name: String,
    pub color: Color,
    pub patch: Patch,
}

impl SubPatchDef {
    pub fn new(name: String, color: Color) -> Self {
        Self {
            name,
            color,
            patch: Patch::new(10, 10),
        }
    }

    pub fn inputs(&self) -> impl Iterator<Item = &Module> {
        self.patch
            .all_modules()
            .filter(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubIn))
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Module> {
        self.patch
            .all_modules()
            .filter(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubOut))
    }

    pub fn input_count(&self) -> usize {
        self.inputs().count()
    }

    pub fn output_count(&self) -> usize {
        self.outputs().count()
    }
}

#[derive(Clone)]
pub struct PatchSet {
    pub root: Patch,
    pub subpatches: HashMap<SubPatchId, SubPatchDef>,
    pub next_subpatch_id: u32,
}

impl PatchSet {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            root: Patch::new(width, height),
            subpatches: HashMap::new(),
            next_subpatch_id: 0,
        }
    }

    pub fn create_subpatch(&mut self, name: String, color: Color) -> SubPatchId {
        let id = SubPatchId(self.next_subpatch_id);
        self.next_subpatch_id += 1;
        self.subpatches.insert(id, SubPatchDef::new(name, color));
        id
    }

    pub fn subpatch(&self, id: SubPatchId) -> Option<&SubPatchDef> {
        self.subpatches.get(&id)
    }

    pub fn subpatch_mut(&mut self, id: SubPatchId) -> Option<&mut SubPatchDef> {
        self.subpatches.get_mut(&id)
    }
}

#[derive(Clone)]
pub struct Patch {
    grid: Grid,
    modules: HashMap<ModuleId, Module>,
    positions: HashMap<ModuleId, GridPos>,
    next_module_id: u32,
    output_id: Option<ModuleId>,
}

impl Patch {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            grid: Grid::new(width, height),
            modules: HashMap::new(),
            positions: HashMap::new(),
            next_module_id: 0,
            output_id: None,
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn module(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }

    pub fn module_mut(&mut self, id: ModuleId) -> Option<&mut Module> {
        self.modules.get_mut(&id)
    }

    pub fn all_modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    pub fn module_at(&self, pos: GridPos) -> Option<&Module> {
        match self.grid.get(pos) {
            Cell::Module { id, .. } => self.modules.get(&id),
            _ => None,
        }
    }

    pub fn module_id_at(&self, pos: GridPos) -> Option<ModuleId> {
        match self.grid.get(pos) {
            Cell::Module { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn module_position(&self, id: ModuleId) -> Option<GridPos> {
        self.positions.get(&id).copied()
    }

    pub fn output_id(&self) -> Option<ModuleId> {
        self.output_id
    }

    pub fn add_module(&mut self, kind: ModuleKind, pos: GridPos) -> Option<ModuleId> {
        if kind == ModuleKind::Standard(StandardModule::Output) && self.output_id.is_some() {
            return None;
        }

        let id = ModuleId(self.next_module_id);
        let module = Module::new(id, kind);
        let width = module.width();
        let height = module.height();

        if !self.grid.place_module(id, pos, width, height) {
            return None;
        }

        self.modules.insert(id, module);
        self.positions.insert(id, pos);
        self.next_module_id += 1;

        if kind == ModuleKind::Standard(StandardModule::Output) {
            self.output_id = Some(id);
        }

        self.rebuild_channels();
        Some(id)
    }

    pub fn add_module_clone(&mut self, source: &Module, pos: GridPos) -> Option<ModuleId> {
        if source.kind == ModuleKind::Standard(StandardModule::Output) && self.output_id.is_some() {
            return None;
        }

        let id = ModuleId(self.next_module_id);
        let mut module = source.clone();
        module.id = id;
        let width = module.width();
        let height = module.height();

        if !self.grid.place_module(id, pos, width, height) {
            return None;
        }

        self.modules.insert(id, module);
        self.positions.insert(id, pos);
        self.next_module_id += 1;

        if source.kind == ModuleKind::Standard(StandardModule::Output) {
            self.output_id = Some(id);
        }

        self.rebuild_channels();
        Some(id)
    }

    pub fn remove_module(&mut self, id: ModuleId) -> bool {
        if !self.modules.contains_key(&id) {
            return false;
        }

        self.grid.remove_module(id);
        self.positions.remove(&id);

        if self.output_id == Some(id) {
            self.output_id = None;
        }

        self.modules.remove(&id);
        self.rebuild_channels();
        true
    }

    pub fn move_module(&mut self, id: ModuleId, new_pos: GridPos) -> bool {
        let Some(module) = self.modules.get(&id) else {
            return false;
        };
        let width = module.width();
        let height = module.height();

        self.grid.clear_channels();

        for dy in 0..height {
            for dx in 0..width {
                let p = GridPos::new(new_pos.x + dx as u16, new_pos.y + dy as u16);
                if !self.grid.in_bounds(p) {
                    self.rebuild_channels();
                    return false;
                }
                let cell = self.grid.get(p);
                match cell {
                    Cell::Empty => {}
                    Cell::Module {
                        id: existing_id, ..
                    } if existing_id == id => {}
                    _ => {
                        self.rebuild_channels();
                        return false;
                    }
                }
            }
        }

        self.grid.remove_module(id);
        self.grid.place_module(id, new_pos, width, height);
        self.positions.insert(id, new_pos);
        self.rebuild_channels();
        true
    }

    pub fn move_modules(&mut self, moves: &[(ModuleId, GridPos)]) -> usize {
        self.grid.clear_channels();

        let module_data: Vec<(ModuleId, GridPos, u8, u8)> = moves
            .iter()
            .filter_map(|(id, new_pos)| {
                let module = self.modules.get(id)?;
                Some((*id, *new_pos, module.width(), module.height()))
            })
            .collect();

        for &(id, _, _, _) in &module_data {
            self.grid.remove_module(id);
        }

        for &(_, new_pos, width, height) in &module_data {
            for dy in 0..height {
                for dx in 0..width {
                    let p = GridPos::new(new_pos.x + dx as u16, new_pos.y + dy as u16);
                    if !self.grid.in_bounds(p) || !self.grid.get(p).is_empty() {
                        for &(id, _, width, height) in &module_data {
                            if let Some(&old_pos) = self.positions.get(&id) {
                                self.grid.place_module(id, old_pos, width, height);
                            }
                        }
                        self.rebuild_channels();
                        return 0;
                    }
                }
            }
        }

        for &(id, new_pos, width, height) in &module_data {
            self.grid.place_module(id, new_pos, width, height);
            self.positions.insert(id, new_pos);
        }

        self.rebuild_channels();
        module_data.len()
    }

    pub fn rotate_module(&mut self, id: ModuleId) -> bool {
        let Some(module) = self.modules.get(&id) else {
            return false;
        };

        if module.kind.is_routing() {
            return false;
        }

        let pos = match self.positions.get(&id) {
            Some(p) => *p,
            None => return false,
        };

        let new_width = module.height();
        let new_height = module.width();

        self.grid.clear_channels();
        self.grid.remove_module(id);

        for dy in 0..new_height {
            for dx in 0..new_width {
                let p = GridPos::new(pos.x + dx as u16, pos.y + dy as u16);
                if !self.grid.in_bounds(p) || !self.grid.get(p).is_empty() {
                    let module = self.modules.get(&id).unwrap();
                    self.grid
                        .place_module(id, pos, module.width(), module.height());
                    self.rebuild_channels();
                    return false;
                }
            }
        }

        self.modules.get_mut(&id).unwrap().rotate();
        let module = self.modules.get(&id).unwrap();
        self.grid
            .place_module(id, pos, module.width(), module.height());
        self.rebuild_channels();
        true
    }

    pub fn refit_module(&mut self, id: ModuleId) -> bool {
        let Some(module) = self.modules.get(&id) else {
            return false;
        };
        let Some(&pos) = self.positions.get(&id) else {
            return false;
        };

        let new_width = module.width();
        let new_height = module.height();

        self.grid.remove_module(id);

        for dy in 0..new_height {
            for dx in 0..new_width {
                let p = GridPos::new(pos.x + dx as u16, pos.y + dy as u16);
                if !self.grid.in_bounds(p) || !self.grid.get(p).is_empty() {
                    self.grid.place_module(id, pos, 1, 1);
                    self.rebuild_channels();
                    return false;
                }
            }
        }

        self.grid.place_module(id, pos, new_width, new_height);
        self.rebuild_channels();
        true
    }

    pub fn rebuild_channels(&mut self) {
        self.grid.clear_channels();

        let module_data: Vec<_> = self
            .modules
            .iter()
            .filter_map(|(id, m)| self.positions.get(id).map(|pos| (*id, *pos, m.clone())))
            .collect();

        for (id, pos, module) in &module_data {
            let color = module.kind.color();
            let width = module.width();
            let height = module.height();
            let bottom_y = pos.y + height as u16;
            let right_x = pos.x + width as u16;

            if module.has_output_bottom() {
                for target_y in bottom_y..self.grid.height() {
                    let target_pos = GridPos::new(pos.x, target_y);
                    if let Cell::Module {
                        id: target_id,
                        local_x,
                        local_y,
                    } = self.grid.get(target_pos)
                    {
                        if target_id != *id {
                            if let Some(target_mod) = self.modules.get(&target_id) {
                                if local_y == 0
                                    && target_mod.has_input_top()
                                    && target_mod.is_port_open(local_x as usize)
                                {
                                    for y in bottom_y..target_y {
                                        let p = GridPos::new(pos.x, y);
                                        match self.grid.get(p) {
                                            Cell::Empty => {
                                                self.grid.set(p, Cell::ChannelV { color })
                                            }
                                            Cell::ChannelH { color: color_h } => {
                                                self.grid.set(
                                                    p,
                                                    Cell::ChannelCross {
                                                        color_v: color,
                                                        color_h,
                                                    },
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }

            if module.has_output_right() {
                let out_y = pos.y;
                for target_x in right_x..self.grid.width() {
                    let target_pos = GridPos::new(target_x, out_y);
                    if let Cell::Module {
                        id: target_id,
                        local_x,
                        local_y,
                    } = self.grid.get(target_pos)
                    {
                        if target_id != *id {
                            if let Some(target_mod) = self.modules.get(&target_id) {
                                if local_x == 0
                                    && target_mod.has_input_left()
                                    && target_mod.is_port_open(local_y as usize)
                                {
                                    for x in right_x..target_x {
                                        let p = GridPos::new(x, out_y);
                                        match self.grid.get(p) {
                                            Cell::Empty => {
                                                self.grid.set(p, Cell::ChannelH { color })
                                            }
                                            Cell::ChannelV { color: color_v } => {
                                                self.grid.set(
                                                    p,
                                                    Cell::ChannelCross {
                                                        color_v,
                                                        color_h: color,
                                                    },
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::new(24, 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_module() {
        let mut patch = Patch::new(10, 10);
        let id = patch.add_module(
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );
        assert!(id.is_some());
        assert!(patch.remove_module(id.unwrap()));
    }

    #[test]
    fn test_move_module() {
        let mut patch = Patch::new(10, 10);
        let id = patch
            .add_module(
                ModuleKind::Standard(StandardModule::Freq),
                GridPos::new(0, 0),
            )
            .unwrap();
        assert!(patch.move_module(id, GridPos::new(2, 2)));
        assert_eq!(patch.module_position(id), Some(GridPos::new(2, 2)));
    }
}
