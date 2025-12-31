use crate::envelopes::{ADSR, Envelope, EnvelopePoint, PointType};
use crate::filters::{HighpassFilter, LowpassFilter};
use crate::delay::Delay;
use crate::reverb::Reverb;
use crate::distortion::Distortion;
use crate::flanger::Flanger;
use crate::oscillators::Osc;
use crate::clock::Clock;
use crate::track::{Track, NoteEvent};
use super::grid::{Cell, GridPos};
use super::module::{Module, ModuleId, ModuleKind, ParamKind};
use super::patch::Patch;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    pitch: u8,
    freq: f32,
    gate: f32,
    age: usize,
}

pub struct TrackState {
    pub track: Option<Track>,
    pub clock: Clock,
    voices: Vec<Voice>,
    age_counter: usize,
}

impl TrackState {
    pub fn new(num_voices: usize) -> Self {
        let mut clock = Clock::default();
        clock.bpm(120.0).bars(1.0);
        Self {
            track: None,
            clock,
            voices: vec![Voice::default(); num_voices],
            age_counter: 0,
        }
    }

    pub fn num_voices(&self) -> usize {
        self.voices.len()
    }
}

impl TrackState {
    pub fn update(&mut self, signal: &mut crate::Signal) {
        let Some(track) = &mut self.track else { return };
        
        let phase = self.clock.output(signal);
        let events = track.play(phase);
        
        for event in events {
            match event {
                NoteEvent::Press { pitch } => {
                    if self.voices.iter().any(|v| v.pitch == pitch && v.gate > 0.5) {
                        continue;
                    }
                    
                    let freq = 440.0 * 2.0f32.powf((pitch as f32 - 69.0) / 12.0);
                    self.age_counter += 1;
                    
                    let idx = self.voices.iter().position(|v| v.gate < 0.5)
                        .or_else(|| self.voices.iter().enumerate().min_by_key(|(_, v)| v.age).map(|(i, _)| i));
                    
                    if let Some(i) = idx {
                        let v = &mut self.voices[i];
                        v.pitch = pitch;
                        v.freq = freq;
                        v.gate = 1.0;
                        v.age = self.age_counter;
                    }
                }
                NoteEvent::Release { pitch } => {
                    if let Some(v) = self.voices.iter_mut().find(|v| v.pitch == pitch && v.gate > 0.5) {
                        v.gate = 0.0;
                    }
                }
            }
        }
    }

    pub fn voice(&self, idx: usize) -> (f32, f32) {
        let v = &self.voices[idx];
        (v.freq, v.gate)
    }
}

enum NodeKind {
    Freq,
    Gate,
    Oscillator(Osc),
    Adsr(ADSR),
    Envelope(Envelope),
    Lpf(LowpassFilter),
    Hpf(HighpassFilter),
    Delay(Delay),
    Reverb(Reverb),
    Distortion(Distortion),
    Flanger(Flanger),
    Mul,
    Add,
    Gain(f32),
    Probe { value: f32 },
    Pass,
    Output,
}

struct AudioNode {
    kind: NodeKind,
    input_sources: Vec<Option<usize>>,
    input_defaults: Vec<f32>,
    output: f32,
}

struct CompiledVoice {
    nodes: Vec<AudioNode>,
    execution_order: Vec<usize>,
    output_node: Option<usize>,
}

