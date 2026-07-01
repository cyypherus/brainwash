use brainwash::compile::{CompiledPatch, PatchControls};
use brainwash::compile::{PatchEngine, UpdateRejected};
use brainwash::patch::ModuleId as AudioModuleId;
use brainwash::sample::Frame;
use brainwash::time::{Hertz, SampleRate};
use brainwash::track::{NoteEvent, Track};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

pub struct AudioRuntime {
    handle: AudioHandle,
    _stream: cpal::Stream,
}

#[derive(Clone, Debug)]
pub struct AudioHandle {
    exchange: Arc<PatchBankExchange>,
    track_exchange: Arc<TrackExchange>,
    probe_bus: Arc<ProbeBus>,
    playing: Arc<AtomicBool>,
    rate: SampleRate,
}

pub(crate) struct ProbeRoute {
    pub(crate) source: AudioModuleId,
    pub(crate) target: u32,
}

#[derive(Debug)]
pub enum AudioStartError {
    NoOutputDevice,
    DefaultConfig(cpal::DefaultStreamConfigError),
    InvalidSampleRate,
    UnsupportedFormat(SampleFormat),
    BuildStream(cpal::BuildStreamError),
    PlayStream(cpal::PlayStreamError),
}

const PLAY_FADE_FRAMES: f32 = 256.0;
const VOICES: usize = 6;
const PROBE_SLOTS: usize = 8;
const PROBE_HISTORY: usize = 88_200;

impl AudioRuntime {
    pub fn start() -> Result<Self, AudioStartError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioStartError::NoOutputDevice)?;
        let supported = device
            .default_output_config()
            .map_err(AudioStartError::DefaultConfig)?;
        let rate = SampleRate::new(supported.sample_rate().0)
            .ok_or(AudioStartError::InvalidSampleRate)?;
        let config = supported.config();
        let exchange = Arc::new(PatchBankExchange::new());
        let track_exchange = Arc::new(TrackExchange::new());
        let probe_bus = Arc::new(ProbeBus::new());
        let playing = Arc::new(AtomicBool::new(false));
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config,
                &exchange,
                &track_exchange,
                &probe_bus,
                &playing,
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config,
                &exchange,
                &track_exchange,
                &probe_bus,
                &playing,
            )?,
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config,
                &exchange,
                &track_exchange,
                &probe_bus,
                &playing,
            )?,
            format => return Err(AudioStartError::UnsupportedFormat(format)),
        };
        stream.play().map_err(AudioStartError::PlayStream)?;
        Ok(Self {
            handle: AudioHandle {
                exchange,
                track_exchange,
                probe_bus,
                playing,
                rate,
            },
            _stream: stream,
        })
    }

    pub fn handle(&self) -> AudioHandle {
        self.handle.clone()
    }
}

impl AudioHandle {
    pub fn sample_rate(&self) -> SampleRate {
        self.rate
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Release);
    }

    pub fn submit(&self, patch: CompiledPatch) {
        drop(self.exchange.submit(Box::new(PatchBank::new(patch))));
        self.collect_retired();
    }

    pub(crate) fn submit_with_probes(&self, patch: CompiledPatch, probes: &[ProbeRoute]) {
        let slots = self.probe_bus.set_routes(probes);
        drop(
            self.exchange
                .submit(Box::new(PatchBank::new_with_probes(patch, slots))),
        );
        self.collect_retired();
    }

    pub(crate) fn probe_history(&self, module: u32, voice: usize, len: usize) -> Vec<f32> {
        self.probe_bus.history(module, voice, len)
    }

    pub fn submit_track(&self, track: Track, bpm: u16) {
        drop(self.track_exchange.submit(Box::new(TrackRuntime::new(
            track, bpm, self.rate,
        ))));
        self.collect_retired();
    }

    pub fn collect_retired(&self) {
        while self.exchange.take_retired().is_some() {}
        while self.track_exchange.take_retired().is_some() {}
    }
}

impl PartialEq for AudioHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.exchange, &other.exchange)
            && Arc::ptr_eq(&self.track_exchange, &other.track_exchange)
            && Arc::ptr_eq(&self.probe_bus, &other.probe_bus)
            && Arc::ptr_eq(&self.playing, &other.playing)
            && self.rate == other.rate
    }
}

