use crate::compile::{CompiledPatch, PatchControls, PatchEngine, UpdateRejected};
use crate::sample::Frame;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};

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
    pending: AtomicPtr<CompiledPatch>,
    retired: AtomicPtr<CompiledPatch>,
}

#[derive(Debug)]
pub struct RealtimePatchEngine {
    engine: PatchEngine,
    exchange: Arc<PatchExchange>,
}

impl PatchExchange {
    pub fn new() -> Self {
        Self {
            pending: AtomicPtr::new(ptr::null_mut()),
            retired: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn submit(&self, patch: Box<CompiledPatch>) -> Option<Box<CompiledPatch>> {
        ptr_to_box(self.pending.swap(Box::into_raw(patch), Ordering::AcqRel))
    }

    pub fn take_retired(&self) -> Option<Box<CompiledPatch>> {
        ptr_to_box(self.retired.swap(ptr::null_mut(), Ordering::AcqRel))
    }

    fn take_pending(&self) -> Option<Box<CompiledPatch>> {
        ptr_to_box(self.pending.swap(ptr::null_mut(), Ordering::AcqRel))
    }

    fn retired_is_empty(&self) -> bool {
        self.retired.load(Ordering::Acquire).is_null()
    }

    fn retire(&self, patch: Box<CompiledPatch>) {
        debug_assert!(self.retired_is_empty());
        self.retired.store(Box::into_raw(patch), Ordering::Release);
    }
}

impl Default for PatchExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PatchExchange {
    fn drop(&mut self) {
        drop(ptr_to_box(*self.pending.get_mut()));
        drop(ptr_to_box(*self.retired.get_mut()));
    }
}

impl RealtimePatchEngine {
    pub fn new(active: Box<CompiledPatch>, exchange: Arc<PatchExchange>) -> Self {
        Self {
            engine: PatchEngine::new_boxed(active),
            exchange,
        }
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
        if self.exchange.retired_is_empty()
            && let Some(retired) = self.engine.take_retired_boxed()
        {
            self.exchange.retire(retired);
        }
    }

    fn accept_pending(&mut self) {
        if !self.engine.can_replace() {
            return;
        }
        let Some(next) = self.exchange.take_pending() else {
            return;
        };
        match self.engine.replace_boxed(next) {
            Ok(()) => {}
            Err(UpdateRejected::Busy(next) | UpdateRejected::RetiredPatchPending(next)) => {
                let ptr = Box::into_raw(next);
                if self
                    .exchange
                    .pending
                    .compare_exchange(ptr::null_mut(), ptr, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    let _ = ptr;
                }
            }
        }
    }
}

fn ptr_to_box(ptr: *mut CompiledPatch) -> Option<Box<CompiledPatch>> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { Box::from_raw(ptr) })
    }
}
