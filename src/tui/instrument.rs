use super::grid::GridPos;
use super::module::{ModuleId, ModuleKind, StandardModule, SubPatchId};
use super::patch::PatchSet;
use std::collections::{HashMap, VecDeque};

pub struct Instrument {
    pub patches: PatchSet,
    pub track_text: String,
    pub scale_idx: usize,
    pub cursor: GridPos,
    pub view_center: GridPos,
    pub editing_subpatch: Option<SubPatchId>,
    pub subpatch_stack: Vec<(Option<SubPatchId>, GridPos)>,
    pub undo_stack: Vec<PatchSet>,
    pub redo_stack: Vec<PatchSet>,
    pub meter_values: HashMap<ModuleId, Vec<f32>>,
    pub probe_values: HashMap<ModuleId, f32>,
    pub probe_histories: HashMap<ModuleId, VecDeque<f32>>,
}

impl Instrument {
    pub fn new() -> Self {
        let mut patches = PatchSet::new(41, 21);
        let mod_id = patches.alloc_module_id();
        patches.root_mut().add_module(
            mod_id,
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(40, 20),
        );

        let track_text = r#"
# _ = rest
# 0+ = sharp, 0- = flat
# 0* = 2x weight, 0** = 3x weight
# (0/_/2) = bar with divisions
# ((0/1)/(2/3/4)) = nested divisions
# {0&2} = polyphony
# ({0&2&4}/{1&3&5}) = two chords in sequence
(0/2/4/7)
"#
        .to_string();

        Self {
            patches,
            track_text,
            scale_idx: 2,
            cursor: GridPos::new(0, 0),
            view_center: GridPos::new(0, 0),
            editing_subpatch: None,
            subpatch_stack: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            meter_values: HashMap::new(),
            probe_values: HashMap::new(),
            probe_histories: HashMap::new(),
        }
    }

    pub fn snapshot(&mut self) {
        self.undo_stack.push(self.patches.clone());
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }
}

impl Default for Instrument {
    fn default() -> Self {
        Self::new()
    }
}