impl Eq for AudioHandle {}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    exchange: &Arc<PatchBankExchange>,
    track_exchange: &Arc<TrackExchange>,
    probe_bus: &Arc<ProbeBus>,
    playing: &Arc<AtomicBool>,
) -> Result<cpal::Stream, AudioStartError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let callback_exchange = Arc::clone(exchange);
    let callback_track_exchange = Arc::clone(track_exchange);
    let callback_probe_bus = Arc::clone(probe_bus);
    let callback_playing = Arc::clone(playing);
    let mut engine = VoicePatchRuntime::new(
        PatchBank::new(CompiledPatch::silence()),
        callback_exchange,
        callback_probe_bus,
    );
    let mut track = None;
    let mut gain = 0.0;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                write_output(
                    data,
                    channels,
                    &callback_playing,
                    &mut engine,
                    &callback_track_exchange,
                    &mut track,
                    &mut gain,
                );
            },
            |_| {},
            None,
        )
        .map_err(AudioStartError::BuildStream)
}

fn write_output<T>(
    data: &mut [T],
    channels: usize,
    playing: &AtomicBool,
    engine: &mut VoicePatchRuntime,
    track_exchange: &TrackExchange,
    track: &mut Option<Box<TrackRuntime>>,
    gain: &mut f32,
) where
    T: Sample + FromSample<f32>,
{
    accept_track(track_exchange, track);
    let is_playing = playing.load(Ordering::Acquire);
    let target = if is_playing {
        1.0
    } else {
        0.0
    };
    let step = 1.0 / PLAY_FADE_FRAMES;
    for output in data.chunks_mut(channels) {
        if *gain < target {
            *gain = (*gain + step).min(1.0);
        } else if *gain > target {
            *gain = (*gain - step).max(0.0);
        }
        let controls = track
            .as_mut()
            .map(|track| track.next(is_playing))
            .unwrap_or_default();
        write_frame(output, engine.next_with_controls(controls), *gain);
    }
}

fn write_frame<T>(output: &mut [T], frame: Frame, gain: f32)
where
    T: Sample + FromSample<f32>,
{
    let left = frame.left().clipped().value() * gain;
    let right = frame.right().clipped().value() * gain;
    for (index, sample) in output.iter_mut().enumerate() {
        let value = if index % 2 == 0 { left } else { right };
        *sample = T::from_sample(value);
    }
}

#[derive(Debug)]
struct ProbeBus {
    ids: [AtomicU32; PROBE_SLOTS],
    values: Box<[AtomicU32]>,
    cursor: AtomicUsize,
}

impl ProbeBus {
    fn new() -> Self {
        let count = VOICES * PROBE_SLOTS * PROBE_HISTORY;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(AtomicU32::new(0.0f32.to_bits()));
        }
        Self {
            ids: std::array::from_fn(|_| AtomicU32::new(0)),
            values: values.into_boxed_slice(),
            cursor: AtomicUsize::new(0),
        }
    }

    fn set_routes(&self, routes: &[ProbeRoute]) -> Box<[(AudioModuleId, usize)]> {
        for id in &self.ids {
            id.store(0, Ordering::Release);
        }
        let mut slots = Vec::new();
        for (slot, route) in routes.iter().take(PROBE_SLOTS).enumerate() {
            self.clear_slot(slot);
            self.ids[slot].store(route.target.saturating_add(1), Ordering::Release);
            slots.push((route.source, slot));
        }
        slots.into_boxed_slice()
    }

    fn clear_slot(&self, slot: usize) {
        for voice in 0..VOICES {
            for cursor in 0..PROBE_HISTORY {
                self.values[probe_value_index(voice, slot, cursor)]
                    .store(0.0f32.to_bits(), Ordering::Release);
            }
        }
    }

    fn write(&self, slot: usize, voice: usize, cursor: usize, value: f32) {
        self.values[probe_value_index(voice, slot, cursor)].store(value.to_bits(), Ordering::Release);
    }

    fn advance(&self, cursor: usize) {
        self.cursor
            .store((cursor + 1) % PROBE_HISTORY, Ordering::Release);
    }

    fn history(&self, module: u32, voice: usize, len: usize) -> Vec<f32> {
        let slot_id = module.saturating_add(1);
        let Some(slot) = self
            .ids
            .iter()
            .position(|id| id.load(Ordering::Acquire) == slot_id)
        else {
            return Vec::new();
        };
        let voice = voice.min(VOICES - 1);
        let count = len.min(PROBE_HISTORY);
        let cursor = self.cursor.load(Ordering::Acquire);
        let start = (cursor + PROBE_HISTORY - count) % PROBE_HISTORY;
        let mut values = Vec::with_capacity(count);
        for offset in 0..count {
            let index = (start + offset) % PROBE_HISTORY;
            values.push(f32::from_bits(
                self.values[probe_value_index(voice, slot, index)].load(Ordering::Acquire),
            ));
        }
        values
    }
}

