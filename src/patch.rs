use crate::sample::{Sample, Unit};
use crate::time::{Duration, Hertz};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub(crate) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    Saw,
    Noise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distortion {
    Clip,
    Tanh,
    Fold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Multiply,
    Add,
    GreaterThan,
    LessThan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputKind {
    In,
    Gate,
    Phase,
    Freq,
    Shift,
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
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Drive(f32);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Gain(f32);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct CompressorRatio(f32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdsrShape {
    pub(crate) attack: Duration,
    pub(crate) decay: Duration,
    pub(crate) sustain: Unit,
    pub(crate) release: Duration,
}

impl Drive {
    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && value >= 0.0).then_some(Self(value))
    }

    pub(crate) fn value(self) -> f32 {
        self.0
    }
}

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

impl AdsrShape {
    pub fn new(attack: Duration, decay: Duration, sustain: Unit, release: Duration) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Module {
    Freq,
    Gate,
    Degree,
    DegreeGate {
        target: i32,
    },
    Constant(Sample),
    Pass,
    Osc {
        wave: Wave,
        frequency: Hertz,
        shift: Sample,
        gain: Unit,
        unipolar: bool,
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
    Adsr {
        attack_ratio: Unit,
        sustain: Unit,
    },
    Envelope {
        points: Arc<Vec<EnvPoint>>,
    },
    Lowpass {
        cutoff: Hertz,
    },
    Highpass {
        cutoff: Hertz,
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
    DelayTap {
        gain: Unit,
    },
    Reverb {
        room: Unit,
        damp: Unit,
        mod_depth: Unit,
        diffusion: Unit,
    },
    Distortion {
        kind: Distortion,
        drive: Drive,
    },
    Compressor {
        threshold: Unit,
        ratio: CompressorRatio,
        attack: Duration,
        release: Duration,
        makeup: Gain,
    },
    Flanger {
        rate: Hertz,
        depth: Unit,
        feedback: Unit,
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnvPoint {
    pub time: Unit,
    pub value: Unit,
    pub curve: bool,
}

impl EnvPoint {
    pub fn new(time: Unit, value: Unit, curve: bool) -> Self {
        Self { time, value, curve }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    modules: Vec<(ModuleId, Module)>,
    connections: Vec<Connection>,
    output: Option<ModuleId>,
    next_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectError {
    ClosedInput,
    InputOccupied,
    MissingModule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputPort {
    pub(crate) module: ModuleId,
    pub(crate) input: InputKind,
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

    pub fn connect(&mut self, from: ModuleId, to: ModuleId) -> Result<(), ConnectError> {
        self.module(from).ok_or(ConnectError::MissingModule)?;
        let module = self.module(to).ok_or(ConnectError::MissingModule)?;
        if module.input_count() != 1 {
            return Err(ConnectError::ClosedInput);
        }
        self.connect_input(
            from,
            InputPort {
                module: to,
                input: module.inputs()[0],
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

    pub fn connect_input(&mut self, from: ModuleId, input: InputPort) -> Result<(), ConnectError> {
        let to = input.module;
        self.module(from).ok_or(ConnectError::MissingModule)?;
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

    pub fn output(&mut self, module: ModuleId) -> Result<(), ConnectError> {
        self.module(module).ok_or(ConnectError::MissingModule)?;
        self.output = Some(module);
        Ok(())
    }

    fn module(&self, id: ModuleId) -> Option<&Module> {
        self.modules
            .iter()
            .find_map(|(module_id, module)| (*module_id == id).then_some(module))
    }

    pub(crate) fn modules(&self) -> &[(ModuleId, Module)] {
        &self.modules
    }

    pub(crate) fn connections(&self) -> &[Connection] {
        &self.connections
    }

    pub(crate) fn output_id(&self) -> Option<ModuleId> {
        self.output
    }
}

impl Module {
    pub fn inputs(&self) -> &'static [InputKind] {
        match self {
            Module::Freq
            | Module::Gate
            | Module::Degree
            | Module::DegreeGate { .. }
            | Module::Constant(_) => &[],
            Module::Random => &[InputKind::Gate],
            Module::Pass | Module::DelayTap { .. } | Module::Probe => &[InputKind::In],
            Module::Rise { .. } | Module::Fall { .. } => &[InputKind::Gate, InputKind::Time],
            Module::Ramp { .. } => &[InputKind::Value, InputKind::Time],
            Module::Adsr { .. } => &[
                InputKind::Rise,
                InputKind::Fall,
                InputKind::Attack,
                InputKind::Sustain,
            ],
            Module::Envelope { .. } => &[InputKind::Phase],
            Module::Lowpass { .. } | Module::Highpass { .. } => {
                &[InputKind::In, InputKind::Freq, InputKind::Q]
            }
            Module::Comb { .. } => &[
                InputKind::In,
                InputKind::Time,
                InputKind::Feedback,
                InputKind::Damp,
            ],
            Module::Allpass { .. } => &[InputKind::In, InputKind::Time, InputKind::Feedback],
            Module::Delay { .. } => &[InputKind::In, InputKind::Time],
            Module::Reverb { .. } => &[
                InputKind::In,
                InputKind::Room,
                InputKind::Damp,
                InputKind::Mod,
                InputKind::Diff,
            ],
            Module::Distortion { .. } => &[InputKind::In, InputKind::Drive, InputKind::Asym],
            Module::Compressor { .. } => &[
                InputKind::In,
                InputKind::Thresh,
                InputKind::Ratio,
                InputKind::Attack,
                InputKind::Release,
                InputKind::Gain,
            ],
            Module::Flanger { .. } => &[
                InputKind::In,
                InputKind::Rate,
                InputKind::Depth,
                InputKind::Feedback,
            ],
            Module::Sample { .. } => &[InputKind::Position],
            Module::Binary { .. } => &[InputKind::A, InputKind::B],
            Module::Osc { .. } => &[InputKind::Freq, InputKind::Shift, InputKind::Gain],
            Module::Switch { .. } => &[InputKind::Select, InputKind::A, InputKind::B],
        }
    }

    pub(crate) fn input_count(&self) -> usize {
        self.inputs().len()
    }

    pub(crate) fn input_index(&self, input: InputKind) -> Option<usize> {
        self.inputs()
            .iter()
            .position(|candidate| *candidate == input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Connection {
    pub(crate) from: ModuleId,
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
            patch.connect(source, target),
            Err(ConnectError::ClosedInput)
        );
        assert_eq!(patch.connections.len(), 0);
    }

    #[test]
    fn connect_rejects_multi_input_targets() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Gate);
        let target = patch.insert(Module::Osc {
            wave: Wave::Sine,
            frequency: Hertz::new(440.0).unwrap(),
            shift: Sample::ZERO,
            gain: Unit::ONE,
            unipolar: false,
        });

        assert_eq!(
            patch.connect(source, target),
            Err(ConnectError::ClosedInput)
        );
        assert_eq!(patch.connections.len(), 0);
    }

    #[test]
    fn checked_input_port_stores_target_slot() {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Gate);
        let target = patch.insert(Module::Osc {
            wave: Wave::Sine,
            frequency: Hertz::new(440.0).unwrap(),
            shift: Sample::ZERO,
            gain: Unit::ONE,
            unipolar: false,
        });
        let port = patch.input_port(target, InputKind::Gain).unwrap();

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
                cutoff: Hertz::new(1000.0).unwrap()
            }
            .inputs(),
            &[InputKind::In, InputKind::Freq, InputKind::Q]
        );
        assert_eq!(
            Module::Binary {
                op: BinaryOp::Add,
                a: Sample::ZERO,
                b: Sample::ZERO
            }
            .inputs(),
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

        let point = EnvPoint::new(unit(0.25), unit(0.75), true);

        assert_eq!(point.time.value(), 0.25);
        assert_eq!(point.value.value(), 0.75);
    }
}
