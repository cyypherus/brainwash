use super::grid::GridPos;
use super::module::{EnvPoint, ModuleId, ModuleKind, ModuleParams, Orientation};
use super::patch::Patch;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct PatchFile {
    pub bpm: f32,
    pub bars: f32,
    pub scale: String,
    pub root: String,
    pub master: f32,
    pub modules: Vec<ModuleDef>,
    pub params: Vec<ParamDef>,
    pub ports: Vec<PortDef>,
    pub envs: Vec<EnvDef>,
    pub track: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ModuleDef {
    pub id: u32,
    pub kind: String,
    pub x: u16,
    pub y: u16,
    pub orientation: Option<char>,
}

#[derive(Clone, Debug)]
pub struct ParamDef {
    pub id: u32,
    pub index: usize,
    pub value: f32,
}

#[derive(Clone, Debug)]
pub struct PortDef {
    pub id: u32,
    pub mask: u8,
}

#[derive(Clone, Debug)]
pub struct EnvDef {
    pub id: u32,
    pub time: f32,
    pub value: f32,
    pub curve: bool,
}

impl PatchFile {
    pub fn new() -> Self {
        Self {
            bpm: 120.0,
            bars: 1.0,
            scale: "chromatic".into(),
            root: "C4".into(),
            master: 1.0,
            modules: Vec::new(),
            params: Vec::new(),
            ports: Vec::new(),
            envs: Vec::new(),
            track: None,
        }
    }

    pub fn parse(content: &str) -> Result<Self, String> {
        let mut pf = PatchFile::new();
        let mut in_track = false;
        let mut track_lines = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if in_track {
                if line == "end" {
                    in_track = false;
                    pf.track = Some(track_lines.join("\n"));
                } else {
                    track_lines.push(line.to_string());
                }
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "bpm" => {
                    if parts.len() >= 2 {
                        pf.bpm = parts[1].parse().unwrap_or(120.0);
                    }
                }
                "bars" => {
                    if parts.len() >= 2 {
                        pf.bars = parts[1].parse().unwrap_or(1.0);
                    }
                }
                "scale" => {
                    if parts.len() >= 2 {
                        pf.scale = parts[1].to_string();
                    }
                }
                "root" => {
                    if parts.len() >= 2 {
                        pf.root = parts[1].to_string();
                    }
                }
                "master" => {
                    if parts.len() >= 2 {
                        pf.master = parts[1].parse().unwrap_or(1.0);
                    }
                }
                "module" => {
                    if parts.len() >= 5 {
                        let id = parts[1].parse().map_err(|_| "invalid module id")?;
                        let kind = parts[2].to_string();
                        let x = parts[3].parse().map_err(|_| "invalid x")?;
                        let y = parts[4].parse().map_err(|_| "invalid y")?;
                        let orientation = parts.get(5).and_then(|s| s.chars().next());
                        pf.modules.push(ModuleDef { id, kind, x, y, orientation });
                    }
                }
                "param" => {
                    if parts.len() >= 4 {
                        let id = parts[1].parse().map_err(|_| "invalid param id")?;
                        let index = parts[2].parse().map_err(|_| "invalid param index")?;
                        let value = parts[3].parse().map_err(|_| "invalid param value")?;
                        pf.params.push(ParamDef { id, index, value });
                    }
                }
                "port" => {
                    if parts.len() >= 3 {
                        let id = parts[1].parse().map_err(|_| "invalid port id")?;
                        let mask = if parts[2].starts_with("0x") {
                            u8::from_str_radix(&parts[2][2..], 16).unwrap_or(0xFF)
                        } else {
                            parts[2].parse().unwrap_or(0xFF)
                        };
                        pf.ports.push(PortDef { id, mask });
                    }
                }
                "env" => {
                    if parts.len() >= 4 {
                        let id = parts[1].parse().map_err(|_| "invalid env id")?;
                        let time = parts[2].parse().map_err(|_| "invalid env time")?;
                        let value = parts[3].parse().map_err(|_| "invalid env value")?;
                        let curve = parts.get(4).map(|s| *s == "curve").unwrap_or(false);
                        pf.envs.push(EnvDef { id, time, value, curve });
                    }
                }
                "track" => {
                    in_track = true;
                    track_lines.clear();
                }
                _ => {}
            }
        }

        Ok(pf)
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();

        if (self.bpm - 120.0).abs() > 0.01 {
            out.push_str(&format!("bpm {}\n", self.bpm));
        }
        if (self.bars - 1.0).abs() > 0.01 {
            out.push_str(&format!("bars {}\n", self.bars));
        }
        if self.scale != "chromatic" {
            out.push_str(&format!("scale {}\n", self.scale));
        }
        if self.root != "C4" {
            out.push_str(&format!("root {}\n", self.root));
        }
        if (self.master - 1.0).abs() > 0.01 {
            out.push_str(&format!("master {}\n", self.master));
        }

        if !out.is_empty() {
            out.push('\n');
        }

        for m in &self.modules {
            if let Some(o) = m.orientation {
                out.push_str(&format!("module {} {} {} {} {}\n", m.id, m.kind, m.x, m.y, o));
            } else {
                out.push_str(&format!("module {} {} {} {}\n", m.id, m.kind, m.x, m.y));
            }
        }

        if !self.modules.is_empty() && !self.params.is_empty() {
            out.push('\n');
        }

        for p in &self.params {
            out.push_str(&format!("param {} {} {}\n", p.id, p.index, p.value));
        }

        if !self.params.is_empty() && !self.ports.is_empty() {
            out.push('\n');
        }

        for p in &self.ports {
            out.push_str(&format!("port {} 0x{:02X}\n", p.id, p.mask));
        }

        if !self.ports.is_empty() && !self.envs.is_empty() {
            out.push('\n');
        }

        for e in &self.envs {
            if e.curve {
                out.push_str(&format!("env {} {} {} curve\n", e.id, e.time, e.value));
            } else {
                out.push_str(&format!("env {} {} {}\n", e.id, e.time, e.value));
            }
        }

        if let Some(ref track) = self.track {
            if !self.envs.is_empty() || !self.ports.is_empty() || !self.params.is_empty() || !self.modules.is_empty() {
                out.push('\n');
            }
            out.push_str("track\n");
            out.push_str(track);
            if !track.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("end\n");
        }

        out
    }
}

