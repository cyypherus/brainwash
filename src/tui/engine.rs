use super::grid::{Cell, GridPos};
use super::module::{Module, ModuleId, ModuleKind, StandardModule, SubPatchId, SubpatchModule};
use super::patch::{Patch, PatchSet};
use crate::allpass::AllpassFilter;
use crate::clock::Clock;
use crate::comb::CombFilter;
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

pub type MeterSender = flume::Sender<MeterFrame>;
pub type MeterReceiver = flume::Receiver<MeterFrame>;
pub type OutputSender = flume::Sender<f32>;
pub type OutputReceiver = flume::Receiver<f32>;
pub type CommandSender = flume::Sender<AudioCommand>;
pub type CommandReceiver = flume::Receiver<AudioCommand>;

pub fn meter_channel() -> (MeterSender, MeterReceiver) {
    flume::unbounded()
}

pub fn output_channel() -> (OutputSender, OutputReceiver) {
    flume::unbounded()
}

pub fn command_channel() -> (CommandSender, CommandReceiver) {
    flume::unbounded()
}

pub enum AudioCommand {
    SetBpm(f32),
    SetInstrument {
        idx: usize,
        voices: Vec<CompiledVoice>,
        track: Option<Track>,
        bars: f32,
    },
    SetVoices {
        idx: usize,
        voices: Vec<CompiledVoice>,
        bars: f32,
    },
    SetProbeVoice(usize),
}

pub struct MeterFrame {
    pub instrument_idx: usize,
    pub ports: Vec<(ModuleId, Vec<f32>)>,
    pub probes: Vec<(ModuleId, Vec<f32>)>,
    pub active_pitches: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    pitch: u8,
    degree: i32,
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

    pub fn set_track(&mut self, mut track: Option<Track>) {
        if let Some(ref mut t) = track {
            t.set_playhead(self.clock.phase());
        }
        self.track = track;
        for v in &mut self.voices {
            v.gate = 0.0;
        }
    }
}

