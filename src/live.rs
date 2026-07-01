use crate::compile::{CompiledPatch, PatchControls, PatchEngine, UpdateRejected};
use crate::sample::Frame;
use rtrb::{Consumer, Producer, RingBuffer};

#[cfg(feature = "live")]
use cpal::traits::{DeviceTrait, HostTrait};
#[cfg(feature = "live")]
use cpal::{Device, StreamConfig};

#[cfg(feature = "live")]
pub struct AudioPlayer {
    pub device: Device,
    pub config: StreamConfig,
}

pub trait AudioSink {
    type Error;

    fn write(&mut self, frame: Frame) -> Result<(), Self::Error>;
}

pub fn run<S: AudioSink>(
    engine: &mut PatchEngine,
    sink: &mut S,
    frames: usize,
) -> Result<(), S::Error> {
    for _ in 0..frames {
        sink.write(engine.next())?;
    }
    Ok(())
}

#[cfg(feature = "live")]
impl AudioPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;
        let config = device.default_output_config()?.into();
        Ok(Self { device, config })
    }
}

#[derive(Debug)]
pub struct PatchExchange {
    pending: Producer<CompiledPatch>,
    retired: Consumer<CompiledPatch>,
}

#[derive(Debug)]
pub struct RealtimePatchEngine {
    engine: PatchEngine,
    pending: Consumer<CompiledPatch>,
    retired: Producer<CompiledPatch>,
    pending_retry: Option<CompiledPatch>,
    retired_retry: Option<CompiledPatch>,
}

impl PatchExchange {
    pub fn submit(&mut self, patch: CompiledPatch) -> Option<CompiledPatch> {
        match self.pending.push(patch) {
            Ok(()) => None,
            Err(error) => match error {
                rtrb::PushError::Full(patch) => Some(patch),
            },
        }
    }

    pub fn take_retired(&mut self) -> Option<CompiledPatch> {
        self.retired.pop().ok()
    }
}

impl RealtimePatchEngine {
    pub fn new(active: CompiledPatch) -> (Self, PatchExchange) {
        let (pending_producer, pending_consumer) = RingBuffer::new(2);
        let (retired_producer, retired_consumer) = RingBuffer::new(2);
        (
            Self {
                engine: PatchEngine::new(active),
                pending: pending_consumer,
                retired: retired_producer,
                pending_retry: None,
                retired_retry: None,
            },
            PatchExchange {
                pending: pending_producer,
                retired: retired_consumer,
            },
        )
    }

    pub fn next(&mut self) -> Frame {
        self.next_with_controls(PatchControls::default())
    }

    pub fn next_with_controls(&mut self, controls: PatchControls) -> Frame {
        self.return_retired();
        self.accept_pending();
        self.engine.next_with_controls(controls)
    }

    fn return_retired(&mut self) {
        if let Some(retired) = self.retired_retry.take()
            && let Err(error) = self.retired.push(retired)
        {
            let rtrb::PushError::Full(retired) = error;
            self.retired_retry = Some(retired);
            return;
        }
        if let Some(retired) = self.engine.take_retired()
            && let Err(error) = self.retired.push(retired)
        {
            let rtrb::PushError::Full(retired) = error;
            self.retired_retry = Some(retired);
        }
    }

    fn accept_pending(&mut self) {
        if self.retired_retry.is_some() || !self.engine.can_replace() {
            return;
        }
        let next = if let Some(next) = self.pending_retry.take() {
            next
        } else {
            let Ok(next) = self.pending.pop() else {
                return;
            };
            next
        };
        match self.engine.replace(next) {
            Ok(()) => {}
            Err(UpdateRejected::Busy(next) | UpdateRejected::RetiredPatchPending(next)) => {
                self.pending_retry = Some(next);
            }
        }
    }
}
