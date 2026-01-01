use super::grid::{Cell, GridPos};
use super::module::{Module, ModuleId, ModuleKind, SubPatchId};
use super::patch::{Patch, PatchSet};
use crate::clock::Clock;
use crate::delay::Delay;
use crate::distortion::Distortion;
use crate::envelopes::{ADSR, Envelope, EnvelopePoint, PointType};
use crate::filters::{HighpassFilter, LowpassFilter};
use crate::flanger::Flanger;
use crate::gate_ramp::GateRamp;
use crate::oscillators::Osc;
use crate::ramp::Ramp;
use crate::reverb::Reverb;
use crate::track::{NoteEvent, Track};
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

    pub fn set_track(&mut self, track: Option<Track>) {
        self.track = track;
        for v in &mut self.voices {
            *v = Voice::default();
        }
        self.age_counter = 0;
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
                    let freq = 440.0 * 2.0f32.powf((pitch as f32 - 69.0) / 12.0);
                    self.age_counter += 1;

                    let idx = self
                        .voices
                        .iter()
                        .position(|v| v.pitch == pitch && v.gate > 0.5)
                        .or_else(|| {
                            self.voices
                                .iter()
                                .enumerate()
                                .filter(|(_, v)| v.gate < 0.5)
                                .min_by_key(|(_, v)| v.age)
                                .map(|(i, _)| i)
                        })
                        .or_else(|| {
                            self.voices
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, v)| v.age)
                                .map(|(i, _)| i)
                        });

                    if let Some(i) = idx {
                        let v = &mut self.voices[i];
                        v.pitch = pitch;
                        v.freq = freq;
                        v.gate = 1.0;
                        v.age = self.age_counter;
                    }
                }
                NoteEvent::Release { pitch } => {
                    if let Some(v) = self
                        .voices
                        .iter_mut()
                        .find(|v| v.pitch == pitch && v.gate > 0.5)
                    {
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
    Rise(GateRamp),
    Fall(GateRamp),
    Ramp(Ramp),
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
    Gt,
    Lt,
    Switch,
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
    last_gate: f32,
}

impl CompiledVoice {
    fn reset(&mut self) {
        for node in &mut self.nodes {
            match &mut node.kind {
                NodeKind::Rise(ramp) | NodeKind::Fall(ramp) => ramp.reset(),
                NodeKind::Adsr(adsr) => adsr.reset(),
                _ => {}
            }
            node.output = 0.0;
        }
    }

    fn process(&mut self, signal: &mut crate::Signal, freq: f32, gate: f32) -> f32 {
        if gate > 0.5 && self.last_gate < 0.5 {
            self.reset();
        }
        self.last_gate = gate;
        for &idx in &self.execution_order {
            let inputs: Vec<f32> = self.nodes[idx]
                .input_sources
                .iter()
                .enumerate()
                .map(|(i, src)| {
                    src.map(|s| self.nodes[s].output).unwrap_or(
                        self.nodes[idx]
                            .input_defaults
                            .get(i)
                            .copied()
                            .unwrap_or(0.0),
                    )
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
                NodeKind::Rise(ramp) | NodeKind::Fall(ramp) => {
                    let gate_in = inputs.first().copied().unwrap_or(0.0);
                    ramp.output(gate_in, signal)
                }
                NodeKind::Ramp(ramp) => {
                    let val = inputs.first().copied().unwrap_or(0.0);
                    ramp.value(val);
                    ramp.output(signal)
                }
                NodeKind::Adsr(adsr) => {
                    let rise = inputs.first().copied().unwrap_or(0.0);
                    let fall = inputs.get(1).copied().unwrap_or(0.0);
                    adsr.output(rise, fall)
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
                NodeKind::Delay(delay) => delay.output(inputs.first().copied().unwrap_or(0.0)),
                NodeKind::Reverb(reverb) => reverb.output(inputs.first().copied().unwrap_or(0.0)),
                NodeKind::Distortion(dist) => dist.output(inputs.first().copied().unwrap_or(0.0)),
                NodeKind::Flanger(flanger) => {
                    flanger.output(inputs.first().copied().unwrap_or(0.0), signal)
                }
                NodeKind::Mul => {
                    inputs.first().copied().unwrap_or(0.0) * inputs.get(1).copied().unwrap_or(0.0)
                }
                NodeKind::Add => {
                    inputs.first().copied().unwrap_or(0.0) + inputs.get(1).copied().unwrap_or(0.0)
                }
                NodeKind::Gain(g) => inputs.first().copied().unwrap_or(0.0) * *g,
                NodeKind::Gt => {
                    let a = inputs.first().copied().unwrap_or(0.0);
                    let b = inputs.get(1).copied().unwrap_or(0.0);
                    if a > b { 1.0 } else { 0.0 }
                }
                NodeKind::Lt => {
                    let a = inputs.first().copied().unwrap_or(0.0);
                    let b = inputs.get(1).copied().unwrap_or(0.0);
                    if a < b { 1.0 } else { 0.0 }
                }
                NodeKind::Switch => {
                    let sel = inputs.first().copied().unwrap_or(0.0);
                    let a = inputs.get(1).copied().unwrap_or(0.0);
                    let b = inputs.get(2).copied().unwrap_or(0.0);
                    if sel <= 0.5 { a } else { b }
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

const PROBE_HISTORY_LEN: usize = 44100 * 2;

pub struct CompiledPatch {
    current: Option<PatchVoices>,
    old: Option<PatchVoices>,
    crossfade_pos: usize,
    probe_histories: Vec<VecDeque<f32>>,
    probe_voice: usize,
}

impl Default for CompiledPatch {
    fn default() -> Self {
        Self {
            current: None,
            old: None,
            crossfade_pos: CROSSFADE_SAMPLES,
            probe_histories: Vec::new(),
            probe_voice: 0,
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
        let new_sample = self
            .current
            .as_mut()
            .map(|p| p.process(signal, track))
            .unwrap_or(0.0);

        let sample = if self.crossfade_pos < CROSSFADE_SAMPLES {
            let old_sample = self
                .old
                .as_mut()
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

        let sample = sample.clamp(-1.0, 1.0);

        if let Some(ref current) = self.current {
            let voice_idx = self.probe_voice.min(current.voices.len().saturating_sub(1));
            if let Some(voice) = current.voices.get(voice_idx) {
                let mut probe_idx = 0;
                for node in &voice.nodes {
                    if let NodeKind::Probe { value } = &node.kind {
                        if probe_idx >= self.probe_histories.len() {
                            self.probe_histories
                                .push(VecDeque::with_capacity(PROBE_HISTORY_LEN));
                        }
                        let history = &mut self.probe_histories[probe_idx];
                        history.push_back(*value);
                        if history.len() > PROBE_HISTORY_LEN {
                            history.pop_front();
                        }
                        probe_idx += 1;
                    }
                }
                self.probe_histories.truncate(probe_idx);
            }
        }

        sample
    }

    pub fn probe_history(&self, idx: usize) -> Option<&VecDeque<f32>> {
        self.probe_histories.get(idx)
    }

    pub fn clear_probe_history(&mut self, idx: usize) {
        if let Some(h) = self.probe_histories.get_mut(idx) {
            h.clear();
        }
    }

    pub fn probe_voice(&self) -> usize {
        self.probe_voice
    }

    pub fn set_probe_voice(&mut self, voice: usize) {
        self.probe_voice = voice;
    }
}

pub fn compile_patch(patch: &mut CompiledPatch, patches: &PatchSet, num_voices: usize) {
    let (modules, connections) = flatten_patchset(patches);
    let module_refs: Vec<&Module> = modules.iter().collect();

    let voices = (0..num_voices)
        .map(|_| compile_voice(&module_refs, &connections))
        .collect();

    patch.set_voices(voices);
}

fn flatten_patchset(patches: &PatchSet) -> (Vec<Module>, Vec<(ModuleId, ModuleId, usize)>) {
    let mut flat_modules: Vec<Module> = Vec::new();
    let mut id_map: HashMap<(Option<SubPatchId>, ModuleId), ModuleId> = HashMap::new();
    let mut next_id = 0u32;

    for module in patches.root.all_modules() {
        if let ModuleKind::SubPatch(sub_id) = module.kind {
            if let Some(sub) = patches.subpatch(sub_id) {
                let sub_pos = patches.root.module_position(module.id);
                expand_subpatch(
                    sub_id,
                    sub,
                    sub_pos,
                    module.id,
                    &mut flat_modules,
                    &mut id_map,
                    &mut next_id,
                );
            }
        } else {
            let new_id = ModuleId(next_id);
            next_id += 1;
            id_map.insert((None, module.id), new_id);

            let mut m = module.clone();
            m.id = new_id;
            flat_modules.push(m);
        }
    }

    let mut connections = Vec::new();

    for module in patches.root.all_modules() {
        if matches!(module.kind, ModuleKind::SubPatch(_)) {
            continue;
        }

        let Some(pos) = patches.root.module_position(module.id) else {
            continue;
        };

        if module.has_output_bottom() {
            if let Some((target_id, port_idx)) =
                trace_down(patches.root.grid(), &patches.root, pos.x, pos.y + module.height() as u16)
            {
                let src = id_map.get(&(None, module.id)).copied();
                let dst = id_map.get(&(None, target_id)).copied();
                if let (Some(s), Some(d)) = (src, dst) {
                    connections.push((s, d, port_idx));
                }
            }
        }

        if module.has_output_right() {
            if let Some((target_id, port_idx)) =
                trace_right(patches.root.grid(), &patches.root, pos.x + module.width() as u16, pos.y)
            {
                let src = id_map.get(&(None, module.id)).copied();
                let dst = id_map.get(&(None, target_id)).copied();
                if let (Some(s), Some(d)) = (src, dst) {
                    connections.push((s, d, port_idx));
                }
            }
        }
    }

    trace_subpatch_connections(patches, &id_map, &mut connections);

    (flat_modules, connections)
}

fn expand_subpatch(
    sub_id: SubPatchId,
    sub: &super::patch::SubPatchDef,
    _parent_pos: Option<GridPos>,
    _parent_module_id: ModuleId,
    flat_modules: &mut Vec<Module>,
    id_map: &mut HashMap<(Option<SubPatchId>, ModuleId), ModuleId>,
    next_id: &mut u32,
) {
    for module in sub.patch.all_modules() {
        let new_id = ModuleId(*next_id);
        *next_id += 1;
        id_map.insert((Some(sub_id), module.id), new_id);

        let mut m = module.clone();
        m.id = new_id;
        flat_modules.push(m);
    }
}

fn trace_subpatch_connections(
    patches: &PatchSet,
    id_map: &HashMap<(Option<SubPatchId>, ModuleId), ModuleId>,
    connections: &mut Vec<(ModuleId, ModuleId, usize)>,
) {
    for module in patches.root.all_modules() {
        let ModuleKind::SubPatch(sub_id) = module.kind else {
            continue;
        };
        let Some(sub) = patches.subpatch(sub_id) else {
            continue;
        };
        let Some(parent_pos) = patches.root.module_position(module.id) else {
            continue;
        };

        let mut sub_inputs: Vec<_> = sub.patch.all_modules()
            .filter(|m| m.kind == ModuleKind::SubIn)
            .filter_map(|m| {
                let pos = sub.patch.module_position(m.id)?;
                Some((pos.y, m.id))
            })
            .collect();
        sub_inputs.sort_by_key(|(y, _)| *y);
        let sub_inputs: Vec<_> = sub_inputs.into_iter().enumerate().map(|(i, (_, id))| (i, id)).collect();

        let mut sub_outputs: Vec<_> = sub.patch.all_modules()
            .filter(|m| m.kind == ModuleKind::SubOut)
            .filter_map(|m| {
                let pos = sub.patch.module_position(m.id)?;
                Some((pos.x, m.id))
            })
            .collect();
        sub_outputs.sort_by_key(|(x, _)| *x);
        let sub_outputs: Vec<_> = sub_outputs.into_iter().enumerate().map(|(i, (_, id))| (i, id)).collect();

        for src in patches.root.all_modules() {
            if src.id == module.id {
                continue;
            }
            let Some(src_pos) = patches.root.module_position(src.id) else {
                continue;
            };

            if src.has_output_right() && module.has_input_left() {
                let start_x = src_pos.x + src.width() as u16;
                if let Some((target_id, port_idx)) =
                    trace_right(patches.root.grid(), &patches.root, start_x, src_pos.y)
                {
                    if target_id == module.id {
                        if let Some(&(_, sub_in_id)) = sub_inputs.iter().find(|(idx, _)| *idx == port_idx) {
                            let flat_src = id_map.get(&(None, src.id)).copied();
                            let flat_dst = id_map.get(&(Some(sub_id), sub_in_id)).copied();
                            if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                                connections.push((s, d, 0));
                            }
                        }
                    }
                }
            }

            if src.has_output_bottom() && module.has_input_top() {
                let start_y = src_pos.y + src.height() as u16;
                if let Some((target_id, port_idx)) =
                    trace_down(patches.root.grid(), &patches.root, src_pos.x, start_y)
                {
                    if target_id == module.id {
                        if let Some(&(_, sub_in_id)) = sub_inputs.iter().find(|(idx, _)| *idx == port_idx) {
                            let flat_src = id_map.get(&(None, src.id)).copied();
                            let flat_dst = id_map.get(&(Some(sub_id), sub_in_id)).copied();
                            if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                                connections.push((s, d, 0));
                            }
                        }
                    }
                }
            }
        }

        if module.has_output_bottom() {
            for (out_idx, sub_out_id) in &sub_outputs {
                let out_x = parent_pos.x + *out_idx as u16;
                let out_y = parent_pos.y + module.height() as u16;

                if let Some((target_id, port_idx)) =
                    trace_down(patches.root.grid(), &patches.root, out_x, out_y)
                {
                    let flat_src = id_map.get(&(Some(sub_id), *sub_out_id)).copied();
                    let flat_dst = id_map.get(&(None, target_id)).copied();
                    if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                        connections.push((s, d, port_idx));
                    }
                }
            }
        }

        if module.has_output_right() {
            for (out_idx, sub_out_id) in &sub_outputs {
                let out_x = parent_pos.x + module.width() as u16;
                let out_y = parent_pos.y + *out_idx as u16;

                if let Some((target_id, port_idx)) =
                    trace_right(patches.root.grid(), &patches.root, out_x, out_y)
                {
                    let flat_src = id_map.get(&(Some(sub_id), *sub_out_id)).copied();
                    let flat_dst = id_map.get(&(None, target_id)).copied();
                    if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                        connections.push((s, d, port_idx));
                    }
                }
            }
        }

        for internal in sub.patch.all_modules() {
            let Some(pos) = sub.patch.module_position(internal.id) else {
                continue;
            };

            if internal.has_output_bottom() {
                if let Some((target_id, port_idx)) =
                    trace_down(sub.patch.grid(), &sub.patch, pos.x, pos.y + internal.height() as u16)
                {
                    let flat_src = id_map.get(&(Some(sub_id), internal.id)).copied();
                    let flat_dst = id_map.get(&(Some(sub_id), target_id)).copied();
                    if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                        connections.push((s, d, port_idx));
                    }
                }
            }

            if internal.has_output_right() {
                if let Some((target_id, port_idx)) =
                    trace_right(sub.patch.grid(), &sub.patch, pos.x + internal.width() as u16, pos.y)
                {
                    let flat_src = id_map.get(&(Some(sub_id), internal.id)).copied();
                    let flat_dst = id_map.get(&(Some(sub_id), target_id)).copied();
                    if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                        connections.push((s, d, port_idx));
                    }
                }
            }
        }
    }
}

fn compile_voice(
    modules: &[&Module],
    connections: &[(ModuleId, ModuleId, usize)],
) -> CompiledVoice {
    let mut voice = CompiledVoice {
        nodes: Vec::new(),
        execution_order: Vec::new(),
        output_node: None,
        last_gate: 0.0,
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
        if let (Some(&src_node), Some(&dst_node)) =
            (module_to_node.get(src_id), module_to_node.get(dst_id))
        {
            if *port_idx < voice.nodes[dst_node].input_sources.len() {
                voice.nodes[dst_node].input_sources[*port_idx] = Some(src_node);
            }
        }
    }

    voice.execution_order = topological_sort(&voice.nodes);
    voice
}

fn trace_down(
    grid: &super::grid::Grid,
    patch: &Patch,
    x: u16,
    start_y: u16,
) -> Option<(ModuleId, usize)> {
    for y in start_y..grid.height() {
        let pos = GridPos::new(x, y);
        match grid.get(pos) {
            Cell::ChannelV { .. } | Cell::ChannelCross { .. } => continue,
            Cell::Module {
                id,
                local_x,
                local_y,
            } => {
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

fn trace_right(
    grid: &super::grid::Grid,
    patch: &Patch,
    start_x: u16,
    y: u16,
) -> Option<(ModuleId, usize)> {
    for x in start_x..grid.width() {
        let pos = GridPos::new(x, y);
        match grid.get(pos) {
            Cell::ChannelH { .. } | Cell::ChannelCross { .. } => continue,
            Cell::Module {
                id,
                local_x,
                local_y,
            } => {
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
    use super::module::{ModuleParams, WaveType};

    match (&module.kind, &module.params) {
        (ModuleKind::Freq, _) => NodeKind::Freq,
        (ModuleKind::Gate, _) => NodeKind::Gate,
        (ModuleKind::Osc, ModuleParams::Osc { wave, uni, .. }) => {
            let mut osc = Osc::default();
            match wave {
                WaveType::Sin => osc.sin(),
                WaveType::Squ => osc.squ(),
                WaveType::Tri => osc.tri(),
                WaveType::Saw => osc.saw(),
                WaveType::RSaw => osc.rsaw(),
                WaveType::Noise => osc.noise(),
            };
            if *uni {
                osc.unipolar();
            }
            NodeKind::Oscillator(osc)
        }
        (ModuleKind::Rise, ModuleParams::Rise { time, .. }) => {
            let mut ramp = GateRamp::default();
            ramp.rise();
            ramp.time(*time);
            NodeKind::Rise(ramp)
        }
        (ModuleKind::Fall, ModuleParams::Fall { time, .. }) => {
            let mut ramp = GateRamp::default();
            ramp.fall();
            ramp.time(*time);
            NodeKind::Fall(ramp)
        }
        (ModuleKind::Ramp, ModuleParams::Ramp { time, .. }) => {
            let mut ramp = Ramp::default();
            ramp.time(*time);
            NodeKind::Ramp(ramp)
        }
        (
            ModuleKind::Adsr,
            ModuleParams::Adsr {
                attack_ratio,
                sustain,
                ..
            },
        ) => {
            let mut adsr = ADSR::default();
            adsr.att(*attack_ratio).sus(*sustain);
            NodeKind::Adsr(adsr)
        }
        (ModuleKind::Envelope, ModuleParams::Envelope { points, .. }) => {
            let env_points: Vec<EnvelopePoint> = points
                .iter()
                .map(|p| EnvelopePoint {
                    time: p.time,
                    value: p.value,
                    point_type: if p.curve {
                        PointType::Curve
                    } else {
                        PointType::Linear
                    },
                })
                .collect();
            NodeKind::Envelope(Envelope::new(env_points))
        }
        (ModuleKind::Lpf, ModuleParams::Filter { freq, q, .. }) => {
            let mut filter = LowpassFilter::default();
            filter.freq(*freq).q(*q);
            NodeKind::Lpf(filter)
        }
        (ModuleKind::Hpf, ModuleParams::Filter { freq, q, .. }) => {
            let mut filter = HighpassFilter::default();
            let mapped = 20.0 * (100.0f32).powf(*freq) / 22050.0;
            filter.freq(mapped).q(*q);
            NodeKind::Hpf(filter)
        }
        (ModuleKind::Delay, ModuleParams::Delay { samples, .. }) => {
            let mut delay = Delay::default();
            delay.delay(*samples);
            NodeKind::Delay(delay)
        }
        (ModuleKind::Reverb, ModuleParams::Reverb { room, damp, .. }) => {
            let mut reverb = Reverb::default();
            reverb.roomsize(*room).damp(*damp);
            NodeKind::Reverb(reverb)
        }
        (ModuleKind::Distortion, ModuleParams::Distortion { drive, gain, .. }) => {
            let mut dist = Distortion::default();
            dist.drive(*drive).gain(*gain);
            NodeKind::Distortion(dist)
        }
        (
            ModuleKind::Flanger,
            ModuleParams::Flanger {
                rate,
                depth,
                feedback,
                ..
            },
        ) => {
            let mut flanger = Flanger::default();
            flanger.freq(*rate).depth(*depth).feedback(*feedback);
            NodeKind::Flanger(flanger)
        }
        (ModuleKind::Mul, _) => NodeKind::Mul,
        (ModuleKind::Add, _) => NodeKind::Add,
        (ModuleKind::Gain, ModuleParams::Gain { gain, .. }) => NodeKind::Gain(*gain),
        (ModuleKind::Gt, _) => NodeKind::Gt,
        (ModuleKind::Lt, _) => NodeKind::Lt,
        (ModuleKind::Switch, _) => NodeKind::Switch,
        (ModuleKind::Probe, _) => NodeKind::Probe { value: 0.0 },
        (ModuleKind::Output, _) => NodeKind::Output,
        (ModuleKind::LSplit, _)
        | (ModuleKind::TSplit, _)
        | (ModuleKind::RJoin, _)
        | (ModuleKind::DJoin, _)
        | (ModuleKind::TurnRD, _)
        | (ModuleKind::TurnDR, _)
        | (ModuleKind::SubIn, _)
        | (ModuleKind::SubOut, _) => NodeKind::Pass,
        (ModuleKind::SubPatch(_), _) => NodeKind::Pass,
        _ => NodeKind::Pass,
    }
}

fn get_input_defaults(module: &Module) -> Vec<f32> {
    if module.kind.is_routing() {
        return vec![0.0; module.kind.port_count()];
    }

    if matches!(module.kind, ModuleKind::SubPatch(_)) {
        return Vec::new();
    }

    match module.kind {
        ModuleKind::SubIn => return Vec::new(),
        ModuleKind::SubOut => return vec![0.0],
        _ => {}
    }

    let defs = module.kind.param_defs();

    defs.iter()
        .enumerate()
        .filter(|(_, d)| !matches!(d.kind, super::module::ParamKind::Enum))
        .map(|(i, _)| module.params.get_float(i).unwrap_or(0.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::grid::GridPos;

    use ratatui::style::Color;

    #[test]
    fn test_flatten_simple() {
        let mut patches = PatchSet::new(20, 20);

        patches.root.add_module(ModuleKind::Freq, GridPos::new(0, 0));
        patches.root.add_module(ModuleKind::Osc, GridPos::new(1, 0));
        patches.root.add_module(ModuleKind::Output, GridPos::new(5, 0));

        let (modules, connections) = flatten_patchset(&patches);

        assert_eq!(modules.len(), 3);
        assert!(connections.iter().any(|(src, dst, _)| {
            let src_kind = modules.iter().find(|m| m.id == *src).map(|m| m.kind);
            let dst_kind = modules.iter().find(|m| m.id == *dst).map(|m| m.kind);
            src_kind == Some(ModuleKind::Freq) && dst_kind == Some(ModuleKind::Osc)
        }));
    }

    #[test]
    fn test_flatten_with_subpatch() {
        let mut patches = PatchSet::new(20, 20);

        patches.root.add_module(ModuleKind::Freq, GridPos::new(0, 0));

        let sub_id = patches.create_subpatch("Test".into(), Color::Red);

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(ModuleKind::SubIn, GridPos::new(0, 0));
            sub.patch.add_module(ModuleKind::Gain, GridPos::new(0, 1));
            sub.patch.add_module(ModuleKind::SubOut, GridPos::new(0, 3));
        }

        patches.root.add_module(ModuleKind::SubPatch(sub_id), GridPos::new(0, 1));
        patches.root.add_module(ModuleKind::Output, GridPos::new(0, 3));

        let (modules, _connections) = flatten_patchset(&patches);

        assert_eq!(modules.len(), 5);

        assert!(modules.iter().any(|m| m.kind == ModuleKind::Freq));
        assert!(modules.iter().any(|m| m.kind == ModuleKind::SubIn));
        assert!(modules.iter().any(|m| m.kind == ModuleKind::Gain));
        assert!(modules.iter().any(|m| m.kind == ModuleKind::SubOut));
        assert!(modules.iter().any(|m| m.kind == ModuleKind::Output));

        assert!(!modules.iter().any(|m| matches!(m.kind, ModuleKind::SubPatch(_))));
    }

    #[test]
    fn test_subpatch_connections() {
        let mut patches = PatchSet::new(20, 20);

        patches.root.add_module(ModuleKind::Freq, GridPos::new(0, 0));

        let sub_id = patches.create_subpatch("Test".into(), Color::Red);

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(ModuleKind::SubIn, GridPos::new(0, 0));
            sub.patch.add_module(ModuleKind::SubOut, GridPos::new(1, 0));
        }

        patches.root.add_module(ModuleKind::SubPatch(sub_id), GridPos::new(1, 0));
        patches.root.add_module(ModuleKind::Output, GridPos::new(2, 0));
        patches.root.rebuild_channels();

        let (modules, connections) = flatten_patchset(&patches);

        let freq_id = modules.iter().find(|m| m.kind == ModuleKind::Freq).map(|m| m.id);
        let sub_in_id = modules.iter().find(|m| m.kind == ModuleKind::SubIn).map(|m| m.id);
        let sub_out_id = modules.iter().find(|m| m.kind == ModuleKind::SubOut).map(|m| m.id);
        let output_id = modules.iter().find(|m| m.kind == ModuleKind::Output).map(|m| m.id);

        assert!(connections.iter().any(|(src, dst, _)| Some(*src) == freq_id && Some(*dst) == sub_in_id),
            "Freq->SubIn connection missing. Connections: {:?}", connections);

        assert!(connections.iter().any(|(src, dst, _)| Some(*src) == sub_in_id && Some(*dst) == sub_out_id),
            "SubIn->SubOut connection missing. Connections: {:?}", connections);

        assert!(connections.iter().any(|(src, dst, _)| Some(*src) == sub_out_id && Some(*dst) == output_id),
            "SubOut->Output connection missing. Connections: {:?}", connections);
    }
}