impl CompiledVoice {
    fn process(&mut self, signal: &mut crate::Signal, freq: f32, gate: f32) -> f32 {
        for &idx in &self.execution_order {
            let inputs: Vec<f32> = self.nodes[idx]
                .input_sources
                .iter()
                .enumerate()
                .map(|(i, src)| {
                    src.map(|s| self.nodes[s].output)
                        .unwrap_or(self.nodes[idx].input_defaults.get(i).copied().unwrap_or(0.0))
                })
                .collect();

            let node = &mut self.nodes[idx];
            node.output = match &mut node.kind {
                NodeKind::Freq => freq,
                NodeKind::Gate => gate,
                NodeKind::Oscillator(osc) => {
                    let f = inputs.first().copied().unwrap_or(440.0);
                    let s = inputs.get(1).copied().unwrap_or(0.0);
                    let g = inputs.get(2).copied().unwrap_or(1.0);
                    osc.freq(f).shift(s).gain(g).output(signal)
                }
                NodeKind::Adsr(adsr) => {
                    let gate_in = inputs.first().copied().unwrap_or(0.0);
                    adsr.output(gate_in, signal)
                }
                NodeKind::Envelope(env) => {
                    let phase = inputs.first().copied().unwrap_or(0.0);
                    env.output(phase)
                }
                NodeKind::Lpf(filter) => {
                    filter.output(inputs.first().copied().unwrap_or(0.0), signal)
                }
                NodeKind::Hpf(filter) => {
                    filter.output(inputs.first().copied().unwrap_or(0.0), signal)
                }
                NodeKind::Delay(delay) => {
                    delay.output(inputs.first().copied().unwrap_or(0.0))
                }
                NodeKind::Reverb(reverb) => {
                    reverb.output(inputs.first().copied().unwrap_or(0.0))
                }
                NodeKind::Distortion(dist) => {
                    dist.output(inputs.first().copied().unwrap_or(0.0))
                }
                NodeKind::Flanger(flanger) => {
                    flanger.output(inputs.first().copied().unwrap_or(0.0), signal)
                }
                NodeKind::Mul => {
                    inputs.first().copied().unwrap_or(0.0) * inputs.get(1).copied().unwrap_or(0.0)
                }
                NodeKind::Add => {
                    inputs.first().copied().unwrap_or(0.0) + inputs.get(1).copied().unwrap_or(0.0)
                }
                NodeKind::Gain(g) => {
                    inputs.first().copied().unwrap_or(0.0) * *g
                }
                NodeKind::Probe { value } => {
                    let v = inputs.first().copied().unwrap_or(0.0);
                    *value = v;
                    v
                }
                NodeKind::Pass => inputs.first().copied().unwrap_or(0.0),
                NodeKind::Output => inputs.first().copied().unwrap_or(0.0),
            };
        }

        self.output_node
            .map(|idx| self.nodes[idx].output)
            .unwrap_or(0.0)
    }
}

struct PatchVoices {
    voices: Vec<CompiledVoice>,
}

impl PatchVoices {
    fn process(&mut self, signal: &mut crate::Signal, track: &TrackState) -> f32 {
        let mut sum = 0.0;
        let n = self.voices.len().min(track.num_voices());
        for i in 0..n {
            let (freq, gate) = track.voice(i);
            sum += self.voices[i].process(signal, freq, gate);
        }
        sum
    }
}

const CROSSFADE_SAMPLES: usize = 441;

pub struct CompiledPatch {
    current: Option<PatchVoices>,
    old: Option<PatchVoices>,
    crossfade_pos: usize,
    probe_values: Vec<f32>,
}

impl Default for CompiledPatch {
    fn default() -> Self {
        Self {
            current: None,
            old: None,
            crossfade_pos: CROSSFADE_SAMPLES,
            probe_values: Vec::new(),
        }
    }
}

impl CompiledPatch {
    fn set_voices(&mut self, voices: Vec<CompiledVoice>) {
        self.old = self.current.take();
        self.current = Some(PatchVoices { voices });
        self.crossfade_pos = 0;
    }

    pub fn process(&mut self, signal: &mut crate::Signal, track: &TrackState) -> f32 {
        let new_sample = self.current.as_mut()
            .map(|p| p.process(signal, track))
            .unwrap_or(0.0);

        let sample = if self.crossfade_pos < CROSSFADE_SAMPLES {
            let old_sample = self.old.as_mut()
                .map(|p| p.process(signal, track))
                .unwrap_or(0.0);
            
            let t = self.crossfade_pos as f32 / CROSSFADE_SAMPLES as f32;
            self.crossfade_pos += 1;
            
            if self.crossfade_pos >= CROSSFADE_SAMPLES {
                self.old = None;
            }
            
            old_sample * (1.0 - t) + new_sample * t
        } else {
            new_sample
        };
        
        if let Some(ref current) = self.current {
            if !current.voices.is_empty() {
                self.probe_values.clear();
                for node in &current.voices[0].nodes {
                    if let NodeKind::Probe { value } = &node.kind {
                        self.probe_values.push(*value);
                    }
                }
            }
        }
        
        sample
    }
    
