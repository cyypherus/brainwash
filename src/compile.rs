use crate::delay::Delay;
use crate::effect::Distortion;
use crate::osc::Oscillator;
use crate::patch::{BinaryOp, EnvPoint, Module, Patch};
use crate::sample::{Frame, Sample, Unit};
use crate::time::{Duration, Hertz, SampleRate};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledPatch {
    nodes: Vec<Node>,
    inputs: Vec<Vec<Option<usize>>>,
    input_values: Vec<Vec<Option<Sample>>>,
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
    Input {
        index: usize,
        default: Sample,
    },
    Silence,
    Freq,
    Gate,
    Degree,
    DegreeGate {
        target: i32,
    },
    Constant(Sample),
    Absolute,
    Pass,
    Osc {
        osc: Oscillator,
        frequency: Hertz,
    },
    Rise(GateRamp),
    Fall(GateRamp),
    Ramp(Ramp),
    Envelope {
        points: Arc<Vec<EnvPoint>>,
    },
    Lowpass(OnePole),
    Highpass(OnePole),
    Comb(Comb),
    Allpass(Allpass),
    Delay(Delay),
    VariableDelay(VariableDelay),
    Waveshaper(Distortion),
    Slew(Slew),
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
        samples: Arc<Vec<Sample>>,
    },
    Probe,
    Composition(Box<CompiledPatch>),
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct Slew {
    rate: SampleRate,
    rise: crate::time::Seconds,
    fall: crate::time::Seconds,
    value: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct VariableDelay {
    buffer: Vec<f32>,
    index: usize,
    rate: SampleRate,
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
            input_values: vec![Vec::new()],
            values: vec![Sample::ZERO],
            output: 0,
            probes: Vec::new(),
            meters: Vec::new(),
        }
    }

    pub fn new(patch: &Patch, rate: SampleRate) -> Result<Self, CompileError> {
        Self::compile(patch, rate, None)
    }

    fn compile(
        patch: &Patch,
        rate: SampleRate,
        composition: Option<&crate::patch::Composition>,
    ) -> Result<Self, CompileError> {
        let order = compile_order(patch)?;
        let mut rank_by_module = Vec::with_capacity(order.len());
        for (rank, module_index) in order.iter().copied().enumerate() {
            rank_by_module.push((patch.modules()[module_index].0, rank));
        }

        let mut inputs = Vec::with_capacity(order.len());
        for module_index in order.iter().copied() {
            inputs.push(vec![None; patch.modules()[module_index].1.input_count()]);
        }

        for connection in patch.connections() {
            let source = rank_by_module
                .iter()
                .find_map(|(id, rank)| (*id == connection.from).then_some(*rank))
                .ok_or(CompileError::MissingModule)?;
            let target = rank_by_module
                .iter()
                .find_map(|(id, rank)| (*id == connection.input.module).then_some(*rank))
                .ok_or(CompileError::MissingModule)?;
            let module = order
                .get(target)
                .and_then(|module_index| patch.modules().get(*module_index))
                .map(|(_, module)| module)
                .ok_or(CompileError::MissingModule)?;
            let input = module
                .input_index(connection.input.input)
                .ok_or(CompileError::InvalidInput)?;
            let slot = inputs
                .get_mut(target)
                .and_then(|inputs| inputs.get_mut(input))
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
                (module.input_count() > 0).then_some((*id, rank))
            })
            .collect();

        let mut nodes = Vec::with_capacity(order.len());
        for module_index in order.iter().copied() {
            nodes.push(create_node(
                &patch.modules()[module_index].1,
                rate,
                composition,
                patch.modules()[module_index].0,
            )?);
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
            input_values: inputs
                .iter()
                .map(|inputs| vec![None; inputs.len()])
                .collect(),
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
        self.next_with_inputs(controls, &[])
    }

    fn next_with_inputs(&mut self, controls: PatchControls, external: &[Option<Sample>]) -> Frame {
        for idx in 0..self.nodes.len() {
            for (value, source) in self.input_values[idx]
                .iter_mut()
                .zip(self.inputs[idx].iter().copied())
            {
                *value = source.and_then(|source| self.values.get(source).copied());
            }
            self.values[idx] = self.nodes[idx].process(&self.input_values[idx], external, controls);
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
    fn process(
        &mut self,
        inputs: &[Option<Sample>],
        external: &[Option<Sample>],
        controls: PatchControls,
    ) -> Sample {
        let in0 = sample(inputs, 0);
        match self {
            Node::Input { index, default } => {
                external.get(*index).copied().flatten().unwrap_or(*default)
            }
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
            Node::Absolute => Sample::raw(in0.value().abs()),
            Node::Pass => Sample::raw(inputs.iter().flatten().map(|input| input.value()).sum()),
            Node::Osc { osc, frequency } => {
                let frequency = inputs
                    .first()
                    .and_then(|input| input.and_then(|input| Hertz::new(input.value())))
                    .unwrap_or(*frequency);
                if let Some(frequency) = Hertz::new(frequency.value()) {
                    osc.set_frequency(frequency);
                }
                osc.next()
            }
            Node::Rise(ramp) | Node::Fall(ramp) => Sample::raw(ramp.next(in0.value())),
            Node::Ramp(ramp) => Sample::raw(ramp.next(in0.value())),
            Node::Envelope { points } => Sample::raw(envelope_value(points, in0.value())),
            Node::Lowpass(filter) => {
                if let Some(cutoff) = inputs
                    .get(1)
                    .and_then(|input| input.and_then(|input| filter_cutoff(input.value())))
                {
                    filter.cutoff = cutoff;
                }
                filter.lowpass(in0)
            }
            Node::Highpass(filter) => {
                if let Some(cutoff) = inputs
                    .get(1)
                    .and_then(|input| input.and_then(|input| filter_cutoff(input.value())))
                {
                    filter.cutoff = cutoff;
                }
                filter.highpass(in0)
            }
            Node::Comb(comb) => Sample::raw(comb.next(in0.value())),
            Node::Allpass(allpass) => Sample::raw(allpass.next(in0.value())),
            Node::Delay(delay) => delay.process(in0),
            Node::VariableDelay(delay) => Sample::raw(delay.next(
                in0.value(),
                inputs.get(1).copied().flatten().map_or(0.0, Sample::value),
                inputs.get(2).copied().flatten().map_or(0.0, Sample::value),
            )),
            Node::Waveshaper(kind) => kind.process(in0),
            Node::Slew(slew) => {
                if let Some(rise) = inputs.get(1).copied().flatten() {
                    slew.rise = crate::time::Seconds::new(rise.value())
                        .unwrap_or_else(|| crate::time::Seconds::new(0.0).unwrap());
                }
                if let Some(fall) = inputs.get(2).copied().flatten() {
                    slew.fall = crate::time::Seconds::new(fall.value())
                        .unwrap_or_else(|| crate::time::Seconds::new(0.0).unwrap());
                }
                Sample::raw(slew.next(in0.value()))
            }
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
                    BinaryOp::Subtract => a - b,
                    BinaryOp::Divide => {
                        if b == 0.0 {
                            0.0
                        } else {
                            a / b
                        }
                    }
                    BinaryOp::Power => a.max(0.0).powf(b),
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
            Node::Composition(composition) => composition.next_with_inputs(controls, inputs).left(),
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

impl Slew {
    fn next(&mut self, input: f32) -> f32 {
        let duration = if input > self.value {
            self.rise
        } else {
            self.fall
        };
        let samples = Duration::Seconds(duration)
            .samples(self.rate)
            .value()
            .max(1) as f32;
        let coefficient = (-1.0 / samples).exp();
        self.value = coefficient * self.value + (1.0 - coefficient) * input;
        self.value
    }
}

impl VariableDelay {
    fn new(rate: SampleRate, max_time: Duration) -> Self {
        Self {
            buffer: vec![0.0; max_time.samples(rate).value().max(2) as usize],
            index: 0,
            rate,
        }
    }

    fn next(&mut self, input: f32, seconds: f32, feedback: f32) -> f32 {
        let delay = (seconds.max(0.0) * self.rate.value() as f32)
            .round()
            .clamp(1.0, (self.buffer.len() - 1) as f32) as usize;
        let read = (self.index + self.buffer.len() - delay) % self.buffer.len();
        let delayed = self.buffer[read];
        self.buffer[self.index] = input + delayed * feedback.clamp(-0.999, 0.999);
        self.index = (self.index + 1) % self.buffer.len();
        delayed
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
        let target = module_index(patch, connection.input.module)?;
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

fn create_node(
    module: &Module,
    rate: SampleRate,
    composition: Option<&crate::patch::Composition>,
    module_id: crate::patch::ModuleId,
) -> Result<Node, CompileError> {
    Ok(match module {
        Module::Input { default, .. } => Node::Input {
            index: composition
                .and_then(|composition| composition.input_index(module_id))
                .ok_or(CompileError::InvalidInput)?,
            default: *default,
        },
        Module::Freq => Node::Freq,
        Module::Gate => Node::Gate,
        Module::Degree => Node::Degree,
        Module::DegreeGate { target } => Node::DegreeGate { target: *target },
        Module::Constant(value) => Node::Constant(*value),
        Module::Absolute => Node::Absolute,
        Module::Pass => Node::Pass,
        Module::Osc { wave, frequency } => Node::Osc {
            osc: Oscillator::new(*wave, rate, *frequency),
            frequency: *frequency,
        },
        Module::Rise { time } => Node::Rise(GateRamp::new(GateRampMode::Rise, *time, rate)),
        Module::Fall { time } => Node::Fall(GateRamp::new(GateRampMode::Fall, *time, rate)),
        Module::Ramp { value, time } => Node::Ramp(Ramp::new(value.value(), *time, rate)),
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
        Module::VariableDelay { max_time } => {
            Node::VariableDelay(VariableDelay::new(rate, *max_time))
        }
        Module::Waveshaper(kind) => Node::Waveshaper(*kind),
        Module::Slew { rise, fall } => Node::Slew(Slew {
            rate,
            rise: *rise,
            fall: *fall,
            value: 0.0,
        }),
        Module::Binary { op, a, b } => Node::Binary {
            op: *op,
            a: a.value(),
            b: b.value(),
        },
        Module::Switch { a, b } => Node::Switch {
            a: a.value(),
            b: b.value(),
        },
        Module::Random => Node::Random {
            last_gate: 0.0,
            value: 0.0,
            seed: 0x1234_5678,
        },
        Module::Sample { samples } => Node::Sample {
            samples: Arc::clone(samples),
        },
        Module::Probe => Node::Probe,
        Module::Composition(composition) => Node::Composition(Box::new(CompiledPatch::compile(
            composition.patch(),
            rate,
            Some(composition),
        )?)),
    })
}

fn sample(inputs: &[Option<Sample>], index: usize) -> Sample {
    inputs.get(index).copied().flatten().unwrap_or(Sample::ZERO)
}

fn filter_cutoff(value: f32) -> Option<Hertz> {
    Hertz::new(20.0 * 1000.0_f32.powf(value.clamp(0.0, 1.0)))
}

fn envelope_value(points: &[EnvPoint], time: f32) -> f32 {
    let time = time.clamp(0.0, 1.0);
    if points.is_empty() {
        return 0.0;
    }
    if points.len() == 1 || time <= points[0].time.value() {
        return points[0].value.value();
    }
    let last = points.len() - 1;
    if time >= points[last].time.value() {
        return points[last].value.value();
    }
    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        if time < start.time.value() || time > end.time.value() {
            continue;
        }
        let span = (end.time.value() - start.time.value()).max(f32::EPSILON);
        let mut amount = (time - start.time.value()) / span;
        amount = match (start.curve, end.curve) {
            (false, false) => amount,
            (true, false) => 1.0 - (1.0 - amount) * (1.0 - amount),
            (false, true) => amount * amount,
            (true, true) if amount < 0.5 => 2.0 * amount * amount,
            (true, true) => 1.0 - 2.0 * (1.0 - amount) * (1.0 - amount),
        };
        return start.value.value() + (end.value.value() - start.value.value()) * amount;
    }
    points[last].value.value()
}

fn sample_value(samples: &[Sample], position: f32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let position = position.clamp(0.0, 1.0);
    let index = position * (samples.len() - 1) as f32;
    let base = index.floor() as usize;
    let amount = index - base as f32;
    let a = samples[base].value();
    let b = samples
        .get(base + 1)
        .copied()
        .unwrap_or(samples[base])
        .value();
    a + (b - a) * amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::osc::Wave;
    use crate::patch::InputKind;
    use crate::patch::{Module, Patch};
    use crate::time::Hertz;

    fn patch(rate: f32) -> Patch {
        let mut patch = Patch::new();
        let osc = patch.insert(Module::Osc {
            wave: Wave::Sine,
            frequency: Hertz::new(rate).unwrap(),
        });
        patch.output(osc).unwrap();
        patch
    }

    #[test]
    fn compile_requires_output() {
        let mut patch = Patch::new();
        patch.insert(Module::Osc {
            wave: Wave::Sine,
            frequency: Hertz::new(440.0).unwrap(),
        });
        let rate = SampleRate::new(44_100).unwrap();

        assert_eq!(
            CompiledPatch::new(&patch, rate),
            Err(CompileError::MissingOutput)
        );
    }

    #[test]
    fn invalid_input_slot_is_rejected_before_compile() {
        let mut patch = Patch::new();
        patch.insert(Module::Freq);
        let gate = patch.insert(Module::Gate);
        assert_eq!(
            patch.input_port(gate, InputKind::Freq),
            Err(crate::patch::ConnectError::ClosedInput)
        );
        patch.output(gate).unwrap();
        let rate = SampleRate::new(44_100).unwrap();

        assert!(CompiledPatch::new(&patch, rate).is_ok());
    }

    #[test]
    fn probe_values_expose_probe_node_output() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(Sample::new(0.25).unwrap()));
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
        let source = patch.insert(Module::Constant(Sample::new(0.25).unwrap()));
        let lowpass = patch.insert(Module::Lowpass {
            cutoff: Hertz::new(1000.0).unwrap(),
        });
        let port = patch.input_port(lowpass, InputKind::In).unwrap();
        patch.connect_input(source, port).unwrap();
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
    fn transpose_converts_semitones_to_a_frequency_ratio() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(Sample::new(0.25).unwrap()));
        let transpose = patch.insert(crate::preset::transpose(Sample::new(12.0).unwrap()));
        let input = patch.input_port(transpose, InputKind::In).unwrap();
        patch.connect_input(source, input).unwrap();
        patch.output(transpose).unwrap();

        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
        assert!((compiled.next().left().value() - 0.5).abs() < 0.001);
    }

    #[test]
    fn input_values_include_high_numbered_semantic_ports() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(Sample::new(0.5).unwrap()));
        let compressor = patch.insert(crate::preset::compressor(
            Unit::new(0.5).unwrap(),
            crate::patch::CompressorRatio::new(2.0).unwrap(),
            crate::time::Seconds::new(0.001).unwrap(),
            crate::time::Seconds::new(0.001).unwrap(),
            crate::patch::Gain::new(1.0).unwrap(),
        ));
        let port = patch.input_port(compressor, InputKind::Gain).unwrap();
        patch.connect_input(source, port).unwrap();
        patch.output(compressor).unwrap();

        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();
        compiled.next();
        let mut values = Vec::new();
        compiled.visit_input_values(|module, input, value| {
            values.push((module, input, value.value()));
        });

        assert!(values.contains(&(compressor, 5, 0.5)));
    }

    #[test]
    fn compositions_are_not_limited_to_six_inputs() {
        let mut inner = Patch::new();
        let inputs = [
            InputKind::In,
            InputKind::Freq,
            InputKind::Gain,
            InputKind::Attack,
            InputKind::Release,
            InputKind::Feedback,
            InputKind::Q,
        ]
        .map(|kind| {
            (
                format!("{kind:?}"),
                kind,
                inner.insert(Module::Input {
                    kind,
                    default: Sample::ZERO,
                }),
            )
        });
        inner.output(inputs[6].2).unwrap();
        let composition = Module::Composition(Box::new(
            crate::patch::Composition::new("Wide", inner, inputs).unwrap(),
        ));
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(Sample::new(0.75).unwrap()));
        let target = patch.insert(composition);
        let port = patch.input_port(target, InputKind::Q).unwrap();
        patch.connect_input(source, port).unwrap();
        patch.output(target).unwrap();
        let mut compiled = CompiledPatch::new(&patch, SampleRate::new(44_100).unwrap()).unwrap();

        assert_eq!(compiled.next().left(), Sample::new(0.75).unwrap());
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
