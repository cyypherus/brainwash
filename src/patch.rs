use crate::sample::{Sample, Unit};
use crate::time::{Duration, Hertz, Seconds};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub(crate) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Multiply,
    Add,
    Subtract,
    Divide,
    Power,
    Remainder,
    Minimum,
    Maximum,
    GreaterThan,
    LessThan,
    Equal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Absolute,
    Sine,
    HyperbolicTangent,
    Arctangent,
    Exponential,
    Sign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Saw,
    ReverseSaw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaturationCurve {
    Tube,
    Tape,
    Fuzz,
    Fold,
    Clip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputKind {
    In,
    Gate,
    Phase,
    Freq,
    Semitones,
    Gain,
    Rise,
    Fall,
    Value,
    Time,
    Feedback,
    Damp,
    Room,
    Mod,
    Diff,
    Drive,
    Asym,
    Thresh,
    Ratio,
    Attack,
    Release,
    Sustain,
    Rate,
    Depth,
    A,
    B,
    Select,
    Position,
    Q,
    Channel1,
    Channel2,
    Channel3,
    Channel4,
    Channel5,
    Channel6,
    Channel7,
    Channel8,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Gain(f32);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct CompressorRatio(f32);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Resonance(f32);

impl Gain {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value >= 0.0).then_some(Self(value))
    }

    pub(crate) fn value(self) -> f32 {
        self.0
    }
}

impl CompressorRatio {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value >= 1.0).then_some(Self(value))
    }

    pub(crate) fn value(self) -> f32 {
        self.0
    }
}