impl TrackState {
    pub fn update(&mut self, signal: &mut crate::Signal) {
        let Some(track) = &mut self.track else { return };

        let phase = self.clock.output(signal);
        let events = track.play(phase);

        for event in events {
            match event {
                NoteEvent::Press { pitch, degree } => {
                    let freq = 440.0 * 2.0f32.powf((pitch as f32 - 69.0) / 12.0);
                    self.age_counter += 1;

                    let idx = self
                        .voices
                        .iter()
                        .position(|v| v.pitch == pitch && v.gate > 0.5)
                        .or_else(|| {
                            self.voices
                                .iter()
                                .position(|v| v.pitch == pitch && v.gate < 0.5)
                        })
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
                        v.degree = degree;
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

    pub fn voice(&self, idx: usize) -> (f32, f32, i32) {
        let v = &self.voices[idx];
        (v.freq, v.gate, v.degree)
    }

    pub fn active_pitches(&self) -> Vec<u8> {
        self.voices
            .iter()
            .filter(|v| v.gate > 0.5)
            .map(|v| v.pitch)
            .collect()
    }
}

enum NodeKind {
    Freq,
    Gate,
    Degree,
    DegreeGate { target: i32 },
    Oscillator(Osc),
    Rise(GateRamp),
    Fall(GateRamp),
    Ramp(Ramp),
    Adsr(ADSR),
    Envelope(Envelope),
    Lpf(LowpassFilter),
    Hpf(HighpassFilter),
    Comb(CombFilter),
    Allpass(AllpassFilter),
    Delay(Delay),
    DelayTap { delay_node: usize, gain: f32 },
    Reverb(Reverb),
    Distortion(Distortion),
    Flanger(Flanger),
    Mul,
    Add,
    Gt,
    Lt,
    Switch,
    Rng { last_gate: f32, value: f32 },
    Sample { samples: std::sync::Arc<Vec<f32>> },
    Probe,
    Pass,
    Output,
}

impl NodeKind {
    fn copy_state_from(&mut self, other: &NodeKind) {
        match (self, other) {
            (NodeKind::Oscillator(new), NodeKind::Oscillator(old)) => {
                new.copy_phase_from(old);
            }
            (NodeKind::Rise(new), NodeKind::Rise(old))
            | (NodeKind::Fall(new), NodeKind::Fall(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Ramp(new), NodeKind::Ramp(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Adsr(new), NodeKind::Adsr(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Envelope(new), NodeKind::Envelope(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Lpf(new), NodeKind::Lpf(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Hpf(new), NodeKind::Hpf(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Comb(new), NodeKind::Comb(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Allpass(new), NodeKind::Allpass(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Delay(new), NodeKind::Delay(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Reverb(new), NodeKind::Reverb(old)) => {
                new.copy_state_from(old);
            }
            (NodeKind::Flanger(new), NodeKind::Flanger(old)) => {
                new.copy_state_from(old);
            }
            (
                NodeKind::Rng {
                    last_gate: new_lg,
                    value: new_v,
                },
                NodeKind::Rng {
                    last_gate: old_lg,
                    value: old_v,
                },
            ) => {
                *new_lg = *old_lg;
                *new_v = *old_v;
            }
            (NodeKind::Freq, NodeKind::Freq)
            | (NodeKind::Gate, NodeKind::Gate)
            | (NodeKind::Degree, NodeKind::Degree)
            | (NodeKind::DegreeGate { .. }, NodeKind::DegreeGate { .. })
            | (NodeKind::DelayTap { .. }, NodeKind::DelayTap { .. })
            | (NodeKind::Distortion(_), NodeKind::Distortion(_))
            | (NodeKind::Mul, NodeKind::Mul)
            | (NodeKind::Add, NodeKind::Add)
            | (NodeKind::Gt, NodeKind::Gt)
            | (NodeKind::Lt, NodeKind::Lt)
            | (NodeKind::Switch, NodeKind::Switch)
            | (NodeKind::Sample { .. }, NodeKind::Sample { .. })
            | (NodeKind::Probe, NodeKind::Probe)
            | (NodeKind::Pass, NodeKind::Pass)
            | (NodeKind::Output, NodeKind::Output) => {}
            (_, _) => {}
        }
    }
}

struct AudioNode {
    kind: NodeKind,
    module_id: ModuleId,
    input_sources: Vec<Option<usize>>,
    input_defaults: Vec<f32>,
    input_values: Vec<f32>,
    output: f32,
}

pub struct CompiledVoice {
    nodes: Vec<AudioNode>,
    execution_order: Vec<usize>,
    output_node: Option<usize>,
    last_gate: f32,
}

impl CompiledVoice {
    fn inherit_state_from(&mut self, old: &CompiledVoice) {
        self.last_gate = old.last_gate;
        for new_node in &mut self.nodes {
            if let Some(old_node) = old.nodes.iter().find(|n| n.module_id == new_node.module_id) {
                new_node.kind.copy_state_from(&old_node.kind);
                new_node.output = old_node.output;
            }
        }
    }

    fn reset(&mut self) {
        for node in &mut self.nodes {
            match &mut node.kind {
                NodeKind::Rise(ramp) | NodeKind::Fall(ramp) => ramp.reset(),
                NodeKind::Adsr(adsr) => adsr.reset(),
                NodeKind::Freq
                | NodeKind::Gate
                | NodeKind::Degree
                | NodeKind::DegreeGate { .. }
                | NodeKind::Oscillator(_)
                | NodeKind::Ramp(_)
                | NodeKind::Envelope(_)
                | NodeKind::Lpf(_)
                | NodeKind::Hpf(_)
                | NodeKind::Comb(_)
                | NodeKind::Allpass(_)
                | NodeKind::Delay(_)
                | NodeKind::DelayTap { .. }
                | NodeKind::Reverb(_)
                | NodeKind::Distortion(_)
                | NodeKind::Flanger(_)
                | NodeKind::Mul
                | NodeKind::Add
                | NodeKind::Gt
                | NodeKind::Lt
                | NodeKind::Switch
                | NodeKind::Rng { .. }
                | NodeKind::Sample { .. }
                | NodeKind::Probe
                | NodeKind::Pass
                | NodeKind::Output => {}
            }
            node.output = 0.0;
            node.input_values.fill(0.0);
        }
    }

    fn process(&mut self, signal: &mut crate::Signal, freq: f32, gate: f32, degree: i32) -> f32 {
        if gate > 0.5 && self.last_gate < 0.5 {
            self.reset();
        }
        self.last_gate = gate;
        for &idx in &self.execution_order {
            let input_count = self.nodes[idx].input_sources.len();
            for i in 0..input_count {
                let val = self.nodes[idx].input_sources[i]
                    .map(|s| self.nodes[s].output)
                    .unwrap_or_else(|| {
                        self.nodes[idx]
                            .input_defaults
                            .get(i)
                            .copied()
                            .unwrap_or(0.0)
                    });
                self.nodes[idx].input_values[i] = val;
            }

            let delay_tap_value =
                if let NodeKind::DelayTap { delay_node, gain } = &self.nodes[idx].kind {
                    if let Some(delay_node_ref) = self.nodes.get(*delay_node) {
                        if let NodeKind::Delay(delay) = &delay_node_ref.kind {
                            Some(delay.tap() * *gain)
                        } else {
                            Some(0.0)
                        }
                    } else {
                        Some(0.0)
                    }
                } else {
                    None
                };

            let node = &mut self.nodes[idx];
            let in0 = node.input_values.first().copied().unwrap_or(0.0);
            let in1 = node.input_values.get(1).copied().unwrap_or(0.0);
            let in2 = node.input_values.get(2).copied().unwrap_or(0.0);
            node.output = match &mut node.kind {
                NodeKind::Freq => freq,
                NodeKind::Gate => gate,
                NodeKind::Degree => degree as f32,
                NodeKind::DegreeGate { target } => {
                    if gate > 0.5 && degree == *target {
                        1.0
                    } else {
                        0.0
                    }
                }
                NodeKind::Oscillator(osc) => {
                    let f = if node.input_values.is_empty() {
                        440.0
                    } else {
                        in0
                    };
                    osc.freq(f)
                        .shift(in1)
                        .gain(if node.input_values.len() > 2 {
                            in2
                        } else {
                            1.0
                        })
                        .output(signal)
                }
                NodeKind::Rise(ramp) | NodeKind::Fall(ramp) => ramp.output(in0, signal),
                NodeKind::Ramp(ramp) => {
                    ramp.value(in0);
                    ramp.output(signal)
                }
                NodeKind::Adsr(adsr) => adsr.output(in0, in1),
                NodeKind::Envelope(env) => env.output(in0),
                NodeKind::Lpf(filter) => filter.output(in0, signal),
                NodeKind::Hpf(filter) => filter.output(in0, signal),
                NodeKind::Comb(comb) => comb.output(in0),
                NodeKind::Allpass(allpass) => allpass.process(in0),
                NodeKind::Delay(delay) => delay.output(in0),
                NodeKind::DelayTap { .. } => delay_tap_value.unwrap_or(0.0),
                NodeKind::Reverb(reverb) => reverb.output(in0),
                NodeKind::Distortion(dist) => dist.output(in0),
                NodeKind::Flanger(flanger) => flanger.output(in0, signal),
                NodeKind::Mul => in0 * in1,
                NodeKind::Add => in0 + in1,
                NodeKind::Gt => {
                    if in0 > in1 {
                        1.0
                    } else {
                        0.0
                    }
                }
                NodeKind::Lt => {
                    if in0 < in1 {
                        1.0
                    } else {
                        0.0
                    }
                }
                NodeKind::Switch => {
                    if in0 <= 0.5 {
                        in1
                    } else {
                        in2
                    }
                }
                NodeKind::Rng { last_gate, value } => {
                    if in0 > 0.5 && *last_gate <= 0.5 {
                        *value = fastrand::f32();
                    }
                    *last_gate = in0;
                    *value
                }
                NodeKind::Sample { samples } => {
                    if samples.is_empty() {
                        0.0
                    } else {
                        let pos = in0.clamp(0.0, 1.0);
                        let idx_f = pos * (samples.len() - 1) as f32;
                        let i = idx_f as usize;
                        let frac = idx_f - i as f32;
                        let s0 = samples[i];
                        let s1 = samples.get(i + 1).copied().unwrap_or(s0);
                        s0 + frac * (s1 - s0)
                    }
                }
                NodeKind::Probe => in0,
                NodeKind::Pass => in0,
                NodeKind::Output => in0,
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
    fn inherit_state_from(&mut self, old: &PatchVoices) {
        for (new_voice, old_voice) in self.voices.iter_mut().zip(old.voices.iter()) {
            new_voice.inherit_state_from(old_voice);
        }
    }

    fn process(&mut self, signal: &mut crate::Signal, track: &TrackState) -> f32 {
        let mut sum = 0.0;
        let n = self.voices.len().min(track.num_voices());
        for i in 0..n {
            let (freq, gate, degree) = track.voice(i);
            sum += self.voices[i].process(signal, freq, gate, degree);
        }
        sum
    }
}

const CROSSFADE_SAMPLES: usize = 2205;

const PROBE_HISTORY_LEN: usize = 44100 * 2;
const METER_INTERVAL: usize = 1024;
pub const OUTPUT_INTERVAL: usize = 1200;

pub struct CompiledPatch {
    current: Option<PatchVoices>,
    old: Option<PatchVoices>,
    pending: Option<PatchVoices>,
    crossfade_pos: usize,
    probe_histories: Vec<VecDeque<f32>>,
    probe_buffers: Vec<Vec<f32>>,
    probe_voice: usize,
    meter_tx: Option<MeterSender>,
    meter_counter: usize,
}

impl Default for CompiledPatch {
    fn default() -> Self {
        Self {
            current: None,
            old: None,
            pending: None,
            crossfade_pos: CROSSFADE_SAMPLES,
            probe_histories: Vec::new(),
            probe_buffers: Vec::new(),
            probe_voice: 0,
            meter_tx: None,
            meter_counter: 0,
        }
    }
}

impl CompiledPatch {
    pub fn set_meter_sender(&mut self, tx: MeterSender) {
        self.meter_tx = Some(tx);
    }
}

impl CompiledPatch {
    fn set_voices(&mut self, voices: Vec<CompiledVoice>) {
        let mut new_patch = PatchVoices { voices };
        if let Some(ref current) = self.current {
            new_patch.inherit_state_from(current);
        }
        if self.crossfade_pos >= CROSSFADE_SAMPLES {
            self.old = self.current.take();
            self.current = Some(new_patch);
            self.crossfade_pos = 0;
        } else {
            self.pending = Some(new_patch);
        }
    }

    fn start_pending_transition(&mut self) {
        if let Some(mut pending) = self.pending.take() {
            if let Some(ref current) = self.current {
                pending.inherit_state_from(current);
            }
            self.old = self.current.take();
            self.current = Some(pending);
            self.crossfade_pos = 0;
        }
    }

    pub fn process(
        &mut self,
        signal: &mut crate::Signal,
        track: &TrackState,
        instrument_idx: usize,
    ) -> f32 {
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
                self.start_pending_transition();
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
                    if matches!(&node.kind, NodeKind::Probe) {
                        while probe_idx >= self.probe_histories.len() {
                            self.probe_histories
                                .push(VecDeque::with_capacity(PROBE_HISTORY_LEN));
                        }
                        while probe_idx >= self.probe_buffers.len() {
                            self.probe_buffers.push(Vec::with_capacity(METER_INTERVAL));
                        }
                        let history = &mut self.probe_histories[probe_idx];
                        let buffer = &mut self.probe_buffers[probe_idx];
                        let val = node.input_values.first().copied().unwrap_or(0.0);
                        history.push_back(val);
                        buffer.push(val);
                        if history.len() > PROBE_HISTORY_LEN {
                            history.pop_front();
                        }
                        probe_idx += 1;
                    }
                }
                self.probe_histories.truncate(probe_idx);
                self.probe_buffers.truncate(probe_idx);

                self.meter_counter += 1;
                if self.meter_counter >= METER_INTERVAL {
                    self.meter_counter = 0;
                    if let Some(ref tx) = self.meter_tx {
                        let ports: Vec<(ModuleId, Vec<f32>)> = voice
                            .nodes
                            .iter()
                            .filter(|n| !n.input_values.is_empty())
                            .map(|n| (n.module_id, n.input_values.clone()))
                            .collect();
                        let probes: Vec<(ModuleId, Vec<f32>)> = voice
                            .nodes
                            .iter()
                            .filter(|n| matches!(&n.kind, NodeKind::Probe))
                            .zip(self.probe_buffers.iter())
                            .map(|(n, buf)| (n.module_id, buf.clone()))
                            .collect();
                        for buf in &mut self.probe_buffers {
                            buf.clear();
                        }
                        let _ = tx.try_send(MeterFrame {
                            instrument_idx,
                            ports,
                            probes,
                            active_pitches: track.active_pitches(),
                        });
                    }
                }
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

pub struct CompileContext {
    pub sample_rate: f32,
    pub bpm: f32,
    pub bars: f32,
}

impl Default for CompileContext {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            bpm: 120.0,
            bars: 1.0,
        }
    }
}

pub fn compile_patch(
    patch: &mut CompiledPatch,
    patches: &PatchSet,
    num_voices: usize,
    ctx: &CompileContext,
) {
    patch.set_voices(compile_voices(patches, num_voices, ctx));
}

pub fn compile_voices(
    patches: &PatchSet,
    num_voices: usize,
    ctx: &CompileContext,
) -> Vec<CompiledVoice> {
    let (modules, connections) = flatten_patchset(patches);
    let module_refs: Vec<&Module> = modules.iter().collect();
    (0..num_voices)
        .map(|_| compile_voice(&module_refs, &connections, ctx))
        .collect()
}

pub struct InstrumentAudio {
    pub patch: CompiledPatch,
    pub track: TrackState,
}

impl InstrumentAudio {
    pub fn new(num_voices: usize) -> Self {
        Self {
            patch: CompiledPatch::default(),
            track: TrackState::new(num_voices),
        }
    }

    pub fn process(&mut self, signal: &mut crate::Signal, instrument_idx: usize) -> f32 {
        self.track.update(signal);
        self.patch.process(signal, &self.track, instrument_idx)
    }
}

pub struct AudioEngine {
    instruments: Vec<InstrumentAudio>,
    cmd_rx: CommandReceiver,
    meter_tx: MeterSender,
    num_voices: usize,
    bpm: f32,
    _sample_rate: f32,
}

impl AudioEngine {
    pub fn new(
        num_voices: usize,
        sample_rate: f32,
        cmd_rx: CommandReceiver,
        meter_tx: MeterSender,
    ) -> Self {
        let mut inst = InstrumentAudio::new(num_voices);
        inst.patch.set_meter_sender(meter_tx.clone());
        Self {
            instruments: vec![inst],
            cmd_rx,
            meter_tx,
            num_voices,
            bpm: 120.0,
            _sample_rate: sample_rate,
        }
    }

    pub fn poll_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            self.handle_command(cmd);
        }
    }

    fn handle_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::SetBpm(bpm) => {
                self.bpm = bpm;
                for inst in &mut self.instruments {
                    inst.track.clock.bpm(bpm);
                }
            }
            AudioCommand::SetInstrument {
                idx,
                voices,
                track,
                bars,
            } => {
                self.ensure_instruments(idx + 1);
                if let Some(inst) = self.instruments.get_mut(idx) {
                    inst.patch.set_voices(voices);
                    inst.track.set_track(track);
                    inst.track.clock.bars(bars);
                }
            }
            AudioCommand::SetVoices { idx, voices, bars } => {
                self.ensure_instruments(idx + 1);
                if let Some(inst) = self.instruments.get_mut(idx) {
                    inst.patch.set_voices(voices);
                    inst.track.clock.bars(bars);
                }
            }
            AudioCommand::SetProbeVoice(voice) => {
                for inst in &mut self.instruments {
                    inst.patch.set_probe_voice(voice);
                }
            }
        }
    }

    fn ensure_instruments(&mut self, count: usize) {
        while self.instruments.len() < count {
            let mut inst = InstrumentAudio::new(self.num_voices);
            inst.patch.set_meter_sender(self.meter_tx.clone());
            inst.track.clock.bpm(self.bpm);
            self.instruments.push(inst);
        }
    }

    pub fn process(&mut self, signal: &mut crate::Signal) -> f32 {
        let mut sum = 0.0;
        for (idx, inst) in self.instruments.iter_mut().enumerate() {
            sum += inst.process(signal, idx);
        }
        sum.clamp(-1.0, 1.0)
    }
}

fn flatten_patchset(patches: &PatchSet) -> (Vec<Module>, Vec<(ModuleId, ModuleId, usize)>) {
    let mut flat_modules: Vec<Module> = Vec::new();
    let mut id_map: HashMap<(Option<SubPatchId>, ModuleId), ModuleId> = HashMap::new();
    let max_root_id = patches
        .root
        .all_modules()
        .map(|m| m.id.0)
        .max()
        .unwrap_or(0);
    let mut next_id = max_root_id + 1;

    for module in patches.root.all_modules() {
        if let ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)) = module.kind {
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
            id_map.insert((None, module.id), module.id);
            flat_modules.push(module.clone());
        }
    }

    let mut connections = Vec::new();

    for module in patches.root.all_modules() {
        if matches!(
            module.kind,
            ModuleKind::Subpatch(SubpatchModule::SubPatch(_))
        ) {
            continue;
        }

        let Some(pos) = patches.root.module_position(module.id) else {
            continue;
        };

        if module.has_output_bottom()
            && let Some((target_id, port_idx)) = trace_down(
                patches.root.grid(),
                &patches.root,
                pos.x,
                pos.y + module.height() as u16,
            )
        {
            let src = id_map.get(&(None, module.id)).copied();
            let dst = id_map.get(&(None, target_id)).copied();
            if let (Some(s), Some(d)) = (src, dst) {
                connections.push((s, d, port_idx));
            }
        }

        if module.has_output_right()
            && let Some((target_id, port_idx)) = trace_right(
                patches.root.grid(),
                &patches.root,
                pos.x + module.width() as u16,
                pos.y,
            )
        {
            let src = id_map.get(&(None, module.id)).copied();
            let dst = id_map.get(&(None, target_id)).copied();
            if let (Some(s), Some(d)) = (src, dst) {
                connections.push((s, d, port_idx));
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
        let ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)) = module.kind else {
            continue;
        };
        let Some(sub) = patches.subpatch(sub_id) else {
            continue;
        };
        let Some(parent_pos) = patches.root.module_position(module.id) else {
            continue;
        };

        let mut sub_inputs: Vec<_> = sub
            .patch
            .all_modules()
            .filter(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubIn))
            .filter_map(|m| {
                let pos = sub.patch.module_position(m.id)?;
                Some((pos.y, m.id))
            })
            .collect();
        sub_inputs.sort_by_key(|(y, _)| *y);
        let sub_inputs: Vec<_> = sub_inputs
            .into_iter()
            .enumerate()
            .map(|(i, (_, id))| (i, id))
            .collect();

        let mut sub_outputs: Vec<_> = sub
            .patch
            .all_modules()
            .filter(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubOut))
            .filter_map(|m| {
                let pos = sub.patch.module_position(m.id)?;
                Some((pos.x, m.id))
            })
            .collect();
        sub_outputs.sort_by_key(|(x, _)| *x);
        let sub_outputs: Vec<_> = sub_outputs
            .into_iter()
            .enumerate()
            .map(|(i, (_, id))| (i, id))
            .collect();

        for src in patches.root.all_modules() {
            if src.id == module.id {
                continue;
            }
            let Some(src_pos) = patches.root.module_position(src.id) else {
                continue;
            };

            if src.has_output_right()
                && module.has_input_left()
                && let Some((target_id, port_idx)) = trace_right(
                    patches.root.grid(),
                    &patches.root,
                    src_pos.x + src.width() as u16,
                    src_pos.y,
                )
                && target_id == module.id
                && let Some(&(_, sub_in_id)) = sub_inputs.iter().find(|(idx, _)| *idx == port_idx)
            {
                let flat_src = id_map.get(&(None, src.id)).copied();
                let flat_dst = id_map.get(&(Some(sub_id), sub_in_id)).copied();
                if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                    connections.push((s, d, 0));
                }
            }

            if src.has_output_bottom()
                && module.has_input_top()
                && let Some((target_id, port_idx)) = trace_down(
                    patches.root.grid(),
                    &patches.root,
                    src_pos.x,
                    src_pos.y + src.height() as u16,
                )
                && target_id == module.id
                && let Some(&(_, sub_in_id)) = sub_inputs.iter().find(|(idx, _)| *idx == port_idx)
            {
                let flat_src = id_map.get(&(None, src.id)).copied();
                let flat_dst = id_map.get(&(Some(sub_id), sub_in_id)).copied();
                if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                    connections.push((s, d, 0));
                }
            }
        }

        let has_sub_outputs = !sub_outputs.is_empty();
        if has_sub_outputs && module.orientation == super::module::Orientation::Vertical {
            for (out_idx, sub_out_id) in &sub_outputs {
                let out_x = parent_pos.x + *out_idx as u16;
                let out_y = parent_pos.y + sub_outputs.len().max(1) as u16;

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

        if has_sub_outputs && module.orientation == super::module::Orientation::Horizontal {
            for (out_idx, sub_out_id) in &sub_outputs {
                let out_x = parent_pos.x + sub_outputs.len().max(1) as u16;
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

            if internal.has_output_bottom()
                && let Some((target_id, port_idx)) = trace_down(
                    sub.patch.grid(),
                    &sub.patch,
                    pos.x,
                    pos.y + internal.height() as u16,
                )
            {
                let flat_src = id_map.get(&(Some(sub_id), internal.id)).copied();
                let flat_dst = id_map.get(&(Some(sub_id), target_id)).copied();
                if let (Some(s), Some(d)) = (flat_src, flat_dst) {
                    connections.push((s, d, port_idx));
                }
            }

            if internal.has_output_right()
                && let Some((target_id, port_idx)) = trace_right(
                    sub.patch.grid(),
                    &sub.patch,
                    pos.x + internal.width() as u16,
                    pos.y,
                )
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

fn compile_voice(
    modules: &[&Module],
    connections: &[(ModuleId, ModuleId, usize)],
    ctx: &CompileContext,
) -> CompiledVoice {
    let mut voice = CompiledVoice {
        nodes: Vec::new(),
        execution_order: Vec::new(),
        output_node: None,
        last_gate: 1.0,
    };
    let mut module_to_node: HashMap<ModuleId, usize> = HashMap::new();

    for module in modules {
        let node_idx = voice.nodes.len();
        module_to_node.insert(module.id, node_idx);

        let kind = create_node_kind(module, ctx);
        let input_defaults = get_input_defaults(module, ctx);
        let input_count = input_defaults.len();

        voice.nodes.push(AudioNode {
            kind,
            module_id: module.id,
            input_sources: vec![None; input_count],
            input_defaults,
            input_values: vec![0.0; input_count],
            output: 0.0,
        });

        if module.kind == ModuleKind::Standard(StandardModule::Output) {
            voice.output_node = Some(node_idx);
        }
    }

    for module in modules {
        if let ModuleKind::Standard(StandardModule::DelayTap(delay_module_id)) = module.kind
            && let Some(&tap_node_idx) = module_to_node.get(&module.id)
            && let Some(&delay_node_idx) = module_to_node.get(&delay_module_id)
            && let NodeKind::DelayTap { delay_node, .. } = &mut voice.nodes[tap_node_idx].kind
        {
            *delay_node = delay_node_idx;
        }
    }

    for (src_id, dst_id, port_idx) in connections {
        if let (Some(&src_node), Some(&dst_node)) =
            (module_to_node.get(src_id), module_to_node.get(dst_id))
            && *port_idx < voice.nodes[dst_node].input_sources.len()
        {
            voice.nodes[dst_node].input_sources[*port_idx] = Some(src_node);
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
            Cell::Empty | Cell::ChannelH { .. } | Cell::ChannelCorner { .. } => return None,
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
            Cell::Empty | Cell::ChannelV { .. } | Cell::ChannelCorner { .. } => return None,
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

fn create_node_kind(module: &Module, ctx: &CompileContext) -> NodeKind {
    use super::module::{ModuleParams, WaveType};

    match (&module.kind, &module.params) {
        (ModuleKind::Routing(_), _) => NodeKind::Pass,
        (ModuleKind::Subpatch(_), _) => NodeKind::Pass,
        (ModuleKind::Standard(StandardModule::Freq), _) => NodeKind::Freq,
        (ModuleKind::Standard(StandardModule::Gate), _) => NodeKind::Gate,
        (ModuleKind::Standard(StandardModule::Degree), _) => NodeKind::Degree,
        (ModuleKind::Standard(StandardModule::DegreeGate), ModuleParams::DegreeGate { degree }) => {
            NodeKind::DegreeGate { target: *degree }
        }
        (ModuleKind::Standard(StandardModule::Osc), ModuleParams::Osc { wave, uni, .. }) => {
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
        (ModuleKind::Standard(StandardModule::Rise), ModuleParams::Rise { time, .. }) => {
            let mut ramp = GateRamp::default();
            ramp.rise();
            ramp.time(time.as_seconds(ctx.bpm, ctx.bars));
            NodeKind::Rise(ramp)
        }
        (ModuleKind::Standard(StandardModule::Fall), ModuleParams::Fall { time, .. }) => {
            let mut ramp = GateRamp::default();
            ramp.fall();
            ramp.time(time.as_seconds(ctx.bpm, ctx.bars));
            NodeKind::Fall(ramp)
        }
        (ModuleKind::Standard(StandardModule::Ramp), ModuleParams::Ramp { time, .. }) => {
            let mut ramp = Ramp::default();
            ramp.time(time.as_seconds(ctx.bpm, ctx.bars));
            NodeKind::Ramp(ramp)
        }
        (
            ModuleKind::Standard(StandardModule::Adsr),
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
        (ModuleKind::Standard(StandardModule::Envelope), ModuleParams::Envelope { points, .. }) => {
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
        (ModuleKind::Standard(StandardModule::Lpf), ModuleParams::Filter { freq, q, .. }) => {
            let mut filter = LowpassFilter::default();
            filter.freq(*freq).q(*q);
            NodeKind::Lpf(filter)
        }
        (ModuleKind::Standard(StandardModule::Hpf), ModuleParams::Filter { freq, q, .. }) => {
            let mut filter = HighpassFilter::default();
            let freq_hz = 20.0 * (1000.0f32).powf(*freq);
            let normalized = freq_hz / 22050.0;
            filter.freq(normalized).q(*q);
            NodeKind::Hpf(filter)
        }
        (
            ModuleKind::Standard(StandardModule::Comb),
            ModuleParams::Comb {
                time,
                feedback,
                damp,
                ..
            },
        ) => {
            let size = time.as_samples(ctx.sample_rate, ctx.bpm, ctx.bars) as usize;
            let mut comb = CombFilter::new(size.max(1));
            comb.feedback(*feedback).damp(*damp);
            NodeKind::Comb(comb)
        }
        (
            ModuleKind::Standard(StandardModule::Allpass),
            ModuleParams::Allpass { time, feedback, .. },
        ) => {
            let size = time.as_samples(ctx.sample_rate, ctx.bpm, ctx.bars) as usize;
            let mut allpass = AllpassFilter::new(size.max(1));
            allpass.feedback(*feedback);
            NodeKind::Allpass(allpass)
        }
        (ModuleKind::Standard(StandardModule::Delay), ModuleParams::Delay { time, .. }) => {
            let mut delay = Delay::default();
            delay.delay(time.as_samples(ctx.sample_rate, ctx.bpm, ctx.bars));
            NodeKind::Delay(delay)
        }
        (ModuleKind::Standard(StandardModule::DelayTap(_)), ModuleParams::DelayTap { gain }) => {
            NodeKind::DelayTap {
                delay_node: usize::MAX,
                gain: *gain,
            }
        }
        (
            ModuleKind::Standard(StandardModule::Reverb),
            ModuleParams::Reverb {
                room,
                damp,
                mod_depth,
                diffusion,
                ..
            },
        ) => {
            let mut reverb = Reverb::default();
            reverb
                .roomsize(*room)
                .damp(*damp)
                .mod_depth(*mod_depth)
                .diffusion(*diffusion);
            NodeKind::Reverb(reverb)
        }
        (
            ModuleKind::Standard(StandardModule::Distortion),
            ModuleParams::Distortion {
                dist_type,
                drive,
                asymmetry,
                ..
            },
        ) => {
            let mut dist = Distortion::default();
            dist.dist_type(dist_type.to_dsp())
                .drive(*drive)
                .asymmetry(*asymmetry);
            NodeKind::Distortion(dist)
        }
        (
            ModuleKind::Standard(StandardModule::Flanger),
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
        (ModuleKind::Standard(StandardModule::Mul), _) => NodeKind::Mul,
        (ModuleKind::Standard(StandardModule::Add), _) => NodeKind::Add,
        (ModuleKind::Standard(StandardModule::Gt), _) => NodeKind::Gt,
        (ModuleKind::Standard(StandardModule::Lt), _) => NodeKind::Lt,
        (ModuleKind::Standard(StandardModule::Switch), _) => NodeKind::Switch,
        (ModuleKind::Standard(StandardModule::Rng), _) => NodeKind::Rng {
            last_gate: 0.0,
            value: 0.0,
        },
        (ModuleKind::Standard(StandardModule::Sample), ModuleParams::Sample { samples, .. }) => {
            NodeKind::Sample {
                samples: samples.clone(),
            }
        }
        (ModuleKind::Standard(StandardModule::Probe), _) => NodeKind::Probe,
        (ModuleKind::Standard(StandardModule::Output), _) => NodeKind::Output,
        (ModuleKind::Standard(StandardModule::DegreeGate), _)
        | (ModuleKind::Standard(StandardModule::Osc), _)
        | (ModuleKind::Standard(StandardModule::Rise), _)
        | (ModuleKind::Standard(StandardModule::Fall), _)
        | (ModuleKind::Standard(StandardModule::Ramp), _)
        | (ModuleKind::Standard(StandardModule::Adsr), _)
        | (ModuleKind::Standard(StandardModule::Envelope), _)
        | (ModuleKind::Standard(StandardModule::Lpf), _)
        | (ModuleKind::Standard(StandardModule::Hpf), _)
        | (ModuleKind::Standard(StandardModule::Comb), _)
        | (ModuleKind::Standard(StandardModule::Allpass), _)
        | (ModuleKind::Standard(StandardModule::Delay), _)
        | (ModuleKind::Standard(StandardModule::DelayTap(_)), _)
        | (ModuleKind::Standard(StandardModule::Reverb), _)
        | (ModuleKind::Standard(StandardModule::Distortion), _)
        | (ModuleKind::Standard(StandardModule::Flanger), _)
        | (ModuleKind::Standard(StandardModule::Sample), _) => {
            unreachable!(
                "ModuleKind {:?} matched with wrong ModuleParams {:?}",
                module.kind, module.params
            )
        }
    }
}

fn get_input_defaults(module: &Module, ctx: &CompileContext) -> Vec<f32> {
    if module.kind.is_routing() {
        return vec![0.0; module.kind.port_count()];
    }

    match &module.kind {
        ModuleKind::Subpatch(SubpatchModule::SubPatch(_)) => return Vec::new(),
        ModuleKind::Subpatch(SubpatchModule::SubIn) => return vec![0.0],
        ModuleKind::Subpatch(SubpatchModule::SubOut) => return vec![0.0],
        ModuleKind::Standard(StandardModule::DelayTap(_)) => return Vec::new(),
        ModuleKind::Routing(_) | ModuleKind::Standard(_) => {}
    }

    let defs = module.kind.param_defs();

    defs.iter()
        .enumerate()
        .filter(|(_, d)| d.kind.is_port())
        .map(|(i, d)| {
            if matches!(d.kind, super::module::ParamKind::Time) {
                module
                    .params
                    .get_time(i)
                    .map(|t| t.as_hz(ctx.bpm, ctx.bars))
                    .unwrap_or(440.0)
            } else {
                module.params.get_float(i).unwrap_or(0.0)
            }
        })
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

        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Osc),
            GridPos::new(1, 0),
        );
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(5, 0),
        );

        let (modules, connections) = flatten_patchset(&patches);

        assert_eq!(modules.len(), 3);
        assert!(connections.iter().any(|(src, dst, _)| {
            let src_kind = modules.iter().find(|m| m.id == *src).map(|m| m.kind);
            let dst_kind = modules.iter().find(|m| m.id == *dst).map(|m| m.kind);
            src_kind == Some(ModuleKind::Standard(StandardModule::Freq))
                && dst_kind == Some(ModuleKind::Standard(StandardModule::Osc))
        }));
    }

    #[test]
    fn test_flatten_with_subpatch() {
        let mut patches = PatchSet::new(20, 20);

        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );

        let sub_id = patches.create_subpatch("Test".into(), Color::Red);

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(
                ModuleKind::Subpatch(SubpatchModule::SubIn),
                GridPos::new(0, 0),
            );
            sub.patch.add_module(
                ModuleKind::Standard(StandardModule::Mul),
                GridPos::new(0, 1),
            );
            sub.patch.add_module(
                ModuleKind::Subpatch(SubpatchModule::SubOut),
                GridPos::new(0, 3),
            );
        }

        patches.root.add_module(
            ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)),
            GridPos::new(0, 1),
        );
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(0, 3),
        );

        let (modules, _connections) = flatten_patchset(&patches);

        assert_eq!(modules.len(), 5);

        assert!(
            modules
                .iter()
                .any(|m| m.kind == ModuleKind::Standard(StandardModule::Freq))
        );
        assert!(
            modules
                .iter()
                .any(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubIn))
        );
        assert!(
            modules
                .iter()
                .any(|m| m.kind == ModuleKind::Standard(StandardModule::Mul))
        );
        assert!(
            modules
                .iter()
                .any(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubOut))
        );
        assert!(
            modules
                .iter()
                .any(|m| m.kind == ModuleKind::Standard(StandardModule::Output))
        );

        assert!(
            !modules
                .iter()
                .any(|m| matches!(m.kind, ModuleKind::Subpatch(SubpatchModule::SubPatch(_))))
        );
    }

    #[test]
    fn test_subpatch_connections() {
        let mut patches = PatchSet::new(20, 20);

        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Freq),
            GridPos::new(0, 0),
        );

        let sub_id = patches.create_subpatch("Test".into(), Color::Red);

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(
                ModuleKind::Subpatch(SubpatchModule::SubIn),
                GridPos::new(0, 0),
            );
            sub.patch.add_module(
                ModuleKind::Subpatch(SubpatchModule::SubOut),
                GridPos::new(1, 0),
            );
        }

        patches.root.add_module(
            ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)),
            GridPos::new(1, 0),
        );
        sync_subpatch_params(&mut patches, sub_id);
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(2, 0),
        );
        patches.root.rebuild_channels();

        let (modules, connections) = flatten_patchset(&patches);

        let freq_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Standard(StandardModule::Freq))
            .map(|m| m.id);
        let sub_in_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubIn))
            .map(|m| m.id);
        let sub_out_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubOut))
            .map(|m| m.id);
        let output_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Standard(StandardModule::Output))
            .map(|m| m.id);

        assert!(
            connections
                .iter()
                .any(|(src, dst, _)| Some(*src) == freq_id && Some(*dst) == sub_in_id),
            "Freq->SubIn connection missing. Connections: {:?}",
            connections
        );

        assert!(
            connections
                .iter()
                .any(|(src, dst, _)| Some(*src) == sub_in_id && Some(*dst) == sub_out_id),
            "SubIn->SubOut connection missing. Connections: {:?}",
            connections
        );

        assert!(
            connections
                .iter()
                .any(|(src, dst, _)| Some(*src) == sub_out_id && Some(*dst) == output_id),
            "SubOut->Output connection missing. Connections: {:?}",
            connections
        );
    }

    fn sync_subpatch_params(patches: &mut PatchSet, sub_id: SubPatchId) {
        let (inputs, outputs) = if let Some(sub) = patches.subpatch(sub_id) {
            (sub.input_count() as u8, sub.output_count() as u8)
        } else {
            return;
        };

        let ids: Vec<ModuleId> = patches
            .root
            .all_modules()
            .filter(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)))
            .map(|m| m.id)
            .collect();

        for id in ids {
            if let Some(m) = patches.root.module_mut(id) {
                m.params = crate::tui::module::ModuleParams::SubPatch { inputs, outputs };
            }
            patches.root.refit_module(id);
        }
    }

    #[test]
    fn test_gate_inside_subpatch() {
        use crate::Signal;
        let mut patches = PatchSet::new(20, 20);

        let sub_id = patches.create_subpatch("Test".into(), Color::Red);

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(
                ModuleKind::Standard(StandardModule::Gate),
                GridPos::new(0, 0),
            );
            sub.patch.add_module(
                ModuleKind::Subpatch(SubpatchModule::SubOut),
                GridPos::new(2, 0),
            );
            sub.patch.rebuild_channels();
        }

        patches.root.add_module(
            ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)),
            GridPos::new(0, 0),
        );
        sync_subpatch_params(&mut patches, sub_id);
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(2, 0),
        );
        patches.root.rebuild_channels();

        let (modules, connections) = flatten_patchset(&patches);

        let gate_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Standard(StandardModule::Gate))
            .map(|m| m.id);
        let sub_out_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Subpatch(SubpatchModule::SubOut))
            .map(|m| m.id);
        let output_id = modules
            .iter()
            .find(|m| m.kind == ModuleKind::Standard(StandardModule::Output))
            .map(|m| m.id);

        assert!(
            gate_id.is_some(),
            "Gate module not found in flattened modules"
        );
        assert!(
            connections
                .iter()
                .any(|(src, dst, _)| Some(*src) == gate_id && Some(*dst) == sub_out_id),
            "Gate->SubOut connection missing. Connections: {:?}",
            connections
        );
        assert!(
            connections
                .iter()
                .any(|(src, dst, _)| Some(*src) == sub_out_id && Some(*dst) == output_id),
            "SubOut->Output connection missing. Connections: {:?}",
            connections
        );

        let module_refs: Vec<&Module> = modules.iter().collect();
        let ctx = CompileContext::default();
        let mut voice = compile_voice(&module_refs, &connections, &ctx);
        let mut signal = Signal::new(44100);

        let output = voice.process(&mut signal, 440.0, 1.0, 0);
        assert!(
            (output - 1.0).abs() < 0.001,
            "Output should be 1.0 (gate), got {}",
            output
        );

        let output = voice.process(&mut signal, 440.0, 0.0, 0);
        assert!(
            output.abs() < 0.001,
            "Output should be 0.0 (gate off), got {}",
            output
        );
    }

    #[test]
    fn test_freq_inside_subpatch() {
        use crate::Signal;
        let mut patches = PatchSet::new(20, 20);

        let sub_id = patches.create_subpatch("Test".into(), Color::Red);

        if let Some(sub) = patches.subpatch_mut(sub_id) {
            sub.patch.add_module(
                ModuleKind::Standard(StandardModule::Freq),
                GridPos::new(0, 0),
            );
            sub.patch.add_module(
                ModuleKind::Subpatch(SubpatchModule::SubOut),
                GridPos::new(2, 0),
            );
            sub.patch.rebuild_channels();
        }

        patches.root.add_module(
            ModuleKind::Subpatch(SubpatchModule::SubPatch(sub_id)),
            GridPos::new(0, 0),
        );
        sync_subpatch_params(&mut patches, sub_id);
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(2, 0),
        );
        patches.root.rebuild_channels();

        let (modules, connections) = flatten_patchset(&patches);
        let module_refs: Vec<&Module> = modules.iter().collect();
        let ctx = CompileContext::default();
        let mut voice = compile_voice(&module_refs, &connections, &ctx);
        let mut signal = Signal::new(44100);

        let output = voice.process(&mut signal, 440.0, 1.0, 0);
        assert!(
            (output - 440.0).abs() < 0.001,
            "Output should be 440.0 (freq), got {}",
            output
        );

        let output = voice.process(&mut signal, 880.0, 1.0, 0);
        assert!(
            (output - 880.0).abs() < 0.001,
            "Output should be 880.0 (freq), got {}",
            output
        );
    }

    #[test]
    fn test_delay_tap_linking() {
        let mut patches = PatchSet::new(20, 20);

        let delay_id = patches
            .root
            .add_module(
                ModuleKind::Standard(StandardModule::Delay),
                GridPos::new(0, 0),
            )
            .unwrap();
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::DelayTap(delay_id)),
            GridPos::new(3, 0),
        );
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(5, 0),
        );

        let (modules, connections) = flatten_patchset(&patches);
        let module_refs: Vec<&Module> = modules.iter().collect();
        let ctx = CompileContext::default();

        let voice = compile_voice(&module_refs, &connections, &ctx);

        let tap_node_idx = voice
            .nodes
            .iter()
            .position(|n| matches!(n.kind, NodeKind::DelayTap { .. }));
        let delay_node_idx = voice
            .nodes
            .iter()
            .position(|n| matches!(n.kind, NodeKind::Delay(_)));

        assert!(tap_node_idx.is_some(), "DelayTap node not found");
        assert!(delay_node_idx.is_some(), "Delay node not found");

        if let NodeKind::DelayTap { delay_node, .. } = &voice.nodes[tap_node_idx.unwrap()].kind {
            assert_eq!(
                *delay_node,
                delay_node_idx.unwrap(),
                "DelayTap delay_node {} doesn't match Delay node index {}",
                delay_node,
                delay_node_idx.unwrap()
            );
        } else {
            panic!("Expected DelayTap node kind");
        }
    }

    #[test]
    fn test_delay_tap_produces_output() {
        use crate::Signal;
        let mut patches = PatchSet::new(20, 20);

        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Osc),
            GridPos::new(0, 0),
        );
        let delay_id = patches
            .root
            .add_module(
                ModuleKind::Standard(StandardModule::Delay),
                GridPos::new(2, 0),
            )
            .unwrap();
        if let Some(m) = patches.root.module_mut(delay_id) {
            m.params = crate::tui::module::ModuleParams::Delay {
                time: crate::tui::module::TimeValue::from_samples(100.0),
                connected: 0xFF,
            };
        }
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::DelayTap(delay_id)),
            GridPos::new(4, 0),
        );
        patches.root.add_module(
            ModuleKind::Standard(StandardModule::Output),
            GridPos::new(6, 0),
        );
        patches.root.rebuild_channels();

        let (modules, connections) = flatten_patchset(&patches);
        let module_refs: Vec<&Module> = modules.iter().collect();
        let ctx = CompileContext::default();

        let mut voice = compile_voice(&module_refs, &connections, &ctx);
        let mut signal = Signal::new(44100);

        for _ in 0..1000 {
            voice.process(&mut signal, 440.0, 1.0, 0);
        }

        let tap_node_idx = voice
            .nodes
            .iter()
            .position(|n| matches!(n.kind, NodeKind::DelayTap { .. }))
            .unwrap();
        let delay_node_idx = voice
            .nodes
            .iter()
            .position(|n| matches!(n.kind, NodeKind::Delay(_)))
            .unwrap();

        assert!(
            voice.nodes[delay_node_idx].output.abs() > 0.001,
            "Delay should have output"
        );
        eprintln!("Delay output: {}", voice.nodes[delay_node_idx].output);

        assert!(
            voice.nodes[delay_node_idx].output.abs() > 0.001,
            "Delay should have output"
        );
        assert!(
            voice.nodes[tap_node_idx].output.abs() > 0.001,
            "DelayTap should have output"
        );
    }
}