fn kind_to_str(kind: ModuleKind) -> &'static str {
    match kind {
        ModuleKind::Freq => "Freq",
        ModuleKind::Gate => "Gate",
        ModuleKind::Osc => "Osc",
        ModuleKind::Rise => "Rise",
        ModuleKind::Fall => "Fall",
        ModuleKind::Ramp => "Ramp",
        ModuleKind::Adsr => "ADSR",
        ModuleKind::Envelope => "Env",
        ModuleKind::Lpf => "LPF",
        ModuleKind::Hpf => "HPF",
        ModuleKind::Delay => "Delay",
        ModuleKind::Reverb => "Reverb",
        ModuleKind::Distortion => "Dist",
        ModuleKind::Flanger => "Flanger",
        ModuleKind::Mul => "Mul",
        ModuleKind::Add => "Add",
        ModuleKind::Gain => "Gain",
        ModuleKind::Gt => "Gt",
        ModuleKind::Lt => "Lt",
        ModuleKind::Switch => "Switch",
        ModuleKind::Probe => "Probe",
        ModuleKind::Output => "Out",
        ModuleKind::LSplit => "LSplit",
        ModuleKind::TSplit => "TSplit",
        ModuleKind::RJoin => "RJoin",
        ModuleKind::DJoin => "DJoin",
        ModuleKind::TurnRD => "TurnRD",
        ModuleKind::TurnDR => "TurnDR",
    }
}