impl Resonance {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && (0.1..=20.0).contains(&value)).then_some(Self(value))
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Module {
    Input {
        kind: InputKind,
        default: Sample,
    },
    Freq,
    Gate,
    Degree,
    Expression,
    Constant(Sample),
    Unary {
        op: UnaryOp,
        input: Sample,
    },
    Damp {
        input: Sample,
        coefficient: Unit,
    },
    Phase {
        frequency: Hertz,
    },
    Oscillator {
        waveform: Waveform,
        frequency: Hertz,
    },
    Saturation {
        curve: SaturationCurve,
        input: Sample,
        drive: Sample,
        asymmetry: Sample,
    },
    Noise,
    Rise {
        gate: Sample,
        time: Duration,
    },
    Fall {
        gate: Sample,
        time: Duration,
    },
    Ramp {
        value: Sample,
        time: Duration,
    },
    Envelope {
        phase: Sample,
        points: Arc<Vec<EnvPoint>>,
    },
    Filter {
        input: Sample,
        cutoff: Hertz,
        resonance: Resonance,
    },
    Comb {
        input: Sample,
        time: Duration,
        feedback: Unit,
        damp: Unit,
    },
    Allpass {
        input: Sample,
        time: Duration,
        feedback: Unit,
    },
    Delay {
        input: Sample,
        time: Duration,
        feedback: Unit,
    },
    DelayTap(DelayTap),
    Slew {
        input: Sample,
        rise: Seconds,
        fall: Seconds,
    },
    Binary {
        op: BinaryOp,
        a: Sample,
        b: Sample,
    },
    Switch {
        select: Sample,
        a: Sample,
        b: Sample,
    },
    Random {
        gate: Sample,
    },
    Sample {
        position: Sample,
        samples: Arc<Vec<Sample>>,
    },
    Probe {
        input: Sample,
    },
    Composition(Box<Composition>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnvPoint {
    pub time: Unit,
    pub value: Sample,
    pub curve: bool,
}

impl EnvPoint {
    pub fn new(time: Unit, value: Sample, curve: bool) -> Self {
        Self { time, value, curve }
    }
}

pub fn envelope_value(points: &[EnvPoint], time: f32) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    let time = time.clamp(0.0, 1.0);
    if points.len() == 1 || time <= points[0].time.value() {
        return points[0].value.value();
    }
    let last = points.len() - 1;
    if time >= points[last].time.value() {
        return points[last].value.value();
    }

    let mut start = 0;
    while start < last {
        let mut end = start + 1;
        while end < last && points[end].curve {
            end += 1;
        }
        if time <= points[end].time.value() {
            if end == start + 1 {
                let span =
                    (points[end].time.value() - points[start].time.value()).max(f32::EPSILON);
                let amount = (time - points[start].time.value()) / span;
                return points[start].value.value()
                    + (points[end].value.value() - points[start].value.value()) * amount;
            }

            let mut low = 0.0;
            let mut high = 1.0;
            for _ in 0..20 {
                let parameter = (low + high) * 0.5;
                if bezier_coordinate(points, start, end, parameter, true) < time {
                    low = parameter;
                } else {
                    high = parameter;
                }
            }
            return bezier_coordinate(points, start, end, (low + high) * 0.5, false);
        }
        start = end;
    }
    points[last].value.value()
}

fn bezier_coordinate(
    points: &[EnvPoint],
    start: usize,
    end: usize,
    parameter: f32,
    time: bool,
) -> f32 {
    if parameter <= 0.0 {
        return if time {
            points[start].time.value()
        } else {
            points[start].value.value()
        };
    }
    if parameter >= 1.0 {
        return if time {
            points[end].time.value()
        } else {
            points[end].value.value()
        };
    }
    let degree = end - start;
    let inverse = 1.0 - parameter;
    let mut basis = inverse.powi(degree as i32);
    let mut result = 0.0;
    for index in 0..=degree {
        let point = points[start + index];
        result += basis
            * if time {
                point.time.value()
            } else {
                point.value.value()
            };
        if index < degree {
            basis *= (degree - index) as f32 / (index + 1) as f32 * parameter / inverse;
        }
    }
    result
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    modules: Vec<(ModuleId, Module)>,
    connections: Vec<Connection>,
    output: Option<OutputPort>,
    next_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectError {
    ClosedInput,
    InputOccupied,
    MissingModule,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    name: String,
    patch: Patch,
    inputs: Vec<CompositionInput>,
    outputs: Vec<CompositionOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionInput {
    label: String,
    kind: InputKind,
    module: ModuleId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionOutput {
    label: String,
    port: OutputPort,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayTap {
    delay: ModuleId,
    gain: Unit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionError {
    Cycle,
    DuplicateLabel,
    DuplicateInput,
    EmptyLabel,
    EmptyName,
    InputKindMismatch,
    MissingInput,
    MissingOutput,
    DuplicateOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputPort {
    pub(crate) module: ModuleId,
    pub(crate) input: InputKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OutputPort {
    pub(crate) module: ModuleId,
    pub(crate) output: u16,
}

impl Patch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, module: Module) -> ModuleId {
        let id = ModuleId(self.next_id);
        self.next_id += 1;
        self.modules.push((id, module));
        id
    }

    pub fn insert_delay_tap(
        &mut self,
        delay: ModuleId,
        gain: Unit,
    ) -> Result<ModuleId, ConnectError> {
        if !matches!(self.module(delay), Some(Module::Delay { .. })) {
            return Err(ConnectError::MissingModule);
        }
        Ok(self.insert(Module::DelayTap(DelayTap { delay, gain })))
    }

    pub fn connect(&mut self, from: OutputPort, to: ModuleId) -> Result<(), ConnectError> {
        let module = self.module(to).ok_or(ConnectError::MissingModule)?;
        if module.input_count() != 1 {
            return Err(ConnectError::ClosedInput);
        }
        self.connect_input(
            from,
            InputPort {
                module: to,
                input: module.input_kinds()[0],
            },
        )
    }

    pub fn input_port(
        &self,
        module: ModuleId,
        input: InputKind,
    ) -> Result<InputPort, ConnectError> {
        let target = self.module(module).ok_or(ConnectError::MissingModule)?;
        if target.input_index(input).is_none() {
            return Err(ConnectError::ClosedInput);
        }
        Ok(InputPort { module, input })
    }

    pub fn connect_input(
        &mut self,
        from: OutputPort,
        input: InputPort,
    ) -> Result<(), ConnectError> {
        let to = input.module;
        self.output_port(from.module, from.output)?;
        let target = self.module(to).ok_or(ConnectError::MissingModule)?;
        if target.input_index(input.input).is_none() {
            return Err(ConnectError::ClosedInput);
        }
        if self
            .connections
            .iter()
            .any(|connection| connection.input == input)
        {
            return Err(ConnectError::InputOccupied);
        }
        self.connections.push(Connection { from, input });
        Ok(())
    }

    pub fn output_port(&self, module: ModuleId, output: u16) -> Result<OutputPort, ConnectError> {
        let source = self.module(module).ok_or(ConnectError::MissingModule)?;
        if output >= source.output_count() {
            return Err(ConnectError::ClosedInput);
        }
        Ok(OutputPort { module, output })
    }

    pub fn output(&mut self, output: OutputPort) -> Result<(), ConnectError> {
        self.output_port(output.module, output.output)?;
        self.output = Some(output);
        Ok(())
    }

    pub fn module_entries(&self) -> impl Iterator<Item = (ModuleId, &Module)> {
        self.modules.iter().map(|(id, module)| (*id, module))
    }

    pub fn connection_entries(&self) -> impl Iterator<Item = (OutputPort, InputPort)> + '_ {
        self.connections
            .iter()
            .map(|connection| (connection.from, connection.input))
    }

    pub fn output_module(&self) -> Option<OutputPort> {
        self.output
    }

    fn module(&self, id: ModuleId) -> Option<&Module> {
        self.modules
            .iter()
            .find_map(|(module_id, module)| (*module_id == id).then_some(module))
    }

    pub(crate) fn module_by_id(&self, id: ModuleId) -> Option<&Module> {
        self.module(id)
    }

    pub(crate) fn modules(&self) -> &[(ModuleId, Module)] {
        &self.modules
    }

    pub(crate) fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub(crate) fn output_id(&self) -> Option<OutputPort> {
        self.output
    }
}

impl ModuleId {
    pub fn value(self) -> u32 {
        self.0
    }
}

impl InputPort {
    pub fn module(self) -> ModuleId {
        self.module
    }

    pub fn kind(self) -> InputKind {
        self.input
    }
}

impl OutputPort {
    pub fn module(self) -> ModuleId {
        self.module
    }

    pub fn index(self) -> u16 {
        self.output
    }
}

impl Composition {
    pub fn new(
        name: impl Into<String>,
        patch: Patch,
        inputs: impl IntoIterator<Item = (String, InputKind, ModuleId)>,
        outputs: impl IntoIterator<Item = (String, OutputPort)>,
    ) -> Result<Self, CompositionError> {
        if patch.output.is_none() {
            return Err(CompositionError::MissingOutput);
        }
        let name = name.into();
        if name.trim().is_empty() {
            return Err(CompositionError::EmptyName);
        }
        let inputs = inputs
            .into_iter()
            .map(|(label, kind, module)| CompositionInput {
                label,
                kind,
                module,
            })
            .collect::<Vec<_>>();
        let outputs = outputs
            .into_iter()
            .map(|(label, port)| CompositionOutput { label, port })
            .collect::<Vec<_>>();
        if outputs.is_empty() {
            return Err(CompositionError::MissingOutput);
        }
        for (index, output) in outputs.iter().enumerate() {
            if output.label.trim().is_empty() {
                return Err(CompositionError::EmptyLabel);
            }
            if outputs[..index]
                .iter()
                .any(|candidate| candidate.label == output.label)
            {
                return Err(CompositionError::DuplicateLabel);
            }
            if outputs[..index]
                .iter()
                .any(|candidate| candidate.port == output.port)
            {
                return Err(CompositionError::DuplicateOutput);
            }
            patch
                .output_port(output.port.module, output.port.output)
                .map_err(|_| CompositionError::MissingOutput)?;
        }
        for (index, input) in inputs.iter().enumerate() {
            if input.label.trim().is_empty() {
                return Err(CompositionError::EmptyLabel);
            }
            if inputs[..index]
                .iter()
                .any(|candidate| candidate.label == input.label)
            {
                return Err(CompositionError::DuplicateLabel);
            }
            if inputs[..index]
                .iter()
                .any(|candidate| candidate.kind == input.kind)
            {
                return Err(CompositionError::DuplicateInput);
            }
            match patch.module(input.module) {
                Some(Module::Input {
                    kind: candidate, ..
                }) if *candidate == input.kind => {}
                Some(Module::Input { .. }) => return Err(CompositionError::InputKindMismatch),
                Some(_) => return Err(CompositionError::InputKindMismatch),
                None => return Err(CompositionError::MissingInput),
            }
        }
        for (module, definition) in &patch.modules {
            if matches!(definition, Module::Input { .. })
                && !inputs.iter().any(|input| input.module == *module)
            {
                return Err(CompositionError::MissingInput);
            }
        }
        crate::compile::check_composition_cycles(&patch).map_err(|error| match error {
            crate::compile::CompileError::Cycle => CompositionError::Cycle,
            _ => CompositionError::MissingInput,
        })?;
        Ok(Self {
            name,
            patch,
            inputs,
            outputs,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn patch(&self) -> &Patch {
        &self.patch
    }

    pub(crate) fn input_index(&self, module: ModuleId) -> Option<usize> {
        self.inputs.iter().position(|input| input.module == module)
    }

    pub fn inputs(&self) -> &[CompositionInput] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[CompositionOutput] {
        &self.outputs
    }
}

impl CompositionOutput {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn port(&self) -> OutputPort {
        self.port
    }
}

impl CompositionInput {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn kind(&self) -> InputKind {
        self.kind
    }

    pub fn module(&self) -> ModuleId {
        self.module
    }
}

impl DelayTap {
    pub fn delay(self) -> ModuleId {
        self.delay
    }

    pub fn gain(self) -> Unit {
        self.gain
    }
}

impl Module {
    pub fn output_count(&self) -> u16 {
        match self {
            Module::Filter { .. } => 2,
            Module::Composition(composition) => composition.outputs.len() as u16,
            _ => 1,
        }
    }

    pub fn input_kinds(&self) -> Vec<InputKind> {
        match self {
            Module::Input { .. } => vec![],
            Module::Freq
            | Module::Gate
            | Module::Degree
            | Module::Expression
            | Module::Constant(_) => vec![],
            Module::Unary { .. } => vec![InputKind::In],
            Module::Damp { .. } => vec![InputKind::In, InputKind::Damp],
            Module::Random { .. } => vec![InputKind::Gate],
            Module::Probe { .. } => vec![InputKind::In],
            Module::Rise { .. } | Module::Fall { .. } => vec![InputKind::Gate, InputKind::Time],
            Module::Ramp { .. } => vec![InputKind::Value, InputKind::Time],
            Module::Envelope { .. } => vec![InputKind::Phase],
            Module::Filter { .. } => {
                vec![InputKind::In, InputKind::Freq, InputKind::Q]
            }
            Module::Comb { .. } => vec![
                InputKind::In,
                InputKind::Time,
                InputKind::Feedback,
                InputKind::Damp,
            ],
            Module::Allpass { .. } => vec![InputKind::In, InputKind::Time, InputKind::Feedback],
            Module::Delay { .. } => vec![InputKind::Feedback, InputKind::Time, InputKind::In],
            Module::DelayTap(_) => vec![],
            Module::Slew { .. } => vec![InputKind::In, InputKind::Rise, InputKind::Fall],
            Module::Sample { .. } => vec![InputKind::Position],
            Module::Binary { .. } => vec![InputKind::A, InputKind::B],
            Module::Phase { .. } | Module::Oscillator { .. } => vec![InputKind::Freq],
            Module::Saturation { .. } => vec![InputKind::In, InputKind::Drive, InputKind::Asym],
            Module::Noise => vec![],
            Module::Switch { .. } => vec![InputKind::Select, InputKind::A, InputKind::B],
            Module::Composition(composition) => {
                composition.inputs.iter().map(|input| input.kind).collect()
            }
        }
    }

    pub(crate) fn input_count(&self) -> usize {
        self.input_kinds().len()
    }

    pub(crate) fn input_index(&self, input: InputKind) -> Option<usize> {
        self.input_kinds()
            .iter()
            .position(|candidate| *candidate == input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Connection {
    pub(crate) from: OutputPort,
    pub(crate) input: InputPort,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(value: f32) -> Unit {
        Unit::new(value).unwrap()
    }

    #[test]
    fn invalid_input_port_is_not_stored() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Gate);
        let target = patch.insert(Module::Gate);

        assert_eq!(
            patch.input_port(target, InputKind::Freq),
            Err(ConnectError::ClosedInput)
        );
        assert_eq!(patch.connections.len(), 0);
        assert_eq!(
            patch.connect(patch.output_port(source, 0).unwrap(), target),
            Err(ConnectError::ClosedInput)
        );
        assert_eq!(patch.connections.len(), 0);
    }

    #[test]
    fn curve_point_pulls_the_chord_between_its_neighbors() {
        let points = [
            EnvPoint::new(unit(0.0), Sample::raw(0.0), false),
            EnvPoint::new(unit(0.5), Sample::raw(1.0), true),
            EnvPoint::new(unit(1.0), Sample::raw(0.0), false),
        ];

        assert!((envelope_value(&points, 0.5) - 0.5).abs() < 0.0001);
        assert!(envelope_value(&points, 0.25) > 0.0);
        assert!(envelope_value(&points, 0.75) > 0.0);
    }

    #[test]
    fn ordinary_point_remains_a_linear_waypoint() {
        let points = [
            EnvPoint::new(unit(0.0), Sample::raw(0.0), false),
            EnvPoint::new(unit(0.5), Sample::raw(1.0), false),
            EnvPoint::new(unit(1.0), Sample::raw(0.0), false),
        ];

        assert!((envelope_value(&points, 0.5) - 1.0).abs() < 0.0001);
        assert!((envelope_value(&points, 0.25) - 0.5).abs() < 0.0001);
    }

    #[test]
    fn connect_rejects_multi_input_targets() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Gate);
        let target = patch.insert(crate::preset::transpose(Sample::ZERO));

        assert_eq!(
            patch.connect(patch.output_port(source, 0).unwrap(), target),
            Err(ConnectError::ClosedInput)
        );
        assert_eq!(patch.connections.len(), 0);
    }

    #[test]
    fn checked_input_port_stores_target_slot() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Gate);
        let target = patch.insert(crate::preset::sine(Hertz::new(440.0).unwrap()));
        let port = patch.input_port(target, InputKind::Freq).unwrap();

        let source = patch.output_port(source, 0).unwrap();
        patch.connect_input(source, port).unwrap();

        assert_eq!(
            patch.connections,
            vec![Connection {
                from: source,
                input: InputPort {
                    module: target,
                    input: InputKind::Freq
                }
            }]
        );
    }

    #[test]
    fn module_inputs_are_semantic_shape() {
        let filter = Module::Filter {
            input: Sample::ZERO,
            cutoff: Hertz::new(1000.0).unwrap(),
            resonance: Resonance::new(0.707).unwrap(),
        };
        assert_eq!(filter.output_count(), 2);
        assert_eq!(
            filter.input_kinds(),
            &[InputKind::In, InputKind::Freq, InputKind::Q]
        );
        assert_eq!(
            Module::Binary {
                op: BinaryOp::Add,
                a: Sample::ZERO,
                b: Sample::ZERO
            }
            .input_kinds(),
            &[InputKind::A, InputKind::B]
        );
    }

    #[test]
    fn filter_exposes_low_and_high_outputs() {
        let mut patch = Patch::new();
        let filter = patch.insert(Module::Filter {
            input: Sample::ZERO,
            cutoff: Hertz::new(1000.0).unwrap(),
            resonance: Resonance::new(0.707).unwrap(),
        });

        assert!(patch.output_port(filter, 0).is_ok());
        assert!(patch.output_port(filter, 1).is_ok());
        assert_eq!(patch.output_port(filter, 2), Err(ConnectError::ClosedInput));
    }

    #[test]
    fn invalid_scalar_constructors_reject_non_dsp_values() {
        assert!(Sample::new(f32::NAN).is_none());
        assert!(Gain::new(f32::NAN).is_none());
        assert!(Gain::new(-0.1).is_none());
        assert!(CompressorRatio::new(f32::NAN).is_none());
        assert!(CompressorRatio::new(0.5).is_none());
        assert!(Unit::new(1.1).is_none());

        let point = EnvPoint::new(unit(0.25), Sample::new(-0.75).unwrap(), true);

        assert_eq!(point.time.value(), 0.25);
        assert_eq!(point.value.value(), -0.75);
    }
}