    pub fn probe_values(&self) -> &[f32] {
        &self.probe_values
    }
}

pub fn compile_patch(patch: &mut CompiledPatch, ui_patch: &Patch, num_voices: usize) {
    let modules: Vec<_> = ui_patch.all_modules().collect();
    let connections = trace_connections(ui_patch);
    
    let voices = (0..num_voices)
        .map(|_| compile_voice(&modules, &connections))
        .collect();
    
    patch.set_voices(voices);
}

fn compile_voice(modules: &[&Module], connections: &[(ModuleId, ModuleId, usize)]) -> CompiledVoice {
    let mut voice = CompiledVoice {
        nodes: Vec::new(),
        execution_order: Vec::new(),
        output_node: None,
    };
    let mut module_to_node: HashMap<ModuleId, usize> = HashMap::new();

    for module in modules {
        let node_idx = voice.nodes.len();
        module_to_node.insert(module.id, node_idx);

        let kind = create_node_kind(module);
        let input_defaults = get_input_defaults(module);
        let input_count = input_defaults.len();

        voice.nodes.push(AudioNode {
            kind,
            input_sources: vec![None; input_count],
            input_defaults,
            output: 0.0,
        });

        if module.kind == ModuleKind::Output {
            voice.output_node = Some(node_idx);
        }
    }

    for (src_id, dst_id, port_idx) in connections {
        if let (Some(&src_node), Some(&dst_node)) = (module_to_node.get(src_id), module_to_node.get(dst_id)) {
            if *port_idx < voice.nodes[dst_node].input_sources.len() {
                voice.nodes[dst_node].input_sources[*port_idx] = Some(src_node);
            }
        }
    }

    voice.execution_order = topological_sort(&voice.nodes);
    voice
}

fn trace_connections(patch: &Patch) -> Vec<(ModuleId, ModuleId, usize)> {
    let mut connections = Vec::new();
    let grid = patch.grid();

    for module in patch.all_modules() {
        let Some(pos) = patch.module_position(module.id) else { continue };
        let width = module.width();
        let height = module.height();

        if module.has_output_bottom() {
            let out_x = pos.x;
            let start_y = pos.y + height as u16;
            if let Some((target_id, port_idx)) = trace_down(grid, patch, out_x, start_y) {
                connections.push((module.id, target_id, port_idx));
            }
        }

        if module.has_output_right() {
            let out_y = pos.y;
            let start_x = pos.x + width as u16;
            if let Some((target_id, port_idx)) = trace_right(grid, patch, start_x, out_y) {
                connections.push((module.id, target_id, port_idx));
            }
        }
    }

    connections
}

fn trace_down(grid: &super::grid::Grid, patch: &Patch, x: u16, start_y: u16) -> Option<(ModuleId, usize)> {
    for y in start_y..grid.height() {
        let pos = GridPos::new(x, y);
        match grid.get(pos) {
            Cell::ChannelV { .. } => continue,
            Cell::Module { id, local_x, local_y } => {
                let module = patch.module(id)?;
                if local_y == 0 && module.has_input_top() {
                    let port_idx = local_x as usize;
                    if module.is_port_open(port_idx) {
                        return Some((id, port_idx));
                    }
                }
                return None;
            }
            _ => return None,
        }
    }
    None
}

fn trace_right(grid: &super::grid::Grid, patch: &Patch, start_x: u16, y: u16) -> Option<(ModuleId, usize)> {
    for x in start_x..grid.width() {
        let pos = GridPos::new(x, y);
        match grid.get(pos) {
            Cell::ChannelH { .. } => continue,
            Cell::Module { id, local_x, local_y } => {
                let module = patch.module(id)?;
                if local_x == 0 && module.has_input_left() {
                    let port_idx = local_y as usize;
                    if module.is_port_open(port_idx) {
                        return Some((id, port_idx));
                    }
                }
                return None;
            }
            _ => return None,
        }
    }
    None
}