fn str_to_kind(s: &str) -> Option<ModuleKind> {
    match s {
        "Freq" => Some(ModuleKind::Freq),
        "Gate" => Some(ModuleKind::Gate),
        "Osc" => Some(ModuleKind::Osc),
        "Rise" => Some(ModuleKind::Rise),
        "Fall" => Some(ModuleKind::Fall),
        "Ramp" => Some(ModuleKind::Ramp),
        "ADSR" => Some(ModuleKind::Adsr),
        "Env" => Some(ModuleKind::Envelope),
        "LPF" => Some(ModuleKind::Lpf),
        "HPF" => Some(ModuleKind::Hpf),
        "Delay" => Some(ModuleKind::Delay),
        "Reverb" => Some(ModuleKind::Reverb),
        "Dist" => Some(ModuleKind::Distortion),
        "Flanger" => Some(ModuleKind::Flanger),
        "Mul" => Some(ModuleKind::Mul),
        "Add" => Some(ModuleKind::Add),
        "Gain" => Some(ModuleKind::Gain),
        "Gt" => Some(ModuleKind::Gt),
        "Lt" => Some(ModuleKind::Lt),
        "Switch" => Some(ModuleKind::Switch),
        "Probe" => Some(ModuleKind::Probe),
        "Out" => Some(ModuleKind::Output),
        "LSplit" => Some(ModuleKind::LSplit),
        "TSplit" => Some(ModuleKind::TSplit),
        "RJoin" => Some(ModuleKind::RJoin),
        "DJoin" => Some(ModuleKind::DJoin),
        "TurnRD" => Some(ModuleKind::TurnRD),
        "TurnDR" => Some(ModuleKind::TurnDR),
        _ => None,
    }
}

pub fn patch_to_file(patch: &Patch, bpm: f32, bars: f32, track: Option<&str>) -> PatchFile {
    let mut pf = PatchFile::new();
    pf.bpm = bpm;
    pf.bars = bars;
    pf.track = track.map(|s| s.to_string());

    let mut env_points: HashMap<u32, Vec<&EnvPoint>> = HashMap::new();

    for module in patch.all_modules() {
        let pos = patch.module_position(module.id).unwrap_or(GridPos::new(0, 0));
        let orientation = match module.orientation {
            Orientation::Horizontal => None,
            Orientation::Vertical => Some('v'),
        };
        
        pf.modules.push(ModuleDef {
            id: module.id.0,
            kind: kind_to_str(module.kind).to_string(),
            x: pos.x,
            y: pos.y,
            orientation,
        });

        let defaults = ModuleParams::default_for(module.kind);
        let param_count = module.kind.param_defs().len();
        for i in 0..param_count {
            if let (Some(val), Some(def_val)) = (module.params.get_float(i), defaults.get_float(i)) {
                if (val - def_val).abs() > 0.0001 {
                    pf.params.push(ParamDef {
                        id: module.id.0,
                        index: i,
                        value: val,
                    });
                }
            }
        }

        if module.params.connected() != 0xFF {
            pf.ports.push(PortDef {
                id: module.id.0,
                mask: module.params.connected(),
            });
        }

        if let Some(pts) = module.params.env_points() {
            for pt in pts {
                env_points.entry(module.id.0).or_default().push(pt);
            }
        }
    }

    for (id, points) in env_points {
        for pt in points {
            pf.envs.push(EnvDef {
                id,
                time: pt.time,
                value: pt.value,
                curve: pt.curve,
            });
        }
    }

    pf
}

