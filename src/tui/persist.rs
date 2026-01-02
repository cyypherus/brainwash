use super::grid::GridPos;
use super::module::{ModuleKind, ModuleParams, Orientation, SubPatchId};
use super::patch::{Patch, PatchSet, SubPatchDef};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchFile {
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    #[serde(default = "default_bars")]
    pub bars: f32,
    #[serde(default)]
    pub scale_idx: usize,
    #[serde(default)]
    pub modules: Vec<ModuleDef>,
    #[serde(default)]
    pub track: Option<String>,
    #[serde(default)]
    pub subpatches: Vec<SubPatchFileDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubPatchFileDef {
    pub id: u32,
    pub name: String,
    pub color: (u8, u8, u8),
    pub modules: Vec<ModuleDef>,
}

fn default_bpm() -> f32 {
    120.0
}
fn default_bars() -> f32 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDef {
    pub id: u32,
    pub kind: ModuleKind,
    pub x: u16,
    pub y: u16,
    #[serde(default, skip_serializing_if = "is_horizontal")]
    pub orientation: Orientation,
    pub params: ModuleParams,
}

fn is_horizontal(o: &Orientation) -> bool {
    *o == Orientation::Horizontal
}

impl PatchFile {
    pub fn new() -> Self {
        Self {
            bpm: 120.0,
            bars: 1.0,
            scale_idx: 0,
            modules: Vec::new(),
            track: None,
            subpatches: Vec::new(),
        }
    }
}

impl Default for PatchFile {
    fn default() -> Self {
        Self::new()
    }
}

fn patch_to_modules(patch: &Patch) -> Vec<ModuleDef> {
    patch
        .all_modules()
        .map(|module| {
            let pos = patch
                .module_position(module.id)
                .unwrap_or(GridPos::new(0, 0));
            ModuleDef {
                id: module.id.0,
                kind: module.kind,
                x: pos.x,
                y: pos.y,
                orientation: module.orientation,
                params: module.params.clone(),
            }
        })
        .collect()
}

fn modules_to_patch(modules: &[ModuleDef], width: u16, height: u16) -> Patch {
    use super::module::ModuleId;
    
    let mut patch = Patch::new(width, height);
    let mut id_map: HashMap<u32, ModuleId> = HashMap::new();
    
    for mdef in modules {
        let pos = GridPos::new(mdef.x, mdef.y);
        if let Some(new_id) = patch.add_module(mdef.kind, pos) {
            id_map.insert(mdef.id, new_id);
            if let Some(module) = patch.module_mut(new_id) {
                module.orientation = mdef.orientation;
                module.params = mdef.params.clone();
            }
        }
    }
    
    let tap_updates: Vec<_> = patch.all_modules()
        .filter_map(|m| {
            if let ModuleKind::DelayTap(old_delay_id) = m.kind {
                let new_delay_id = id_map.get(&old_delay_id.0).copied()?;
                Some((m.id, new_delay_id))
            } else {
                None
            }
        })
        .collect();
    
    for (tap_id, new_delay_id) in tap_updates {
        if let Some(m) = patch.module_mut(tap_id) {
            m.kind = ModuleKind::DelayTap(new_delay_id);
        }
    }
    
    patch.rebuild_channels();
    patch
}

fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (255, 150, 50),
    }
}

pub fn patchset_to_file(patches: &PatchSet, bpm: f32, bars: f32, scale_idx: usize, track: Option<&str>) -> PatchFile {
    let mut pf = PatchFile::new();
    pf.bpm = bpm;
    pf.bars = bars;
    pf.scale_idx = scale_idx;
    pf.track = track.map(|s| s.to_string());
    pf.modules = patch_to_modules(&patches.root);

    for (id, sub) in &patches.subpatches {
        pf.subpatches.push(SubPatchFileDef {
            id: id.0,
            name: sub.name.clone(),
            color: color_to_rgb(sub.color),
            modules: patch_to_modules(&sub.patch),
        });
    }

    pf
}

pub fn file_to_patchset(pf: &PatchFile) -> (PatchSet, f32, f32, usize, Option<String>) {
    let root = modules_to_patch(&pf.modules, 32, 32);

    let mut subpatches = HashMap::new();
    let mut max_id = 0u32;

    for sub in &pf.subpatches {
        let id = SubPatchId(sub.id);
        max_id = max_id.max(sub.id);

        let (r, g, b) = sub.color;
        let mut def = SubPatchDef::new(sub.name.clone(), Color::Rgb(r, g, b));
        def.patch = modules_to_patch(&sub.modules, 10, 10);
        subpatches.insert(id, def);
    }

    let patches = PatchSet {
        root,
        subpatches,
        next_subpatch_id: max_id + 1,
    };

    (patches, pf.bpm, pf.bars, pf.scale_idx, pf.track.clone())
}

