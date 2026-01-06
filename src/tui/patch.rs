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
    patches: HashMap<Option<SubPatchId>, SubPatchDef>,
    next_subpatch_id: u32,
    next_module_id: u32,
}

impl PatchSet {
    pub fn new(width: u16, height: u16) -> Self {
        let mut patches = HashMap::new();
        let mut root_def = SubPatchDef::new("Root".into(), Color::White);
        root_def.patch = Patch::new(width, height);
        patches.insert(None, root_def);
        Self {
            patches,
            next_subpatch_id: 0,
            next_module_id: 0,
        }
    }

    pub fn alloc_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_module_id);
        self.next_module_id += 1;
        id
    }

    pub fn patch(&self, id: Option<SubPatchId>) -> Option<&Patch> {
        self.patches.get(&id).map(|def| &def.patch)
    }

    pub fn patch_mut(&mut self, id: Option<SubPatchId>) -> Option<&mut Patch> {
        self.patches.get_mut(&id).map(|def| &mut def.patch)
    }

    pub fn root(&self) -> &Patch {
        &self.patches.get(&None).unwrap().patch
    }

    pub fn root_mut(&mut self) -> &mut Patch {
        &mut self.patches.get_mut(&None).unwrap().patch
    }

    pub fn create_subpatch(&mut self, name: String, color: Color) -> SubPatchId {
        let id = SubPatchId(self.next_subpatch_id);
        self.next_subpatch_id += 1;
        self.patches.insert(Some(id), SubPatchDef::new(name, color));
        id
    }

    pub fn subpatch(&self, id: SubPatchId) -> Option<&SubPatchDef> {
        self.patches.get(&Some(id))
    }

    pub fn subpatch_mut(&mut self, id: SubPatchId) -> Option<&mut SubPatchDef> {
        self.patches.get_mut(&Some(id))
    }

    pub fn subpatches(&self) -> impl Iterator<Item = (SubPatchId, &SubPatchDef)> {
        self.patches.iter().filter_map(|(k, v)| k.map(|id| (id, v)))
    }

    pub fn subpatch_count(&self) -> usize {
        self.patches.len() - 1
    }

    pub fn set_root(&mut self, patch: Patch) {
        if let Some(def) = self.patches.get_mut(&None) {
            def.patch = patch;
        }
    }

    pub fn insert_subpatch(&mut self, id: SubPatchId, def: SubPatchDef) {
        self.patches.insert(Some(id), def);
        if id.0 >= self.next_subpatch_id {
            self.next_subpatch_id = id.0 + 1;
        }
    }

    pub fn set_next_module_id(&mut self, id: u32) {
        self.next_module_id = id;
    }

    pub fn move_module(
        &mut self,
        from_patch: Option<SubPatchId>,
        to_patch: Option<SubPatchId>,
        module_id: ModuleId,
        to_pos: GridPos,
    ) -> bool {
        let Some(from) = self.patches.get_mut(&from_patch) else {
            return false;
        };
        let Some((module, _from_pos)) = from.patch.extract_module(module_id) else {
            return false;
        };

        let Some(to) = self.patches.get_mut(&to_patch) else {
            if let Some(from) = self.patches.get_mut(&from_patch) {
                from.patch.insert_module(module, _from_pos);
            }
            return false;
        };

        if !to.patch.insert_module(module.clone(), to_pos) {
            if let Some(from) = self.patches.get_mut(&from_patch) {
                from.patch.insert_module(module, _from_pos);
            }
            return false;
        }

        true
    }

    pub fn extract_module_from(
        &mut self,
        patch_id: Option<SubPatchId>,
        module_id: ModuleId,
    ) -> Option<Module> {
        let patch = self.patches.get_mut(&patch_id)?;
        patch.patch.extract_module(module_id).map(|(m, _)| m)
    }

    pub fn insert_module_into(
        &mut self,
        patch_id: Option<SubPatchId>,
        module: Module,
        pos: GridPos,
    ) -> bool {
        let Some(patch) = self.patches.get_mut(&patch_id) else {
            return false;
        };
        patch.patch.insert_module(module, pos)
    }

    pub fn add_module(
        &mut self,
        patch_id: Option<SubPatchId>,
        module: Module,
        pos: GridPos,
    ) -> bool {
        let Some(patch) = self.patches.get_mut(&patch_id) else {
            return false;
        };
        patch.patch.insert_module(module, pos)
    }
}

#[derive(Clone)]
pub struct Patch {
    grid: Grid,
    modules: HashMap<ModuleId, Module>,
    positions: HashMap<ModuleId, GridPos>,
    output_id: Option<ModuleId>,
}

impl Patch {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            grid: Grid::new(width, height),
            modules: HashMap::new(),
            positions: HashMap::new(),
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

    pub fn insert_module(&mut self, module: Module, pos: GridPos) -> bool {
        if module.kind == ModuleKind::Standard(StandardModule::Output) && self.output_id.is_some() {
            return false;
        }

        let id = module.id;
        let width = module.width();
        let height = module.height();

        for dy in 0..height {
            for dx in 0..width {
                let p = GridPos::new(pos.x + dx as u16, pos.y + dy as u16);
                if !self.grid.in_bounds(p) {
                    return false;
                }
            }
        }

        if module.kind == ModuleKind::Standard(StandardModule::Output) {
            self.output_id = Some(id);
        }

        self.modules.insert(id, module);
        self.positions.insert(id, pos);
        self.update_disabled_states();
        self.rebuild_channels();
        true
    }

    pub fn extract_module(&mut self, id: ModuleId) -> Option<(Module, GridPos)> {
        let module = self.modules.remove(&id)?;
        let pos = self.positions.remove(&id)?;

        if self.output_id == Some(id) {
            self.output_id = None;
        }

        self.update_disabled_states();
        self.rebuild_channels();
        Some((module, pos))
    }