pub fn file_to_patch(pf: &PatchFile) -> (Patch, f32, f32, Option<String>) {
    let mut patch = Patch::new(32, 32);
    let mut id_map: HashMap<u32, ModuleId> = HashMap::new();

    for mdef in &pf.modules {
        let Some(kind) = str_to_kind(&mdef.kind) else { continue };
        let pos = GridPos::new(mdef.x, mdef.y);
        
        if let Some(id) = patch.add_module(kind, pos) {
            id_map.insert(mdef.id, id);
            
            if let Some(module) = patch.module_mut(id) {
                if mdef.orientation == Some('v') {
                    module.orientation = Orientation::Vertical;
                }
            }
        }
    }

    for pdef in &pf.params {
        if let Some(&id) = id_map.get(&pdef.id) {
            if let Some(module) = patch.module_mut(id) {
                module.params.set_float(pdef.index, pdef.value);
            }
        }
    }

    for portdef in &pf.ports {
        if let Some(&id) = id_map.get(&portdef.id) {
            if let Some(module) = patch.module_mut(id) {
                if let Some(c) = module.params.connected_mut() {
                    *c = portdef.mask;
                }
            }
        }
    }

    let mut env_map: HashMap<u32, Vec<EnvPoint>> = HashMap::new();
    for edef in &pf.envs {
        env_map.entry(edef.id).or_default().push(EnvPoint {
            time: edef.time,
            value: edef.value,
            curve: edef.curve,
        });
    }

    for (file_id, mut points) in env_map {
        if let Some(&id) = id_map.get(&file_id) {
            if let Some(module) = patch.module_mut(id) {
                if let Some(env_pts) = module.params.env_points_mut() {
                    points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
                    *env_pts = points;
                }
            }
        }
    }

    patch.rebuild_channels();

    (patch, pf.bpm, pf.bars, pf.track.clone())
}

pub fn save_patch(path: &Path, patch: &Patch, bpm: f32, bars: f32, track: Option<&str>) -> io::Result<()> {
    let pf = patch_to_file(patch, bpm, bars, track);
    fs::write(path, pf.to_string())
}