fn probe_value_index(voice: usize, slot: usize, cursor: usize) -> usize {
    ((voice * PROBE_SLOTS) + slot) * PROBE_HISTORY + cursor
}

#[derive(Debug)]
struct PatchBankExchange {
    pending: std::sync::atomic::AtomicPtr<PatchBank>,
    retired: std::sync::atomic::AtomicPtr<PatchBank>,
}

struct PatchBank {
    voices: [Box<CompiledPatch>; VOICES],
    probes: Box<[(AudioModuleId, usize)]>,
}

struct VoicePatchRuntime {
    voices: [PatchEngine; VOICES],
    exchange: Arc<PatchBankExchange>,
    probe_bus: Arc<ProbeBus>,
    probes: Box<[(AudioModuleId, usize)]>,
    probe_cursor: usize,
}

impl PatchBankExchange {
    fn new() -> Self {
        Self {
            pending: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
            retired: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    fn submit(&self, bank: Box<PatchBank>) -> Option<Box<PatchBank>> {
        patch_bank_ptr_to_box(self.pending.swap(Box::into_raw(bank), Ordering::AcqRel))
    }

    fn take_retired(&self) -> Option<Box<PatchBank>> {
        patch_bank_ptr_to_box(self.retired.swap(std::ptr::null_mut(), Ordering::AcqRel))
    }

    fn take_pending(&self) -> Option<Box<PatchBank>> {
        patch_bank_ptr_to_box(self.pending.swap(std::ptr::null_mut(), Ordering::AcqRel))
    }

    fn retired_is_empty(&self) -> bool {
        self.retired.load(Ordering::Acquire).is_null()
    }

    fn retire(&self, bank: Box<PatchBank>) {
        self.retired.store(Box::into_raw(bank), Ordering::Release);
    }
}

impl Drop for PatchBankExchange {
    fn drop(&mut self) {
        drop(patch_bank_ptr_to_box(*self.pending.get_mut()));
        drop(patch_bank_ptr_to_box(*self.retired.get_mut()));
    }
}

impl PatchBank {
    fn new(patch: CompiledPatch) -> Self {
        Self::new_with_probes(patch, Vec::new().into_boxed_slice())
    }

    fn new_with_probes(patch: CompiledPatch, probes: Box<[(AudioModuleId, usize)]>) -> Self {
        Self {
            voices: std::array::from_fn(|_| Box::new(patch.clone())),
            probes,
        }
    }
}

impl VoicePatchRuntime {
    fn new(bank: PatchBank, exchange: Arc<PatchBankExchange>, probe_bus: Arc<ProbeBus>) -> Self {
        Self {
            voices: bank.voices.map(PatchEngine::new_boxed),
            exchange,
            probe_bus,
            probes: bank.probes,
            probe_cursor: 0,
        }
    }

    fn next_with_controls(&mut self, controls: [PatchControls; VOICES]) -> Frame {
        self.return_retired();
        self.accept_pending();
        let mut left = 0.0;
        let mut right = 0.0;
        for (voice, (engine, controls)) in self.voices.iter_mut().zip(controls).enumerate() {
            let frame = engine.next_with_controls(controls);
            record_probe_values(&self.probe_bus, &self.probes, self.probe_cursor, voice, engine);
            left += frame.left().value();
            right += frame.right().value();
        }
        self.probe_bus.advance(self.probe_cursor);
        self.probe_cursor = (self.probe_cursor + 1) % PROBE_HISTORY;
        Frame::stereo(
            brainwash::sample::Sample::new(left).unwrap_or_default(),
            brainwash::sample::Sample::new(right).unwrap_or_default(),
        )
    }

    fn return_retired(&mut self) {
        if !self.exchange.retired_is_empty()
            || self
                .voices
                .iter()
                .any(|engine| !engine.retired_pending())
        {
            return;
        }
        let retired = std::array::from_fn(|index| {
            self.voices[index]
                .take_retired_boxed()
                .unwrap_or_else(|| Box::new(CompiledPatch::silence()))
        });
        self.exchange.retire(Box::new(PatchBank {
            voices: retired,
            probes: Vec::new().into_boxed_slice(),
        }));
    }

    fn accept_pending(&mut self) {
        if self.voices.iter().any(|engine| !engine.can_replace()) {
            return;
        }
        let Some(bank) = self.exchange.take_pending() else {
            return;
        };
        self.probes = bank.probes;
        for (engine, next) in self.voices.iter_mut().zip(bank.voices) {
            if let Err(UpdateRejected::Busy(_) | UpdateRejected::RetiredPatchPending(_)) =
                engine.replace_boxed(next)
            {
                return;
            }
        }
    }
}

fn record_probe_values(
    bus: &ProbeBus,
    probes: &[(AudioModuleId, usize)],
    cursor: usize,
    voice: usize,
    engine: &PatchEngine,
) {
    for (id, value) in engine.probe_values() {
        if let Some((_, slot)) = probes.iter().find(|(probe, _)| *probe == id) {
            bus.write(*slot, voice, cursor, value.value());
        }
    }
}

fn patch_bank_ptr_to_box(ptr: *mut PatchBank) -> Option<Box<PatchBank>> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { Box::from_raw(ptr) })
    }
}

