use super::grid::GridPos;
use super::module::{ModuleKind, ModuleParams, Orientation, StandardModule, SubPatchId};
use super::patch::{Patch, PatchSet, SubPatchDef};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

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

fn load_wav_samples(path: &str) -> Option<Arc<Vec<f32>>> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .step_by(spec.channels as usize)
            .collect(),
        hound::SampleFormat::Int => {
            let max = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(|s| s.ok())
                .step_by(spec.channels as usize)
                .map(|s| s as f32 / max)
                .collect()
        }
    };
    Some(Arc::new(samples))
}

fn reload_samples_in_patch(patch: &mut Patch) -> Vec<String> {
    let mut missing = Vec::new();
    let ids: Vec<_> = patch.all_modules().map(|m| m.id).collect();
    for id in ids {
        if let Some(m) = patch.module_mut(id)
            && let ModuleParams::Sample {
                file_name, samples, ..
            } = &mut m.params
            && !file_name.is_empty()
        {
            if let Some(loaded) = load_wav_samples(file_name) {
                *samples = loaded;
            } else {
                missing.push(file_name.clone());
            }
        }
    }
    missing
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

fn modules_to_patch(modules: &[ModuleDef], width: u16, height: u16, next_id: &mut u32) -> Patch {
    use super::module::ModuleId;

    let mut patch = Patch::new(width, height);
    let mut id_map: HashMap<u32, ModuleId> = HashMap::new();

    for mdef in modules {
        let pos = GridPos::new(mdef.x, mdef.y);
        let new_id = ModuleId(*next_id);
        *next_id += 1;
        if patch.add_module(new_id, mdef.kind, pos) {
            id_map.insert(mdef.id, new_id);
            if let Some(module) = patch.module_mut(new_id) {
                module.orientation = mdef.orientation;
                module.params = mdef.params.clone();
            }
        }
    }

    let tap_updates: Vec<_> = patch
        .all_modules()
        .filter_map(|m| {
            if let ModuleKind::Standard(StandardModule::DelayTap(old_delay_id)) = m.kind {
                let new_delay_id = id_map.get(&old_delay_id.0).copied()?;
                Some((m.id, new_delay_id))
            } else {
                None
            }
        })
        .collect();

    for (tap_id, new_delay_id) in tap_updates {
        if let Some(m) = patch.module_mut(tap_id) {
            m.kind = ModuleKind::Standard(StandardModule::DelayTap(new_delay_id));
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

pub fn patchset_to_file(
    patches: &PatchSet,
    bpm: f32,
    bars: f32,
    scale_idx: usize,
    track: Option<&str>,
) -> PatchFile {
    let mut pf = PatchFile::new();
    pf.bpm = bpm;
    pf.bars = bars;
    pf.scale_idx = scale_idx;
    pf.track = track.map(|s| s.to_string());
    pf.modules = patch_to_modules(patches.root());

    for (id, sub) in patches.subpatches() {
        pf.subpatches.push(SubPatchFileDef {
            id: id.0,
            name: sub.name.clone(),
            color: color_to_rgb(sub.color),
            modules: patch_to_modules(&sub.patch),
        });
    }

    pf
}

pub struct LoadResult {
    pub patches: PatchSet,
    pub bpm: f32,
    pub bars: f32,
    pub scale_idx: usize,
    pub track: Option<String>,
    pub missing_samples: Vec<String>,
}

pub fn file_to_patchset(pf: &PatchFile) -> LoadResult {
    let mut next_module_id = 0u32;

    let mut root = modules_to_patch(&pf.modules, 41, 21, &mut next_module_id);
    let mut missing_samples = reload_samples_in_patch(&mut root);

    let mut patches = PatchSet::new(41, 21);
    patches.set_root(root);

    for sub in &pf.subpatches {
        let id = SubPatchId(sub.id);

        let (r, g, b) = sub.color;
        let mut def = SubPatchDef::new(sub.name.clone(), Color::Rgb(r, g, b));
        def.patch = modules_to_patch(&sub.modules, 10, 10, &mut next_module_id);
        missing_samples.extend(reload_samples_in_patch(&mut def.patch));
        patches.insert_subpatch(id, def);
    }

    patches.set_next_module_id(next_module_id);

    LoadResult {
        patches,
        bpm: pf.bpm,
        bars: pf.bars,
        scale_idx: pf.scale_idx,
        track: pf.track.clone(),
        missing_samples,
    }
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

pub fn load_patchset(path: &Path) -> io::Result<LoadResult> {
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
        let id = patches.alloc_module_id();
        patches.root_mut().add_module(
            id,
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );
        let osc_id = patches.alloc_module_id();
        patches.root_mut().add_module(
            osc_id,
            ModuleKind::Standard(StandardModule::Osc),
            GridPos::new(0, 2),
        );
        let id = patches.alloc_module_id();
        patches.root_mut().add_module(
            id,
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(0, 7),
        );

        if let Some(osc) = patches.root_mut().module_mut(osc_id) {
            osc.params = ModuleParams::Osc {
                wave: crate::tui::module::WaveType::Saw,
                freq: crate::tui::module::TimeValue::from_hz(440.0),
                shift: 0.0,
                gain: 1.0,
                uni: true,
                connected: 0xFF,
            };
        }

        let pf = patchset_to_file(&patches, 120.0, 1.0, 0, Some("C4 D4 E4"));
        let config = ron::ser::PrettyConfig::new()
            .depth_limit(4)
            .indentor("  ".to_string());
        println!("{}", ron::ser::to_string_pretty(&pf, config).unwrap());
    }

    #[test]
    fn test_roundtrip() {
        let mut patches = PatchSet::new(20, 20);

        let _freq_id = patches.alloc_module_id();
        patches.root_mut().add_module(
            _freq_id,
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );
        let osc_id = patches.alloc_module_id();
        patches.root_mut().add_module(
            osc_id,
            ModuleKind::Standard(StandardModule::Osc),
            GridPos::new(0, 2),
        );
        let env_id = patches.alloc_module_id();
        patches.root_mut().add_module(
            env_id,
            ModuleKind::Standard(StandardModule::Envelope),
            GridPos::new(2, 0),
        );
        let _out_id = patches.alloc_module_id();
        patches.root_mut().add_module(
            _out_id,
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(0, 7),
        );

        if let Some(osc) = patches.root_mut().module_mut(osc_id) {
            osc.orientation = Orientation::Vertical;
            osc.params = ModuleParams::Osc {
                wave: crate::tui::module::WaveType::Saw,
                freq: crate::tui::module::TimeValue::from_hz(660.0),
                shift: 12.0,
                gain: 0.8,
                uni: true,
                connected: 0x05,
            };
        }

        if let Some(env) = patches.root_mut().module_mut(env_id) {
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

        let serialized =
            ron::ser::to_string_pretty(&pf, ron::ser::PrettyConfig::default()).unwrap();
        let pf2: PatchFile = ron::from_str(&serialized).unwrap();

        assert!((pf2.bpm - 90.0).abs() < 0.01);
        assert!((pf2.bars - 4.0).abs() < 0.01);
        assert_eq!(pf2.scale_idx, 5);
        assert_eq!(pf2.modules.len(), 4);
        assert!(pf2.track.as_ref().unwrap().contains("# comment"));

        let result = file_to_patchset(&pf2);
        assert_eq!(result.scale_idx, 5);

        assert!((result.bpm - 90.0).abs() < 0.01);
        assert!((result.bars - 4.0).abs() < 0.01);
        assert!(result.track.unwrap().contains("# comment"));

        let modules: Vec<_> = result.patches.root().all_modules().collect();
        assert_eq!(modules.len(), 4);

        let osc2 = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Standard(StandardModule::Osc))
            .unwrap();
        assert_eq!(osc2.orientation, Orientation::Vertical);
        if let ModuleParams::Osc {
            wave,
            freq,
            shift,
            gain,
            uni,
            connected,
        } = &osc2.params
        {
            assert_eq!(*wave, crate::tui::module::WaveType::Saw);
            assert!((freq.hz - 660.0).abs() < 0.01);
            assert!((shift - 12.0).abs() < 0.01);
            assert!((gain - 0.8).abs() < 0.01);
            assert!(*uni);
            assert_eq!(*connected, 0x05);
        } else {
            panic!("Expected Osc params");
        }

        let env2 = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Standard(StandardModule::Envelope))
            .unwrap();
        assert_eq!(env2.params.env_points().unwrap().len(), 3);
        assert!(env2.params.env_points().unwrap()[1].curve);
    }

    #[test]
    fn test_subpatch_serialization() {
        use crate::tui::module::{RoutingModule, SubpatchModule};
        use ratatui::style::Color;

        let mut patches = PatchSet::new(20, 20);

        let id = patches.alloc_module_id();
        patches.root_mut().add_module(
            id,
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );

        let sub_id = patches.create_subpatch("TestSub".into(), Color::Rgb(100, 150, 200));
        let id1 = patches.alloc_module_id();
        let id2 = patches.alloc_module_id();
        let id3 = patches.alloc_module_id();

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(
                id1,
                ModuleKind::Subpatch(SubpatchModule::SubIn),
                GridPos::new(0, 0),
            );
            sub.patch.add_module(
                id2,
                ModuleKind::Standard(StandardModule::Mul),
                GridPos::new(1, 0),
            );
            sub.patch.add_module(
                id3,
                ModuleKind::Subpatch(SubpatchModule::SubOut),
                GridPos::new(2, 0),
            );
        }

        let id = patches.alloc_module_id();
        patches.root_mut().add_module(
            id,
            ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)),
            GridPos::new(1, 0),
        );
        let id = patches.alloc_module_id();
        patches.root_mut().add_module(
            id,
            ModuleKind::Routing(RoutingModule::LSplit),
            GridPos::new(3, 0),
        );
        let id = patches.alloc_module_id();
        patches.root_mut().add_module(
            id,
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(5, 0),
        );

        let pf = patchset_to_file(&patches, 120.0, 1.0, 0, None);

        let serialized =
            ron::ser::to_string_pretty(&pf, ron::ser::PrettyConfig::default()).unwrap();

        let pf2: PatchFile = ron::from_str(&serialized).unwrap();
        let result = file_to_patchset(&pf2);

        assert_eq!(result.patches.root().all_modules().count(), 4);
        assert_eq!(result.patches.subpatch_count(), 1);

        let sub = result.patches.subpatch(sub_id).unwrap();
        assert_eq!(sub.patch.all_modules().count(), 3);
        assert!(
            sub.patch
                .all_modules()
                .any(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubIn))
        );
        assert!(
            sub.patch
                .all_modules()
                .any(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubOut))
        );
    }
}