pub fn save_patchset(
    path: &Path,
    patches: &PatchSet,
    bpm: f32,
    bars: f32,
    scale_idx: usize,
    track: Option<&str>,
) -> io::Result<()> {
    let pf = patchset_to_file(patches, bpm, bars, scale_idx, track);
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .indentor("  ".to_string());
    let content = ron::ser::to_string_pretty(&pf, config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, content)
}

pub fn load_patchset(path: &Path) -> io::Result<(PatchSet, f32, f32, usize, Option<String>)> {
    let content = fs::read_to_string(path)?;
    let pf: PatchFile =
        ron::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(file_to_patchset(&pf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::module::EnvPoint;

    #[test]
    fn test_ron_format() {
        let mut patches = PatchSet::new(20, 20);
        patches.root.add_module(ModuleKind::Freq, GridPos::new(0, 0));
        let osc_id = patches.root.add_module(ModuleKind::Osc, GridPos::new(0, 2)).unwrap();
        patches.root.add_module(ModuleKind::Output, GridPos::new(0, 7));

        if let Some(osc) = patches.root.module_mut(osc_id) {
            osc.params = ModuleParams::Osc {
                wave: crate::tui::module::WaveType::Saw,
                freq: 440.0,
                shift: 0.0,
                gain: 1.0,
                uni: true,
                connected: 0xFF,
            };
        }

        let pf = patchset_to_file(&patches, 120.0, 1.0, 0, Some("C4 D4 E4"));
        let config = ron::ser::PrettyConfig::new().depth_limit(4).indentor("  ".to_string());
        println!("{}", ron::ser::to_string_pretty(&pf, config).unwrap());
    }

    #[test]
    fn test_roundtrip() {
        let mut patches = PatchSet::new(20, 20);

        let _freq_id = patches.root
            .add_module(ModuleKind::Freq, GridPos::new(0, 0))
            .unwrap();
        let osc_id = patches.root
            .add_module(ModuleKind::Osc, GridPos::new(0, 2))
            .unwrap();
        let env_id = patches.root
            .add_module(ModuleKind::Envelope, GridPos::new(2, 0))
            .unwrap();
        let _out_id = patches.root
            .add_module(ModuleKind::Output, GridPos::new(0, 7))
            .unwrap();

        if let Some(osc) = patches.root.module_mut(osc_id) {
            osc.orientation = Orientation::Vertical;
            osc.params = ModuleParams::Osc {
                wave: crate::tui::module::WaveType::Saw,
                freq: 660.0,
                shift: 12.0,
                gain: 0.8,
                uni: true,
                connected: 0x05,
            };
        }

        if let Some(env) = patches.root.module_mut(env_id) {
            if let Some(pts) = env.params.env_points_mut() {
                *pts = vec![
                    EnvPoint {
                        time: 0.0,
                        value: 0.0,
                        curve: false,
                    },
                    EnvPoint {
                        time: 0.5,
                        value: 1.0,
                        curve: true,
                    },
                    EnvPoint {
                        time: 1.0,
                        value: 0.0,
                        curve: false,
                    },
                ];
            }
        }

        let pf = patchset_to_file(&patches, 90.0, 4.0, 5, Some("C4 E4 G4\n# comment\nD4"));

        let serialized = ron::ser::to_string_pretty(&pf, ron::ser::PrettyConfig::default()).unwrap();
        let pf2: PatchFile = ron::from_str(&serialized).unwrap();

        assert!((pf2.bpm - 90.0).abs() < 0.01);
        assert!((pf2.bars - 4.0).abs() < 0.01);
        assert_eq!(pf2.scale_idx, 5);
        assert_eq!(pf2.modules.len(), 4);
        assert!(pf2.track.as_ref().unwrap().contains("# comment"));

        let (patches2, bpm2, bars2, scale_idx2, track2) = file_to_patchset(&pf2);
        assert_eq!(scale_idx2, 5);

        assert!((bpm2 - 90.0).abs() < 0.01);
        assert!((bars2 - 4.0).abs() < 0.01);
        assert!(track2.unwrap().contains("# comment"));

        let modules: Vec<_> = patches2.root.all_modules().collect();
        assert_eq!(modules.len(), 4);

        let osc2 = modules.iter().find(|m| m.kind == ModuleKind::Osc).unwrap();
        assert_eq!(osc2.orientation, Orientation::Vertical);
        if let ModuleParams::Osc { wave, freq, shift, gain, uni, connected } = &osc2.params {
            assert_eq!(*wave, crate::tui::module::WaveType::Saw);
            assert!((freq - 660.0).abs() < 0.01);
            assert!((shift - 12.0).abs() < 0.01);
            assert!((gain - 0.8).abs() < 0.01);
            assert!(*uni);
            assert_eq!(*connected, 0x05);
        } else {
            panic!("Expected Osc params");
        }

        let env2 = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Envelope)
            .unwrap();
        assert_eq!(env2.params.env_points().unwrap().len(), 3);
        assert!(env2.params.env_points().unwrap()[1].curve);
    }
}