#[derive(Debug)]
struct TrackExchange {
    pending: std::sync::atomic::AtomicPtr<TrackRuntime>,
    retired: std::sync::atomic::AtomicPtr<TrackRuntime>,
}

struct TrackRuntime {
    track: Track,
    bpm: f32,
    bars: f32,
    rate: SampleRate,
    phase: f32,
    voices: [Voice; VOICES],
    age: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    pitch: u8,
    frequency: f32,
    gate: f32,
    degree: i32,
    age: usize,
}

impl TrackExchange {
    fn new() -> Self {
        Self {
            pending: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
            retired: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    fn submit(&self, track: Box<TrackRuntime>) -> Option<Box<TrackRuntime>> {
        track_ptr_to_box(self.pending.swap(Box::into_raw(track), Ordering::AcqRel))
    }

    fn take_retired(&self) -> Option<Box<TrackRuntime>> {
        track_ptr_to_box(self.retired.swap(std::ptr::null_mut(), Ordering::AcqRel))
    }

    fn take_pending(&self) -> Option<Box<TrackRuntime>> {
        track_ptr_to_box(self.pending.swap(std::ptr::null_mut(), Ordering::AcqRel))
    }

    fn retire(&self, track: Box<TrackRuntime>) {
        self.retired.store(Box::into_raw(track), Ordering::Release);
    }
}

impl Drop for TrackExchange {
    fn drop(&mut self) {
        drop(track_ptr_to_box(*self.pending.get_mut()));
        drop(track_ptr_to_box(*self.retired.get_mut()));
    }
}

impl TrackRuntime {
    fn new(track: Track, bpm: u16, rate: SampleRate) -> Self {
        let bars = track.bar_count().max(1) as f32;
        Self {
            track,
            bpm: bpm.max(1) as f32,
            bars,
            rate,
            phase: 0.0,
            voices: [Voice::default(); VOICES],
            age: 0,
        }
    }

    fn next(&mut self, playing: bool) -> [PatchControls; VOICES] {
        if !playing {
            return [PatchControls::default(); VOICES];
        }
        self.advance();
        self.voices.map(|voice| PatchControls {
            frequency: Hertz::new(voice.frequency),
            gate: voice.gate,
            degree: voice.degree,
        })
    }

    fn advance(&mut self) {
        let bars_per_second = self.bpm / 60.0 / 4.0 / self.bars;
        self.phase = (self.phase + bars_per_second / self.rate.value() as f32).fract();
        let mut events = [None; 64];
        let count = self.track.play_into(self.phase, &mut events);
        for event in events.into_iter().take(count).flatten() {
            self.apply_event(event);
        }
    }

    fn apply_event(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::Press { pitch, degree } => {
                let index = self
                    .voices
                    .iter()
                    .position(|voice| voice.pitch == pitch && voice.gate > 0.5)
                    .or_else(|| {
                        self.voices
                            .iter()
                            .position(|voice| voice.pitch == pitch && voice.gate < 0.5)
                    })
                    .or_else(|| {
                        self.voices
                            .iter()
                            .enumerate()
                            .filter(|(_, voice)| voice.gate < 0.5)
                            .min_by_key(|(_, voice)| voice.age)
                            .map(|(index, _)| index)
                    })
                    .unwrap_or(0);
                self.age += 1;
                self.voices[index] = Voice {
                    pitch,
                    frequency: 440.0 * 2.0f32.powf((pitch as f32 - 69.0) / 12.0),
                    gate: 1.0,
                    degree,
                    age: self.age,
                };
            }
            NoteEvent::Release { pitch } => {
                if let Some(voice) = self
                    .voices
                    .iter_mut()
                    .find(|voice| voice.pitch == pitch && voice.gate > 0.5)
                {
                    voice.gate = 0.0;
                }
            }
        }
    }
}

fn accept_track(exchange: &TrackExchange, track: &mut Option<Box<TrackRuntime>>) {
    if let Some(next) = exchange.take_pending() {
        if let Some(old) = track.replace(next) {
            exchange.retire(old);
        }
    }
}

fn track_ptr_to_box(ptr: *mut TrackRuntime) -> Option<Box<TrackRuntime>> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { Box::from_raw(ptr) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwash::patch::{Module, Patch, Wave};
    use brainwash::sample::Unit;
    use brainwash::scale::cmin;

    #[test]
    fn track_runtime_exposes_polyphonic_voice_controls() {
        let track = Track::parse("{0&2}", &cmin()).unwrap();
        let mut runtime = TrackRuntime::new(track, 120, SampleRate::new(44_100).unwrap());

        let controls = runtime.next(true);
        let active = controls.iter().filter(|control| control.gate > 0.5).count();

        assert_eq!(active, 2);
    }

    #[test]
    fn voice_patch_runtime_sums_independent_voices() {
        let rate = SampleRate::new(44_100).unwrap();
        let mut patch = Patch::new();
        let freq = patch.insert(Module::Freq);
        let osc = patch.insert(Module::Osc {
            wave: Wave::Saw,
            frequency: Hertz::new(110.0).unwrap(),
            shift: 0.0,
            gain: Unit::ONE,
            unipolar: false,
        });
        patch.connect(freq, osc).unwrap();
        patch.output(osc).unwrap();
        let compiled = CompiledPatch::new(&patch, rate).unwrap();
        let exchange = Arc::new(PatchBankExchange::new());
        let mut runtime =
            VoicePatchRuntime::new(PatchBank::new(compiled), exchange, Arc::new(ProbeBus::new()));
        let mut controls = [PatchControls::default(); VOICES];
        controls[0] = PatchControls {
            frequency: Hertz::new(220.0),
            gate: 1.0,
            degree: 0,
        };
        controls[1] = PatchControls {
            frequency: Hertz::new(330.0),
            gate: 1.0,
            degree: 2,
        };

        let mut audible = false;
        for _ in 0..64 {
            let frame = runtime.next_with_controls(controls);
            audible |= frame.left().value().abs() > 0.001 || frame.right().value().abs() > 0.001;
        }

        assert!(audible);
    }

    #[test]
    fn voice_patch_runtime_returns_retired_bank_after_crossfade() {
        let rate = SampleRate::new(44_100).unwrap();
        let exchange = Arc::new(PatchBankExchange::new());
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled_saw_patch(110.0, rate)),
            Arc::clone(&exchange),
            Arc::new(ProbeBus::new()),
        );
        assert!(exchange
            .submit(Box::new(PatchBank::new(compiled_saw_patch(220.0, rate))))
            .is_none());
        let controls = [PatchControls {
            frequency: Hertz::new(330.0),
            gate: 1.0,
            degree: 0,
        }; VOICES];

        runtime.next_with_controls(controls);
        assert!(exchange.take_retired().is_none());

        for _ in 0..127 {
            runtime.next_with_controls(controls);
        }
        runtime.next_with_controls(controls);

        assert!(exchange.take_retired().is_some());
    }

    fn compiled_saw_patch(frequency: f32, rate: SampleRate) -> CompiledPatch {
        let mut patch = Patch::new();
        let freq = patch.insert(Module::Freq);
        let osc = patch.insert(Module::Osc {
            wave: Wave::Saw,
            frequency: Hertz::new(frequency).unwrap(),
            shift: 0.0,
            gain: Unit::ONE,
            unipolar: false,
        });
        patch.connect(freq, osc).unwrap();
        patch.output(osc).unwrap();
        CompiledPatch::new(&patch, rate).unwrap()
    }
}
