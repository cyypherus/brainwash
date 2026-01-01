use super::grid::GridPos;
use super::module::{ModuleKind, ModuleParams, Orientation};
use super::patch::Patch;
use serde::{Deserialize, Serialize};
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
    pub modules: Vec<ModuleDef>,
    #[serde(default)]
    pub track: Option<String>,
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
            modules: Vec::new(),
            track: None,
        }
    }
}

impl Default for PatchFile {
    fn default() -> Self {
        Self::new()
    }
}

pub fn patch_to_file(patch: &Patch, bpm: f32, bars: f32, track: Option<&str>) -> PatchFile {
    let mut pf = PatchFile::new();
    pf.bpm = bpm;
    pf.bars = bars;
    pf.track = track.map(|s| s.to_string());

    for module in patch.all_modules() {
        let pos = patch
            .module_position(module.id)
            .unwrap_or(GridPos::new(0, 0));

        pf.modules.push(ModuleDef {
            id: module.id.0,
            kind: module.kind,
            x: pos.x,
            y: pos.y,
            orientation: module.orientation,
            params: module.params.clone(),
        });
    }

    pf
}

pub fn file_to_patch(pf: &PatchFile) -> (Patch, f32, f32, Option<String>) {
    let mut patch = Patch::new(32, 32);

    for mdef in &pf.modules {
        let pos = GridPos::new(mdef.x, mdef.y);

        if let Some(id) = patch.add_module(mdef.kind, pos) {
            if let Some(module) = patch.module_mut(id) {
                module.orientation = mdef.orientation;
                module.params = mdef.params.clone();
            }
        }
    }

    patch.rebuild_channels();

    (patch, pf.bpm, pf.bars, pf.track.clone())
}

pub fn save_patch(
    path: &Path,
    patch: &Patch,
    bpm: f32,
    bars: f32,
    track: Option<&str>,
) -> io::Result<()> {
    let pf = patch_to_file(patch, bpm, bars, track);
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(4)
        .indentor("  ".to_string());
    let content = ron::ser::to_string_pretty(&pf, config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, content)
}

pub fn load_patch(path: &Path) -> io::Result<(Patch, f32, f32, Option<String>)> {
    let content = fs::read_to_string(path)?;
    let pf: PatchFile =
        ron::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(file_to_patch(&pf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::module::EnvPoint;

    #[test]
    fn test_ron_format() {
        let mut patch = Patch::new(20, 20);
        patch.add_module(ModuleKind::Freq, GridPos::new(0, 0));
        let osc_id = patch.add_module(ModuleKind::Osc, GridPos::new(0, 2)).unwrap();
        patch.add_module(ModuleKind::Output, GridPos::new(0, 7));

        if let Some(osc) = patch.module_mut(osc_id) {
            osc.params = ModuleParams::Osc {
                wave: crate::tui::module::WaveType::Saw,
                freq: 440.0,
                shift: 0.0,
                gain: 1.0,
                uni: true,
                connected: 0xFF,
            };
        }

        let pf = patch_to_file(&patch, 120.0, 1.0, Some("C4 D4 E4"));
        let config = ron::ser::PrettyConfig::new().depth_limit(4).indentor("  ".to_string());
        println!("{}", ron::ser::to_string_pretty(&pf, config).unwrap());
    }

    #[test]
    fn test_roundtrip() {
        let mut patch = Patch::new(20, 20);

        let _freq_id = patch
            .add_module(ModuleKind::Freq, GridPos::new(0, 0))
            .unwrap();
        let osc_id = patch
            .add_module(ModuleKind::Osc, GridPos::new(0, 2))
            .unwrap();
        let env_id = patch
            .add_module(ModuleKind::Envelope, GridPos::new(2, 0))
            .unwrap();
        let _out_id = patch
            .add_module(ModuleKind::Output, GridPos::new(0, 7))
            .unwrap();

        if let Some(osc) = patch.module_mut(osc_id) {
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

        if let Some(env) = patch.module_mut(env_id) {
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

        let pf = patch_to_file(&patch, 90.0, 4.0, Some("C4 E4 G4\n# comment\nD4"));

        let serialized = ron::ser::to_string_pretty(&pf, ron::ser::PrettyConfig::default()).unwrap();
        let pf2: PatchFile = ron::from_str(&serialized).unwrap();

        assert!((pf2.bpm - 90.0).abs() < 0.01);
        assert!((pf2.bars - 4.0).abs() < 0.01);
        assert_eq!(pf2.modules.len(), 4);
        assert!(pf2.track.as_ref().unwrap().contains("# comment"));

        let (patch2, bpm2, bars2, track2) = file_to_patch(&pf2);

        assert!((bpm2 - 90.0).abs() < 0.01);
        assert!((bars2 - 4.0).abs() < 0.01);
        assert!(track2.unwrap().contains("# comment"));

        let modules: Vec<_> = patch2.all_modules().collect();
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