pub fn load_patch(path: &Path) -> io::Result<(Patch, f32, f32, Option<String>)> {
    let content = fs::read_to_string(path)?;
    let pf = PatchFile::parse(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(file_to_patch(&pf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_all_features() {
        let input = r#"# Test patch with all features
bpm 140
bars 2
scale minor
root D4
master 0.8

module 1 Freq 0 0
module 2 Gate 1 0
module 3 Osc 0 2 v
module 4 ADSR 1 2 v
module 5 Env 2 2
module 6 Mul 0 5
module 7 LPF 0 7 v
module 8 Reverb 0 10 v
module 9 Delay 3 5 v
module 10 HPF 4 5 v
module 11 Dist 5 5 v
module 12 Flanger 6 5 v
module 13 Add 0 13
module 14 Gain 0 15
module 15 Probe 1 15
module 16 Out 0 17
module 17 LSplit 3 0
module 18 TSplit 4 0
module 19 RJoin 5 0
module 20 DJoin 6 0
module 21 TurnRD 7 0
module 22 TurnDR 8 0

param 3 0 1
param 3 1 880
param 4 1 0.05
param 4 2 0.2
param 4 3 0.6
param 4 4 0.4
param 7 1 0.3
param 7 2 1.5
param 8 1 0.7
param 8 2 0.4

port 3 0x0F
port 6 0x03

env 5 0.0 0.0
env 5 0.25 1.0 curve
env 5 0.75 0.5 curve
env 5 1.0 0.0

track
C4 D4 E4 F4
G4/2 A4/2
(C4/E4/G4)
end
"#;

        let pf = PatchFile::parse(input).expect("parse failed");

        assert!((pf.bpm - 140.0).abs() < 0.01);
        assert!((pf.bars - 2.0).abs() < 0.01);
        assert_eq!(pf.scale, "minor");
        assert_eq!(pf.root, "D4");
        assert!((pf.master - 0.8).abs() < 0.01);

        assert_eq!(pf.modules.len(), 22);
        
        let osc = pf.modules.iter().find(|m| m.kind == "Osc").unwrap();
        assert_eq!(osc.x, 0);
        assert_eq!(osc.y, 2);
        assert_eq!(osc.orientation, Some('v'));

        let freq = pf.modules.iter().find(|m| m.kind == "Freq").unwrap();
        assert_eq!(freq.orientation, None);

        assert!(pf.modules.iter().any(|m| m.kind == "LSplit"));
        assert!(pf.modules.iter().any(|m| m.kind == "TSplit"));
        assert!(pf.modules.iter().any(|m| m.kind == "RJoin"));
        assert!(pf.modules.iter().any(|m| m.kind == "DJoin"));
        assert!(pf.modules.iter().any(|m| m.kind == "TurnRD"));
        assert!(pf.modules.iter().any(|m| m.kind == "TurnDR"));

        let osc_params: Vec<_> = pf.params.iter().filter(|p| p.id == 3).collect();
        assert!(osc_params.iter().any(|p| p.index == 0 && (p.value - 1.0).abs() < 0.01));
        assert!(osc_params.iter().any(|p| p.index == 1 && (p.value - 880.0).abs() < 0.01));

        let adsr_params: Vec<_> = pf.params.iter().filter(|p| p.id == 4).collect();
        assert_eq!(adsr_params.len(), 4);

        let osc_port = pf.ports.iter().find(|p| p.id == 3).unwrap();
        assert_eq!(osc_port.mask, 0x0F);
        let mul_port = pf.ports.iter().find(|p| p.id == 6).unwrap();
        assert_eq!(mul_port.mask, 0x03);

        let env_points: Vec<_> = pf.envs.iter().filter(|e| e.id == 5).collect();
        assert_eq!(env_points.len(), 4);
        assert!(!env_points[0].curve);
        assert!(env_points[1].curve);
        assert!(env_points[2].curve);
        assert!(!env_points[3].curve);

        let track = pf.track.as_ref().unwrap();
        assert!(track.contains("C4 D4 E4 F4"));
        assert!(track.contains("G4/2 A4/2"));
        assert!(track.contains("(C4/E4/G4)"));

        let output = pf.to_string();

        assert!(output.contains("bpm 140"));
        assert!(output.contains("bars 2"));
        assert!(output.contains("scale minor"));
        assert!(output.contains("root D4"));
        assert!(output.contains("master 0.8"));
        assert!(output.contains("module 3 Osc 0 2 v"));
        assert!(output.contains("param 3 0 1"));
        assert!(output.contains("port 3 0x0F"));
        assert!(output.contains("env 5 0.25 1 curve"));
        assert!(output.contains("track\n"));
        assert!(output.contains("\nend\n"));

        let pf2 = PatchFile::parse(&output).expect("re-parse failed");
        assert!((pf2.bpm - pf.bpm).abs() < 0.01);
        assert!((pf2.bars - pf.bars).abs() < 0.01);
        assert_eq!(pf2.scale, pf.scale);
        assert_eq!(pf2.root, pf.root);
        assert_eq!(pf2.modules.len(), pf.modules.len());
        assert_eq!(pf2.params.len(), pf.params.len());
        assert_eq!(pf2.ports.len(), pf.ports.len());
        assert_eq!(pf2.envs.len(), pf.envs.len());
        assert!(pf2.track.is_some());
    }

    #[test]
    fn test_defaults_omitted() {
        let pf = PatchFile::new();
        let output = pf.to_string();
        
        assert!(!output.contains("bpm"));
        assert!(!output.contains("bars"));
        assert!(!output.contains("scale"));
        assert!(!output.contains("root"));
        assert!(!output.contains("master"));
    }

    #[test]
    fn test_comments_and_blanks_ignored() {
        let input = r#"
# This is a comment
   # Indented comment

module 1 Freq 0 0

# Another comment
module 2 Out 5 5
"#;
        let pf = PatchFile::parse(input).expect("parse failed");
        assert_eq!(pf.modules.len(), 2);
    }

    #[test]
    fn test_patch_to_file_and_back() {
        let mut patch = Patch::new(20, 20);
        
        let _freq_id = patch.add_module(ModuleKind::Freq, GridPos::new(0, 0)).unwrap();
        let osc_id = patch.add_module(ModuleKind::Osc, GridPos::new(0, 2)).unwrap();
        let env_id = patch.add_module(ModuleKind::Envelope, GridPos::new(2, 0)).unwrap();
        let _out_id = patch.add_module(ModuleKind::Output, GridPos::new(0, 7)).unwrap();

        if let Some(osc) = patch.module_mut(osc_id) {
            osc.orientation = Orientation::Vertical;
            osc.params.set_float(1, 660.0);
            osc.params.set_float(2, 12.0);
            if let Some(c) = osc.params.connected_mut() {
                *c = 0x05;
            }
        }

        if let Some(env) = patch.module_mut(env_id) {
            if let Some(pts) = env.params.env_points_mut() {
                *pts = vec![
                    EnvPoint { time: 0.0, value: 0.0, curve: false },
                    EnvPoint { time: 0.5, value: 1.0, curve: true },
                    EnvPoint { time: 1.0, value: 0.0, curve: false },
                ];
            }
        }

        let pf = patch_to_file(&patch, 90.0, 4.0, Some("C4 E4 G4"));

        assert!((pf.bpm - 90.0).abs() < 0.01);
        assert!((pf.bars - 4.0).abs() < 0.01);
        assert_eq!(pf.modules.len(), 4);
        assert_eq!(pf.track, Some("C4 E4 G4".into()));

        let osc_def = pf.modules.iter().find(|m| m.kind == "Osc").unwrap();
        assert_eq!(osc_def.orientation, Some('v'));

        let osc_params: Vec<_> = pf.params.iter().filter(|p| p.id == osc_id.0).collect();
        assert!(osc_params.iter().any(|p| p.index == 1 && (p.value - 660.0).abs() < 0.01));
        assert!(osc_params.iter().any(|p| p.index == 2 && (p.value - 12.0).abs() < 0.01));

        let osc_port = pf.ports.iter().find(|p| p.id == osc_id.0);
        assert!(osc_port.is_some());
        assert_eq!(osc_port.unwrap().mask, 0x05);

        let env_points: Vec<_> = pf.envs.iter().filter(|e| e.id == env_id.0).collect();
        assert_eq!(env_points.len(), 3);

        let serialized = pf.to_string();
        let pf2 = PatchFile::parse(&serialized).expect("parse failed");
        let (patch2, bpm2, bars2, track2) = file_to_patch(&pf2);

        assert!((bpm2 - 90.0).abs() < 0.01);
        assert!((bars2 - 4.0).abs() < 0.01);
        assert_eq!(track2, Some("C4 E4 G4".into()));

        let modules: Vec<_> = patch2.all_modules().collect();
        assert_eq!(modules.len(), 4);

        let osc2 = modules.iter().find(|m| m.kind == ModuleKind::Osc).unwrap();
        assert_eq!(osc2.orientation, Orientation::Vertical);
        assert!((osc2.params.get_float(1).unwrap() - 660.0).abs() < 0.01);
        assert!((osc2.params.get_float(2).unwrap() - 12.0).abs() < 0.01);
        assert_eq!(osc2.params.connected(), 0x05);

        let env2 = modules.iter().find(|m| m.kind == ModuleKind::Envelope).unwrap();
        assert_eq!(env2.params.env_points().unwrap().len(), 3);
        assert!(env2.params.env_points().unwrap()[1].curve);
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let input = r#"
bpm 120
unknown_field value
module 1 Freq 0 0
future_feature 1 2 3
module 2 Out 5 5
"#;
        let pf = PatchFile::parse(input).expect("parse failed");
        assert_eq!(pf.modules.len(), 2);
        assert!((pf.bpm - 120.0).abs() < 0.01);
    }
}