    pub fn remove_module(&mut self, id: ModuleId) -> bool {
        if !self.modules.contains_key(&id) {
            return false;
        }

        self.positions.remove(&id);

        if self.output_id == Some(id) {
            self.output_id = None;
        }

        self.modules.remove(&id);
        self.update_disabled_states();
        self.rebuild_channels();
        true
    }

    pub fn move_module(&mut self, id: ModuleId, new_pos: GridPos) -> bool {
        let Some(module) = self.modules.get(&id) else {
            return false;
        };
        let width = module.width();
        let height = module.height();

        for dy in 0..height {
            for dx in 0..width {
                let p = GridPos::new(new_pos.x + dx as u16, new_pos.y + dy as u16);
                if !self.grid.in_bounds(p) {
                    return false;
                }
            }
        }

        self.positions.insert(id, new_pos);
        self.update_disabled_states();
        self.rebuild_channels();
        true
    }

    pub fn move_modules(&mut self, moves: &[(ModuleId, GridPos)]) -> usize {
        let module_data: Vec<(ModuleId, GridPos, u8, u8)> = moves
            .iter()
            .filter_map(|(id, new_pos)| {
                let module = self.modules.get(id)?;
                Some((*id, *new_pos, module.width(), module.height()))
            })
            .collect();

        for &(_, new_pos, width, height) in &module_data {
            for dy in 0..height {
                for dx in 0..width {
                    let p = GridPos::new(new_pos.x + dx as u16, new_pos.y + dy as u16);
                    if !self.grid.in_bounds(p) {
                        return 0;
                    }
                }
            }
        }

        for &(id, new_pos, _, _) in &module_data {
            self.positions.insert(id, new_pos);
        }

        self.update_disabled_states();
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

        for dy in 0..new_height {
            for dx in 0..new_width {
                let p = GridPos::new(pos.x + dx as u16, pos.y + dy as u16);
                if !self.grid.in_bounds(p) {
                    return false;
                }
            }
        }

        self.modules.get_mut(&id).unwrap().rotate();
        self.update_disabled_states();
        self.rebuild_channels();
        true
    }

    pub fn refit_module(&mut self) {
        self.update_disabled_states();
        self.rebuild_channels();
    }

    pub fn update_disabled_states(&mut self) {
        let mut overlapping: std::collections::HashSet<ModuleId> = std::collections::HashSet::new();

        let module_bounds: Vec<_> = self
            .modules
            .iter()
            .filter_map(|(&id, m)| {
                self.positions
                    .get(&id)
                    .map(|&pos| (id, pos, m.width(), m.height()))
            })
            .collect();

        for (i, &(id_a, pos_a, w_a, h_a)) in module_bounds.iter().enumerate() {
            for &(id_b, pos_b, w_b, h_b) in &module_bounds[i + 1..] {
                let overlap_x = pos_a.x < pos_b.x + w_b as u16 && pos_b.x < pos_a.x + w_a as u16;
                let overlap_y = pos_a.y < pos_b.y + h_b as u16 && pos_b.y < pos_a.y + h_a as u16;

                if overlap_x && overlap_y {
                    overlapping.insert(id_a);
                    overlapping.insert(id_b);
                }
            }
        }

        for module in self.modules.values_mut() {
            module.disabled = overlapping.contains(&module.id);
        }
    }

    pub fn has_disabled_modules(&self) -> bool {
        self.modules.values().any(|m| m.disabled)
    }

    pub fn rebuild_grid(&mut self) {
        self.grid.clear_all();

        for (&id, module) in &self.modules {
            let Some(&pos) = self.positions.get(&id) else {
                continue;
            };
            self.grid
                .place_module(id, pos, module.width(), module.height());
        }
    }

    pub fn rebuild_channels(&mut self) {
        self.rebuild_grid();

        let module_data: Vec<_> = self
            .modules
            .iter()
            .filter(|(_, m)| !m.disabled)
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
                        && target_id != *id
                    {
                        if let Some(target_mod) = self.modules.get(&target_id)
                            && !target_mod.disabled
                            && local_y == 0
                            && target_mod.has_input_top()
                            && target_mod.is_port_open(local_x as usize)
                        {
                            for y in bottom_y..target_y {
                                let p = GridPos::new(pos.x, y);
                                match self.grid.get(p) {
                                    Cell::Empty => self.grid.set(p, Cell::ChannelV { color }),
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
                        break;
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
                        && target_id != *id
                    {
                        if let Some(target_mod) = self.modules.get(&target_id)
                            && !target_mod.disabled
                            && local_x == 0
                            && target_mod.has_input_left()
                            && target_mod.is_port_open(local_y as usize)
                        {
                            for x in right_x..target_x {
                                let p = GridPos::new(x, out_y);
                                match self.grid.get(p) {
                                    Cell::Empty => self.grid.set(p, Cell::ChannelH { color }),
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
                        break;
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
        let mut patches = PatchSet::new(10, 10);
        let id = patches.alloc_module_id();
        patches.add_module(
            None,
            Module::new(id, ModuleKind::Standard(StandardModule::Freq)),
            GridPos::new(0, 0),
        );
        assert!(patches.root_mut().remove_module(id));
    }

    #[test]
    fn test_move_module() {
        let mut patches = PatchSet::new(10, 10);
        let id = patches.alloc_module_id();
        patches.add_module(
            None,
            Module::new(id, ModuleKind::Standard(StandardModule::Freq)),
            GridPos::new(0, 0),
        );
        assert!(patches.root_mut().move_module(id, GridPos::new(2, 2)));
        assert_eq!(patches.root().module_position(id), Some(GridPos::new(2, 2)));
    }
}
