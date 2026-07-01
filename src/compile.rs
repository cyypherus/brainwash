use crate::delay::Delay;
use crate::effect::{Distortion, Drive};
use crate::osc::{Oscillator, Wave};
use crate::patch::{BinaryOp, EnvPoint, Module, Patch};
use crate::sample::{Frame, Sample, Unit};
use crate::time::{Duration, Hertz, SampleRate};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPatch {
    nodes: Vec<Node>,
    inputs: Vec<Vec<Option<usize>>>,
    values: Vec<Sample>,
    output: usize,
    probes: Vec<(crate::patch::ModuleId, usize)>,
    meters: Vec<(crate::patch::ModuleId, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchEngine {
    active: CompiledPatch,
    transition: Option<Transition>,
    retired: Option<CompiledPatch>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompileError {
    InvalidDelay,
    InvalidInput,
    MissingModule,
    MissingOutput,
    Cycle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchControls {
    pub frequency: Option<Hertz>,
    pub gate: f32,
    pub degree: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateRejected<T = CompiledPatch> {
    Busy(T),
    RetiredPatchPending(T),
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Silence,
    Freq,
    Gate,
    Degree,
    DegreeGate {
        target: i32,
    },
    Constant(Sample),
    Pass,
    Osc {
        osc: Oscillator,
        frequency: Hertz,
        shift: f32,
        gain: Unit,
        unipolar: bool,
    },
    Rise(GateRamp),
    Fall(GateRamp),
    Ramp(Ramp),
    Adsr(Adsr),
    Envelope {
        points: Arc<Vec<EnvPoint>>,
    },
    Lowpass(OnePole),
    Highpass(OnePole),
    Comb(Comb),
    Allpass(Allpass),
    Delay(Delay),
    DelayTap {
        gain: Unit,
    },
    Reverb(Reverb),
    Distortion {
        kind: Distortion,
        drive: Drive,
    },
    Compressor(Compressor),
    Flanger(Flanger),
    Binary {
        op: BinaryOp,
        a: f32,
        b: f32,
    },
    Switch {
        a: f32,
        b: f32,
    },
    Random {
        last_gate: f32,
        value: f32,
        seed: u32,
    },
    Sample {
        samples: Arc<Vec<f32>>,
    },
    Probe,
}

#[derive(Clone, Debug, PartialEq)]
struct Transition {
    old: CompiledPatch,
    position: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GateRamp {
    mode: GateRampMode,
    samples: u64,
    elapsed: u64,
    value: f32,
    last_gate: f32,
    active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GateRampMode {
    Rise,
    Fall,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ramp {
    samples: u64,
    current: f32,
    target: f32,
    start: f32,
    elapsed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Adsr {
    attack_ratio: Unit,
    sustain: Unit,
    release_start: f32,
    last_rise: f32,
    last_fall: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OnePole {
    rate: SampleRate,
    cutoff: Hertz,
    value: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    feedback: Unit,
    damp: Unit,
    store: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Allpass {
    buffer: Vec<f32>,
    index: usize,
    feedback: Unit,
}

#[derive(Clone, Debug, PartialEq)]
struct Reverb {
    delays: Vec<Vec<f32>>,
    indices: Vec<usize>,
    room: Unit,
    damp: Unit,
    mod_depth: Unit,
    diffusion: Unit,
    store: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Compressor {
    threshold: Unit,
    ratio: f32,
    attack: Duration,
    release: Duration,
    makeup: f32,
    envelope: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Flanger {
    buffer: Vec<f32>,
    index: usize,
    phase: f32,
    rate: Hertz,
    depth: Unit,
    feedback: Unit,
    last: f32,
}

const UPDATE_FADE_FRAMES: usize = 128;

impl Default for PatchControls {
    fn default() -> Self {
        Self {
            frequency: None,
            gate: 0.0,
            degree: 0,
        }
    }
}

impl CompiledPatch {
    pub fn silence() -> Self {
        Self {
            nodes: vec![Node::Silence],
            inputs: vec![Vec::new()],
            values: vec![Sample::ZERO],
            output: 0,
            probes: Vec::new(),
            meters: Vec::new(),
        }
    }

    pub fn new(patch: &Patch, rate: SampleRate) -> Result<Self, CompileError> {
        let order = compile_order(patch)?;
        let mut rank_by_module = Vec::with_capacity(order.len());
        for (rank, module_index) in order.iter().copied().enumerate() {
            rank_by_module.push((patch.modules()[module_index].0, rank));
        }

        let mut inputs = Vec::with_capacity(order.len());
        for module_index in order.iter().copied() {
            inputs.push(vec![None; input_count(&patch.modules()[module_index].1)]);
        }

        for connection in patch.connections() {
            let source = rank_by_module
                .iter()
                .find_map(|(id, rank)| (*id == connection.from).then_some(*rank))
                .ok_or(CompileError::MissingModule)?;
            let target = rank_by_module
                .iter()
                .find_map(|(id, rank)| (*id == connection.to).then_some(*rank))
                .ok_or(CompileError::MissingModule)?;
            let slot = inputs
                .get_mut(target)
                .and_then(|inputs| inputs.get_mut(connection.input))
                .ok_or(CompileError::InvalidInput)?;
            *slot = Some(source);
        }

        let probes = order
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(rank, module_index)| {
                let (id, module) = &patch.modules()[module_index];
                matches!(module, Module::Probe).then_some((*id, rank))
            })
            .collect();
        let meters = order
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(rank, module_index)| {
                let (id, module) = &patch.modules()[module_index];
                (input_count(module) > 0).then_some((*id, rank))
            })
            .collect();

        let mut nodes = Vec::with_capacity(order.len());
        for module_index in order.iter().copied() {
            nodes.push(create_node(&patch.modules()[module_index].1, rate)?);
        }
        let output = patch
            .output_id()
            .and_then(|id| {
                rank_by_module
                    .iter()
                    .find_map(|(module_id, rank)| (*module_id == id).then_some(*rank))
            })
            .ok_or(CompileError::MissingOutput)?;
        Ok(Self {
            nodes,
            inputs,
            values: vec![Sample::ZERO; patch.modules().len()],
            output,
            probes,
            meters,
        })
    }

    pub fn next(&mut self) -> Frame {
        self.next_with_controls(PatchControls::default())
    }

    pub fn next_with_controls(&mut self, controls: PatchControls) -> Frame {
        for idx in 0..self.nodes.len() {
            let inputs = input_values(&self.inputs[idx], &self.values);
            self.values[idx] = self.nodes[idx].process(&inputs, controls);
        }
        Frame::mono(self.values[self.output].clipped())
    }

    pub fn probe_values(&self) -> impl Iterator<Item = (crate::patch::ModuleId, Sample)> + '_ {
        self.probes
            .iter()
            .map(|(id, index)| (*id, self.values[*index]))
    }

    fn visit_input_values(&self, mut visitor: impl FnMut(crate::patch::ModuleId, usize, Sample)) {
        for (id, index) in &self.meters {
            for (input, source) in self.inputs[*index].iter().copied().enumerate() {
                visitor(
                    *id,
                    input,
                    source
                        .map(|source| self.values[source])
                        .unwrap_or(Sample::ZERO),
                );
            }
        }
    }
}

impl PatchEngine {
    pub fn new(active: CompiledPatch) -> Self {
        Self {
            active,
            transition: None,
            retired: None,
        }
    }

    pub fn replace(&mut self, next: CompiledPatch) -> Result<(), UpdateRejected> {
        if self.transition.is_some() {
            return Err(UpdateRejected::Busy(next));
        }
        if self.retired.is_some() {
            return Err(UpdateRejected::RetiredPatchPending(next));
        }
        let old = std::mem::replace(&mut self.active, next);
        self.transition = Some(Transition { old, position: 0 });
        Ok(())
    }

    pub fn take_retired(&mut self) -> Option<CompiledPatch> {
        self.retired.take()
    }

    pub fn can_replace(&self) -> bool {
        self.transition.is_none() && self.retired.is_none()
    }

    pub fn retired_pending(&self) -> bool {
        self.retired.is_some()
    }

    pub fn next(&mut self) -> Frame {
        self.next_with_controls(PatchControls::default())
    }

    pub fn next_with_controls(&mut self, controls: PatchControls) -> Frame {
        let next = self.active.next_with_controls(controls);
        let Some(transition) = &mut self.transition else {
            return next;
        };
        let old = transition.old.next_with_controls(controls);
        let frame = fade(old, next, transition.position);
        transition.position += 1;
        if transition.position >= UPDATE_FADE_FRAMES {
            let transition = self.transition.take().unwrap();
            self.retired = Some(transition.old);
        }
        frame
    }

    pub fn probe_values(&self) -> impl Iterator<Item = (crate::patch::ModuleId, Sample)> + '_ {
        self.active.probe_values()
    }

    pub fn visit_input_values(&self, visitor: impl FnMut(crate::patch::ModuleId, usize, Sample)) {
        self.active.visit_input_values(visitor);
    }
}

impl Node {
    fn process(&mut self, inputs: &[Option<Sample>], controls: PatchControls) -> Sample {
        let in0 = sample(inputs, 0);
        let in1 = sample(inputs, 1);
        match self {
            Node::Silence => Sample::ZERO,
            Node::Freq => Sample::new(
                controls
                    .frequency
                    .map(|frequency| frequency.value())
                    .unwrap_or(0.0),
            )
            .unwrap_or(Sample::ZERO),
            Node::Gate => Sample::new(controls.gate).unwrap_or(Sample::ZERO),
            Node::Degree => Sample::new(controls.degree as f32).unwrap_or(Sample::ZERO),
            Node::DegreeGate { target } => {
                if controls.gate > 0.5 && controls.degree == *target {
                    Sample::raw(1.0)
                } else {
                    Sample::ZERO
                }
            }
            Node::Constant(value) => *value,
            Node::Pass => Sample::raw(inputs.iter().flatten().map(|input| input.value()).sum()),
            Node::Osc {
                osc,
                frequency,
                shift,
                gain,
                unipolar,
            } => {
                let frequency = inputs
                    .first()
                    .and_then(|input| input.and_then(|input| Hertz::new(input.value())))
                    .unwrap_or(*frequency);
                let shift = inputs
                    .get(1)
                    .and_then(|input| input.map(|input| input.value()))
                    .unwrap_or(*shift);
                let gain = inputs
                    .get(2)
                    .and_then(|input| input.and_then(|input| Unit::new(input.value())))
                    .unwrap_or(*gain);
                if let Some(frequency) = Hertz::new(frequency.value() * 2.0_f32.powf(shift / 12.0))
                {
                    osc.set_frequency(frequency);
                }
                let mut value = osc.next().value();
                if *unipolar {
                    value = (value + 1.0) * 0.5;
                }
                Sample::raw(value * gain.value())
            }
            Node::Rise(ramp) | Node::Fall(ramp) => Sample::raw(ramp.next(in0.value())),
            Node::Ramp(ramp) => Sample::raw(ramp.next(in0.value())),
            Node::Adsr(adsr) => Sample::raw(adsr.next(in0.value(), in1.value())),
            Node::Envelope { points } => Sample::raw(envelope_value(points, in0.value())),
            Node::Lowpass(filter) => filter.lowpass(in0),
            Node::Highpass(filter) => filter.highpass(in0),
            Node::Comb(comb) => Sample::raw(comb.next(in0.value())),
            Node::Allpass(allpass) => Sample::raw(allpass.next(in0.value())),
            Node::Delay(delay) => delay.process(in0),
            Node::DelayTap { gain } => in0.attenuate(*gain),
            Node::Reverb(reverb) => Sample::raw(reverb.next(in0.value())),
            Node::Distortion { kind, drive } => kind.process(in0, *drive),
            Node::Compressor(compressor) => Sample::raw(compressor.next(in0.value())),
            Node::Flanger(flanger) => Sample::raw(flanger.next(in0.value())),
            Node::Binary { op, a, b } => {
                let a = inputs
                    .first()
                    .and_then(|input| input.map(|input| input.value()))
                    .unwrap_or(*a);
                let b = inputs
                    .get(1)
                    .and_then(|input| input.map(|input| input.value()))
                    .unwrap_or(*b);
                Sample::raw(match op {
                    BinaryOp::Multiply => a * b,
                    BinaryOp::Add => a + b,
                    BinaryOp::GreaterThan => {
                        if a > b {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    BinaryOp::LessThan => {
                        if a < b {
                            1.0
                        } else {
                            0.0
                        }
                    }
                })
            }
            Node::Switch { a, b } => {
                let a = inputs
                    .get(1)
                    .and_then(|input| input.map(|input| input.value()))
                    .unwrap_or(*a);
                let b = inputs
                    .get(2)
                    .and_then(|input| input.map(|input| input.value()))
                    .unwrap_or(*b);
                if in0.value() <= 0.5 {
                    Sample::raw(a)
                } else {
                    Sample::raw(b)
                }
            }
            Node::Random {
                last_gate,
                value,
                seed,
            } => {
                if in0.value() > 0.5 && *last_gate <= 0.5 {
                    *seed = seed.wrapping_mul(196314165).wrapping_add(907633515);
                    *value = *seed as f32 / u32::MAX as f32;
                }
                *last_gate = in0.value();
                Sample::raw(*value)
            }
            Node::Sample { samples } => Sample::raw(sample_value(samples, in0.value())),
            Node::Probe => in0,
        }
    }
}

impl GateRamp {
    fn new(mode: GateRampMode, time: Duration, rate: SampleRate) -> Self {
        Self {
            mode,
            samples: time.samples(rate).value().max(1),
            elapsed: 0,
            value: 0.0,
            last_gate: 0.0,
            active: false,
        }
    }

    fn next(&mut self, gate: f32) -> f32 {
        let pressed = gate > 0.5;
        match self.mode {
            GateRampMode::Rise => {
                if pressed && self.last_gate <= 0.5 {
                    self.elapsed = 0;
                    self.active = true;
                    self.value = 0.0;
                }
                if pressed && self.active {
                    self.value = (self.elapsed as f32 / self.samples as f32).clamp(0.0, 1.0);
                    self.elapsed = self.elapsed.saturating_add(1);
                }
            }
            GateRampMode::Fall => {
                if pressed {
                    self.value = 0.0;
                    self.active = false;
                    self.elapsed = 0;
                } else if self.last_gate > 0.5 {
                    self.active = true;
                    self.elapsed = 0;
                    self.value = 0.0;
                } else if self.active {
                    self.value = (self.elapsed as f32 / self.samples as f32).clamp(0.0, 1.0);
                    self.elapsed = self.elapsed.saturating_add(1);
                }
            }
        }
        self.last_gate = gate;
        self.value
    }
}

impl Ramp {
    fn new(value: f32, time: Duration, rate: SampleRate) -> Self {
        Self {
            samples: time.samples(rate).value().max(1),
            current: value,
            target: value,
            start: value,
            elapsed: 0,
        }
    }

    fn next(&mut self, target: f32) -> f32 {
        if (target - self.target).abs() > f32::EPSILON {
            self.start = self.current;
            self.target = target;
            self.elapsed = 0;
        }
        if self.elapsed >= self.samples {
            self.current = self.target;
        } else {
            let amount = self.elapsed as f32 / self.samples as f32;
            self.current = self.start + (self.target - self.start) * amount;
            self.elapsed = self.elapsed.saturating_add(1);
        }
        self.current
    }
}

impl Adsr {
    fn next(&mut self, rise: f32, fall: f32) -> f32 {
        let rise = rise.clamp(0.0, 1.0);
        let fall = fall.clamp(0.0, 1.0);
        if fall < self.last_fall {
            self.release_start = self.ads(rise);
        }
        self.last_rise = rise;
        self.last_fall = fall;
        if fall > 0.0 {
            self.release_start * (1.0 - fall)
        } else {
            self.ads(rise)
        }
    }

    fn ads(self, rise: f32) -> f32 {
        let attack = self.attack_ratio.value();
        let sustain = self.sustain.value();
        if attack <= 0.0 {
            sustain
        } else if rise < attack {
            rise / attack
        } else if attack >= 1.0 {
            1.0
        } else {
            let decay = (rise - attack) / (1.0 - attack);
            1.0 + (sustain - 1.0) * decay
        }
    }
}

impl OnePole {
    fn lowpass(&mut self, input: Sample) -> Sample {
        let alpha = (self.cutoff.value() / self.rate.value() as f32).clamp(0.0, 1.0);
        self.value += (input.value() - self.value) * alpha;
        Sample::raw(self.value)
    }

    fn highpass(&mut self, input: Sample) -> Sample {
        input.sub(self.lowpass(input))
    }
}

impl Comb {
    fn new(rate: SampleRate, time: Duration, feedback: Unit, damp: Unit) -> Self {
        Self {
            buffer: vec![0.0; time.samples(rate).value().max(1) as usize],
            index: 0,
            feedback,
            damp,
            store: 0.0,
        }
    }

    fn next(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        self.store = output * (1.0 - self.damp.value()) + self.store * self.damp.value();
        self.buffer[self.index] = input + self.store * self.feedback.value();
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

impl Allpass {
    fn new(rate: SampleRate, time: Duration, feedback: Unit) -> Self {
        Self {
            buffer: vec![0.0; time.samples(rate).value().max(1) as usize],
            index: 0,
            feedback,
        }
    }

    fn next(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.index];
        let output = -input + delayed;
        self.buffer[self.index] = input + delayed * self.feedback.value();
        self.index = (self.index + 1) % self.buffer.len();
        output
    }
}

impl Reverb {
    fn new(rate: SampleRate, room: Unit, damp: Unit, mod_depth: Unit, diffusion: Unit) -> Self {
        let scale = rate.value() as f32 / 44_100.0;
        let sizes = [149, 211, 263, 293]
            .iter()
            .map(|size| ((*size as f32 * scale).round() as usize).max(1))
            .collect::<Vec<_>>();
        Self {
            delays: sizes.iter().map(|size| vec![0.0; *size]).collect(),
            indices: vec![0; sizes.len()],
            room,
            damp,
            mod_depth,
            diffusion,
            store: vec![0.0; sizes.len()],
        }
    }

    fn next(&mut self, input: f32) -> f32 {
        let mut sum = 0.0;
        for idx in 0..self.delays.len() {
            let position = self.indices[idx];
            let delayed = self.delays[idx][position];
            self.store[idx] =
                delayed * (1.0 - self.damp.value()) + self.store[idx] * self.damp.value();
            self.delays[idx][position] =
                input * self.diffusion.value() + self.store[idx] * self.room.value();
            self.indices[idx] = (position + 1) % self.delays[idx].len();
            sum += delayed;
        }
        input * (1.0 - self.mod_depth.value() * 0.5) + sum * 0.25 * self.mod_depth.value()
    }
}

impl Compressor {
    fn next(&mut self, input: f32) -> f32 {
        let level = input.abs();
        let attack = duration_seconds(self.attack).max(0.0001);
        let release = duration_seconds(self.release).max(0.0001);
        let coeff = if level > self.envelope {
            (-1.0 / (attack * 44_100.0)).exp()
        } else {
            (-1.0 / (release * 44_100.0)).exp()
        };
        self.envelope = coeff * self.envelope + (1.0 - coeff) * level;
        if self.envelope <= self.threshold.value() {
            return input * self.makeup;
        }
        let over = self.envelope / self.threshold.value().max(0.0001);
        let gain = over.powf((1.0 / self.ratio.max(1.0)) - 1.0);
        input * gain * self.makeup
    }
}

impl Flanger {
    fn new(rate: SampleRate, frequency: Hertz, depth: Unit, feedback: Unit) -> Self {
        Self {
            buffer: vec![0.0; (rate.value() / 40).max(2) as usize],
            index: 0,
            phase: 0.0,
            rate: frequency,
            depth,
            feedback,
            last: 0.0,
        }
    }

    fn next(&mut self, input: f32) -> f32 {
        self.phase += self.rate.value() / 44_100.0;
        self.phase -= self.phase.floor();
        let depth = self.depth.value();
        let delay = 2.0
            + (self.buffer.len() as f32 - 3.0)
                * depth
                * (self.phase * std::f32::consts::TAU).sin().mul_add(0.5, 0.5);
        let read = (self.index + self.buffer.len() - delay as usize % self.buffer.len())
            % self.buffer.len();
        let delayed = self.buffer[read];
        self.buffer[self.index] = input + self.last * self.feedback.value();
        self.index = (self.index + 1) % self.buffer.len();
        self.last = delayed;
        input + delayed
    }
}

fn fade(old: Frame, next: Frame, position: usize) -> Frame {
    let amount = (position as f32 / (UPDATE_FADE_FRAMES - 1) as f32).clamp(0.0, 1.0);
    Frame::stereo(
        old.left()
            .scale(1.0 - amount)
            .add(next.left().scale(amount)),
        old.right()
            .scale(1.0 - amount)
            .add(next.right().scale(amount)),
    )
}

fn compile_order(patch: &Patch) -> Result<Vec<usize>, CompileError> {
    let mut indegree = vec![0usize; patch.modules().len()];
    let mut outgoing = vec![Vec::new(); patch.modules().len()];
    for connection in patch.connections() {
        let source = module_index(patch, connection.from)?;
        let target = module_index(patch, connection.to)?;
        outgoing[source].push(target);
        indegree[target] += 1;
    }

    let mut ready = indegree
        .iter()
        .enumerate()
        .filter_map(|(idx, count)| (*count == 0).then_some(idx))
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(indegree.len());
    while let Some(idx) = ready.pop_front() {
        order.push(idx);
        for target in &outgoing[idx] {
            indegree[*target] -= 1;
            if indegree[*target] == 0 {
                ready.push_back(*target);
            }
        }
    }

    if order.len() == patch.modules().len() {
        Ok(order)
    } else {
        Err(CompileError::Cycle)
    }
}

fn module_index(patch: &Patch, id: crate::patch::ModuleId) -> Result<usize, CompileError> {
    patch
        .modules()
        .iter()
        .position(|(module_id, _)| *module_id == id)
        .ok_or(CompileError::MissingModule)
}

fn create_node(module: &Module, rate: SampleRate) -> Result<Node, CompileError> {
    Ok(match module {
        Module::Freq => Node::Freq,
        Module::Gate => Node::Gate,
        Module::Degree => Node::Degree,
        Module::DegreeGate { target } => Node::DegreeGate { target: *target },
        Module::Constant(value) => Node::Constant(Sample::new(*value).unwrap_or(Sample::ZERO)),
        Module::Pass => Node::Pass,
        Module::Osc {
            wave,
            frequency,
            shift,
            gain,
            unipolar,
        } => Node::Osc {
            osc: Oscillator::new(audio_wave(*wave), rate, *frequency),
            frequency: *frequency,
            shift: *shift,
            gain: *gain,
            unipolar: *unipolar,
        },
        Module::Rise { time } => Node::Rise(GateRamp::new(GateRampMode::Rise, *time, rate)),
        Module::Fall { time } => Node::Fall(GateRamp::new(GateRampMode::Fall, *time, rate)),
        Module::Ramp { value, time } => Node::Ramp(Ramp::new(*value, *time, rate)),
        Module::Adsr {
            attack_ratio,
            sustain,
        } => Node::Adsr(Adsr {
            attack_ratio: *attack_ratio,
            sustain: *sustain,
            release_start: 0.0,
            last_rise: 0.0,
            last_fall: 1.0,
        }),
        Module::Envelope { points } => Node::Envelope {
            points: Arc::clone(points),
        },
        Module::Lowpass { cutoff } => Node::Lowpass(OnePole {
            rate,
            cutoff: *cutoff,
            value: 0.0,
        }),
        Module::Highpass { cutoff } => Node::Highpass(OnePole {
            rate,
            cutoff: *cutoff,
            value: 0.0,
        }),
        Module::Comb {
            time,
            feedback,
            damp,
        } => Node::Comb(Comb::new(rate, *time, *feedback, *damp)),
        Module::Allpass { time, feedback } => Node::Allpass(Allpass::new(rate, *time, *feedback)),
        Module::Delay { time, feedback } => {
            Node::Delay(Delay::new(rate, *time, *feedback).ok_or(CompileError::InvalidDelay)?)
        }
        Module::DelayTap { gain } => Node::DelayTap { gain: *gain },
        Module::Reverb {
            room,
            damp,
            mod_depth,
            diffusion,
        } => Node::Reverb(Reverb::new(rate, *room, *damp, *mod_depth, *diffusion)),
        Module::Distortion { kind, drive } => Node::Distortion {
            kind: match kind {
                crate::patch::Distortion::Clip => Distortion::Clip,
                crate::patch::Distortion::Tanh => Distortion::Tanh,
                crate::patch::Distortion::Fold => Distortion::Fold,
            },
            drive: Drive::new(drive.value()).unwrap(),
        },
        Module::Compressor {
            threshold,
            ratio,
            attack,
            release,
            makeup,
        } => Node::Compressor(Compressor {
            threshold: *threshold,
            ratio: *ratio,
            attack: *attack,
            release: *release,
            makeup: *makeup,
            envelope: 0.0,
        }),
        Module::Flanger {
            rate: frequency,
            depth,
            feedback,
        } => Node::Flanger(Flanger::new(rate, *frequency, *depth, *feedback)),
        Module::Binary { op, a, b } => Node::Binary {
            op: *op,
            a: *a,
            b: *b,
        },
        Module::Switch { a, b } => Node::Switch { a: *a, b: *b },
        Module::Random => Node::Random {
            last_gate: 0.0,
            value: 0.0,
            seed: 0x1234_5678,
        },
        Module::Sample { samples } => Node::Sample {
            samples: Arc::clone(samples),
        },
        Module::Probe => Node::Probe,
    })
}

fn input_count(module: &Module) -> usize {
    match module {
        Module::Freq
        | Module::Gate
        | Module::Degree
        | Module::DegreeGate { .. }
        | Module::Constant(_)
        | Module::Random => 0,
        Module::Pass
        | Module::Rise { .. }
        | Module::Fall { .. }
        | Module::Ramp { .. }
        | Module::Envelope { .. }
        | Module::Lowpass { .. }
        | Module::Highpass { .. }
        | Module::Comb { .. }
        | Module::Allpass { .. }
        | Module::Delay { .. }
        | Module::DelayTap { .. }
        | Module::Reverb { .. }
        | Module::Distortion { .. }
        | Module::Compressor { .. }
        | Module::Flanger { .. }
        | Module::Sample { .. }
        | Module::Probe => 1,
        Module::Binary { .. } | Module::Adsr { .. } => 2,
        Module::Osc { .. } | Module::Switch { .. } => 3,
    }
}

fn input_values(input_slots: &[Option<usize>], values: &[Sample]) -> [Option<Sample>; 3] {
    let mut inputs = [None; 3];
    for (index, source) in input_slots.iter().copied().take(inputs.len()).enumerate() {
        inputs[index] = source.and_then(|source| values.get(source).copied());
    }
    inputs
}

fn sample(inputs: &[Option<Sample>], index: usize) -> Sample {
    inputs.get(index).copied().flatten().unwrap_or(Sample::ZERO)
}

fn audio_wave(wave: crate::patch::Wave) -> Wave {
    match wave {
        crate::patch::Wave::Sine => Wave::Sine,
        crate::patch::Wave::Square => Wave::Square,
        crate::patch::Wave::Triangle => Wave::Triangle,
        crate::patch::Wave::Saw => Wave::Saw,
        crate::patch::Wave::Noise => Wave::Noise,
    }
}

fn duration_seconds(duration: Duration) -> f32 {
    match duration {
        Duration::Seconds(seconds) => seconds.value(),
        Duration::Samples(samples) => samples.value() as f32 / 44_100.0,
    }
}

fn envelope_value(points: &[EnvPoint], time: f32) -> f32 {
    let time = time.clamp(0.0, 1.0);
    if points.is_empty() {
        return 0.0;
    }
    if points.len() == 1 || time <= points[0].time {
        return points[0].value;
    }
    let last = points.len() - 1;
    if time >= points[last].time {
        return points[last].value;
    }
    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        if time < start.time || time > end.time {
            continue;
        }
        let span = (end.time - start.time).max(f32::EPSILON);
        let mut amount = (time - start.time) / span;
        amount = match (start.curve, end.curve) {
            (false, false) => amount,
            (true, false) => 1.0 - (1.0 - amount) * (1.0 - amount),
            (false, true) => amount * amount,
            (true, true) if amount < 0.5 => 2.0 * amount * amount,
            (true, true) => 1.0 - 2.0 * (1.0 - amount) * (1.0 - amount),
        };
        return start.value + (end.value - start.value) * amount;
    }
    points[last].value
}

fn sample_value(samples: &[f32], position: f32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let position = position.clamp(0.0, 1.0);
    let index = position * (samples.len() - 1) as f32;
    let base = index.floor() as usize;
    let amount = index - base as f32;
    let a = samples[base];
    let b = samples.get(base + 1).copied().unwrap_or(a);
    a + (b - a) * amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{Module, Patch};
    use crate::time::Hertz;

    fn patch(rate: f32) -> Patch {
        let mut patch = Patch::new();
        let osc = patch.insert(Module::Osc {
            wave: crate::patch::Wave::Sine,
            frequency: Hertz::new(rate).unwrap(),
            shift: 0.0,
            gain: Unit::ONE,
            unipolar: false,
        });
        patch.output(osc).unwrap();
        patch
    }

    #[test]
    fn compile_requires_output() {
        let mut patch = Patch::new();
        patch.insert(Module::Osc {
            wave: crate::patch::Wave::Sine,
            frequency: Hertz::new(440.0).unwrap(),
            shift: 0.0,
            gain: Unit::ONE,
            unipolar: false,
        });
        let rate = SampleRate::new(44_100).unwrap();

        assert_eq!(
            CompiledPatch::new(&patch, rate),
            Err(CompileError::MissingOutput)
        );
    }

    #[test]
    fn compile_rejects_invalid_input_slot() {
        let mut patch = Patch::new();
        let freq = patch.insert(Module::Freq);
        let gate = patch.insert(Module::Gate);
        patch.connect_input(freq, gate, 0).unwrap();
        patch.output(gate).unwrap();
        let rate = SampleRate::new(44_100).unwrap();

        assert_eq!(
            CompiledPatch::new(&patch, rate),
            Err(CompileError::InvalidInput)
        );
    }

    #[test]
    fn probe_values_expose_probe_node_output() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(0.25));
        let probe = patch.insert(Module::Probe);
        patch.connect(source, probe).unwrap();
        patch.output(probe).unwrap();

        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
        compiled.next();
        let values = compiled.probe_values().collect::<Vec<_>>();

        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0, probe);
        assert!((values[0].1.value() - 0.25).abs() < 0.001);
    }

    #[test]
    fn input_values_expose_connected_module_inputs() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(0.25));
        let lowpass = patch.insert(Module::Lowpass {
            cutoff: Hertz::new(1000.0).unwrap(),
        });
        patch.connect(source, lowpass).unwrap();
        patch.output(lowpass).unwrap();

        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
        compiled.next();
        let mut values = Vec::new();
        compiled.visit_input_values(|module, input, value| {
            values.push((module, input, value.value()));
        });

        assert!(values.contains(&(lowpass, 0, 0.25)));
    }

    #[test]
    fn engine_rejects_overlapping_updates() {
        let rate = SampleRate::new(44_100).unwrap();
        let mut engine = PatchEngine::new(CompiledPatch::new(&patch(440.0), rate).unwrap());

        assert!(
            engine
                .replace(CompiledPatch::new(&patch(880.0), rate).unwrap())
                .is_ok()
        );
        let pending = CompiledPatch::new(&patch(220.0), rate).unwrap();
        let Err(UpdateRejected::Busy(pending)) = engine.replace(pending) else {
            panic!("update should be rejected while fading");
        };

        for _ in 0..UPDATE_FADE_FRAMES {
            engine.next();
        }

        let Err(UpdateRejected::RetiredPatchPending(pending)) = engine.replace(pending) else {
            panic!("update should be rejected until retired patch is taken");
        };
        assert!(engine.take_retired().is_some());
        assert!(engine.replace(pending).is_ok());
    }
}
