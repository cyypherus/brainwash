use crate::osc::Wave;
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
    DegreeGate {
        target: i32,
    },
    Constant(Sample),
    Unary(UnaryOp),
    Pass,
    Damp {
        coefficient: Unit,
    },
    Osc {
        wave: Wave,
        frequency: Hertz,
    },
    Rise {
        time: Duration,
    },
    Fall {
        time: Duration,
    },
    Ramp {
        value: Sample,
        time: Duration,
    },
    Envelope {
        points: Arc<Vec<EnvPoint>>,
    },
    Lowpass {
        cutoff: Hertz,
        resonance: Resonance,
    },
    Highpass {
        cutoff: Hertz,
        resonance: Resonance,
    },
    Comb {
        time: Duration,
        feedback: Unit,
        damp: Unit,
    },
    Allpass {
        time: Duration,
        feedback: Unit,
    },
    Delay {
        time: Duration,
        feedback: Unit,
    },
    DelayTap(DelayTap),
    VariableDelay {
        max_time: Duration,
    },
    Slew {
        rise: Seconds,
        fall: Seconds,
    },
    Binary {
        op: BinaryOp,
        a: Sample,
        b: Sample,
    },
    Switch {
        a: Sample,
        b: Sample,
    },
    Random,
    Sample {
        samples: Arc<Vec<Sample>>,
    },
    Probe,
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
        let mut indegree = vec![0usize; patch.modules.len()];
        let mut outgoing = vec![Vec::new(); patch.modules.len()];
        for connection in &patch.connections {
            let source = patch
                .modules
                .iter()
                .position(|(id, _)| *id == connection.from.module)
                .ok_or(CompositionError::MissingInput)?;
            let target = patch
                .modules
                .iter()
                .position(|(id, _)| *id == connection.input.module)
                .ok_or(CompositionError::MissingInput)?;
            outgoing[source].push(target);
            indegree[target] += 1;
        }
        let mut ready = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect::<Vec<_>>();
        let mut visited = 0;
        while let Some(source) = ready.pop() {
            visited += 1;
            for target in &outgoing[source] {
                indegree[*target] -= 1;
                if indegree[*target] == 0 {
                    ready.push(*target);
                }
            }
        }
        if visited != patch.modules.len() {
            return Err(CompositionError::Cycle);
        }
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
            | Module::DegreeGate { .. }
            | Module::Constant(_) => vec![],
            Module::Unary(_) => vec![InputKind::In],
            Module::Damp { .. } => vec![InputKind::In, InputKind::Damp],
            Module::Random => vec![InputKind::Gate],
            Module::Pass | Module::Probe => vec![InputKind::In],
            Module::Rise { .. } | Module::Fall { .. } => vec![InputKind::Gate, InputKind::Time],
            Module::Ramp { .. } => vec![InputKind::Value, InputKind::Time],
            Module::Envelope { .. } => vec![InputKind::Phase],
            Module::Lowpass { .. } | Module::Highpass { .. } => {
                vec![InputKind::In, InputKind::Freq, InputKind::Q]
            }
            Module::Comb { .. } => vec![
                InputKind::In,
                InputKind::Time,
                InputKind::Feedback,
                InputKind::Damp,
            ],
            Module::Allpass { .. } => vec![InputKind::In, InputKind::Time, InputKind::Feedback],
            Module::Delay { .. } => vec![InputKind::Time, InputKind::In],
            Module::DelayTap(_) => vec![],
            Module::VariableDelay { .. } => {
                vec![InputKind::In, InputKind::Time, InputKind::Feedback]
            }
            Module::Slew { .. } => vec![InputKind::In, InputKind::Rise, InputKind::Fall],
            Module::Sample { .. } => vec![InputKind::Position],
            Module::Binary { .. } => vec![InputKind::A, InputKind::B],
            Module::Osc { .. } => vec![InputKind::Freq],
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
    fn connect_rejects_multi_input_targets() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Gate);
        let target = patch.insert(crate::preset::oscillator(
            Wave::Sine,
            Hertz::new(440.0).unwrap(),
            Unit::ONE,
            false,
        ));

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
        let target = patch.insert(crate::preset::oscillator(
            Wave::Sine,
            Hertz::new(440.0).unwrap(),
            Unit::ONE,
            false,
        ));
        let port = patch.input_port(target, InputKind::Gain).unwrap();

        let source = patch.output_port(source, 0).unwrap();
        patch.connect_input(source, port).unwrap();

        assert_eq!(
            patch.connections,
            vec![Connection {
                from: source,
                input: InputPort {
                    module: target,
                    input: InputKind::Gain
                }
            }]
        );
    }

    #[test]
    fn module_inputs_are_semantic_shape() {
        assert_eq!(
            Module::Lowpass {
                cutoff: Hertz::new(1000.0).unwrap(),
                resonance: Resonance::new(0.707).unwrap(),
            }
            .input_kinds(),
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