fn topological_sort(nodes: &[AudioNode]) -> Vec<usize> {
    let n = nodes.len();
    if n == 0 {
        return Vec::new();
    }

    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for (idx, node) in nodes.iter().enumerate() {
        for src in node.input_sources.iter().flatten() {
            dependents[*src].push(idx);
            in_degree[idx] += 1;
        }
    }

    let mut queue: VecDeque<usize> = VecDeque::new();
    for (idx, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            queue.push_back(idx);
        }
    }

    let mut order = Vec::with_capacity(n);
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &dep in &dependents[idx] {
            in_degree[dep] -= 1;
            if in_degree[dep] == 0 {
                queue.push_back(dep);
            }
        }
    }

    if order.len() != n {
        return (0..n).collect();
    }

    order
}

fn create_node_kind(module: &Module) -> NodeKind {
    let p = &module.params.floats;
    
    match module.kind {
        ModuleKind::Freq => NodeKind::Freq,
        ModuleKind::Gate => NodeKind::Gate,
        ModuleKind::Osc => {
            let mut osc = Osc::default();
            match p[0] as usize {
                0 => osc.sin(),
                1 => osc.squ(),
                2 => osc.tri(),
                3 => osc.saw(),
                4 => osc.rsaw(),
                _ => osc.noise(),
            };
            NodeKind::Oscillator(osc)
        }
        ModuleKind::Adsr => {
            let mut adsr = ADSR::default();
            adsr.att(p[1]).dec(p[2]).sus(p[3]).rel(p[4]);
            NodeKind::Adsr(adsr)
        }
        ModuleKind::Envelope => {
            let points: Vec<EnvelopePoint> = module.params.env_points
                .iter()
                .map(|p| EnvelopePoint {
                    time: p.time,
                    value: p.value,
                    point_type: if p.curve { PointType::Curve } else { PointType::Linear },
                })
                .collect();
            NodeKind::Envelope(Envelope::new(points))
        }
        ModuleKind::Lpf => {
            let mut filter = LowpassFilter::default();
            filter.freq(p[1]).q(p[2]);
            NodeKind::Lpf(filter)
        }
        ModuleKind::Hpf => {
            let mut filter = HighpassFilter::default();
            filter.freq(p[1]).q(p[2]);
            NodeKind::Hpf(filter)
        }
        ModuleKind::Delay => {
            let mut delay = Delay::default();
            delay.delay(p[1]);
            NodeKind::Delay(delay)
        }
        ModuleKind::Reverb => {
            let mut reverb = Reverb::default();
            reverb.roomsize(p[1]).damp(p[2]);
            NodeKind::Reverb(reverb)
        }
        ModuleKind::Distortion => {
            let mut dist = Distortion::default();
            dist.drive(p[1]).gain(p[2]);
            NodeKind::Distortion(dist)
        }
        ModuleKind::Flanger => {
            let mut flanger = Flanger::default();
            flanger.freq(p[1]).depth(p[2]).feedback(p[3]);
            NodeKind::Flanger(flanger)
        }
        ModuleKind::Mul => NodeKind::Mul,
        ModuleKind::Add => NodeKind::Add,
        ModuleKind::Gain => NodeKind::Gain(p[1]),
        ModuleKind::Probe => NodeKind::Probe { value: 0.0 },
        ModuleKind::Output => NodeKind::Output,
        ModuleKind::LSplit | ModuleKind::TSplit | ModuleKind::RJoin | ModuleKind::DJoin
            | ModuleKind::TurnRD | ModuleKind::TurnDR => NodeKind::Pass,
    }
}

fn get_input_defaults(module: &Module) -> Vec<f32> {
    if module.kind.is_routing() {
        return vec![0.0; module.kind.port_count()];
    }
    
    let p = &module.params.floats;
    let defs = module.kind.param_defs();
    
    defs.iter()
        .enumerate()
        .filter(|(_, d)| !matches!(d.kind, ParamKind::Enum { .. }))
        .map(|(i, _)| p[i])
        .collect()
}


