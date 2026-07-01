use crate::sample::Unit;
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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Drive(f32);

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
    Constant(f32),
    Pass,
    Osc {
        wave: Wave,
        frequency: Hertz,
        shift: f32,
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
        value: f32,
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
        ratio: f32,
        attack: Duration,
        release: Duration,
        makeup: f32,
    },
    Flanger {
        rate: Hertz,
        depth: Unit,
        feedback: Unit,
    },
    Binary {
        op: BinaryOp,
        a: f32,
        b: f32,
    },
    Switch {
        a: f32,
        b: f32,
    },
    Random,
    Sample {
        samples: Arc<Vec<f32>>,
    },
    Probe,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnvPoint {
    pub time: f32,
    pub value: f32,
    pub curve: bool,
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
        self.connect_input(from, to, 0)
    }

    pub fn connect_input(
        &mut self,
        from: ModuleId,
        to: ModuleId,
        input: usize,
    ) -> Result<(), ConnectError> {
        self.module(from).ok_or(ConnectError::MissingModule)?;
        self.module(to).ok_or(ConnectError::MissingModule)?;
        if self
            .connections
            .iter()
            .any(|connection| connection.to == to && connection.input == input)
        {
            return Err(ConnectError::InputOccupied);
        }
        self.connections.push(Connection { from, to, input });
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Connection {
    pub(crate) from: ModuleId,
    pub(crate) to: ModuleId,
    pub(crate) input: usize,
}
