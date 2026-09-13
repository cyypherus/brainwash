use crate::model::ModuleId;
use brainwash::compile::{CompiledPatch, PatchControls};
use brainwash::compile::{PatchEngine, UpdateRejected};
use brainwash::patch::ModuleId as AudioModuleId;
use brainwash::sample::Frame;
use brainwash::sequence::{Player, VOICES};
use brainwash::time::SampleRate;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use rtrb::{Consumer, Producer, RingBuffer};
use std::fmt;
use std::num::{NonZeroU8, NonZeroU16};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub struct AudioRuntime {
    handle: Option<AudioHandle>,
    _stream: cpal::Stream,
}

pub struct AudioHandle {
    patch_pending: Producer<PatchBank>,
    patch_retired: Consumer<PatchBank>,
    track_pending: Producer<Player>,
    track_retired: Consumer<Player>,
    sequence_waiting: Option<Player>,
    play_pending: Producer<bool>,
    probe_bus: Arc<ProbeBus>,
    meter_bus: Arc<MeterBus>,
    rate: SampleRate,
    patch_generation: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProbeRoute {
    pub(crate) source: AudioModuleId,
    pub(crate) target: ModuleId,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MeterRoute {
    source: AudioModuleId,
    target: ModuleId,
    input_count: NonZeroU8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VoiceMode {
    Single,
    Polyphonic,
}

impl MeterRoute {
    pub(crate) fn new(source: AudioModuleId, target: ModuleId, input_count: usize) -> Option<Self> {
        if input_count > METER_INPUTS {
            return None;
        }
        Some(Self {
            source,
            target,
            input_count: NonZeroU8::new(input_count as u8)?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioCommandRejected;

impl VoiceMode {
    fn count(self) -> usize {
        match self {
            VoiceMode::Single => 1,
            VoiceMode::Polyphonic => VOICES,
        }
    }
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
const PLAY_COMMAND_LIMIT: usize = 2;
const PROBE_SLOTS: usize = 8;
const PROBE_HISTORY: usize = 88_200;
const PROBE_WRITE_BIT: u32 = 1 << 31;
const METER_SLOTS: usize = 128;
const METER_INPUTS: usize = 8;

impl AudioRuntime {
    pub fn start() -> Result<Self, AudioStartError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioStartError::NoOutputDevice)?;
        let supported = device
            .default_output_config()
            .map_err(AudioStartError::DefaultConfig)?;
        let rate =
            SampleRate::new(supported.sample_rate().0).ok_or(AudioStartError::InvalidSampleRate)?;
        let config = supported.config();
        let (patch_pending, callback_patch_pending) = RingBuffer::new(2);
        let (callback_patch_retired, patch_retired) = RingBuffer::new(2);
        let (track_pending, callback_track_pending) = RingBuffer::new(2);
        let (callback_track_retired, track_retired) = RingBuffer::new(2);
        let (play_pending, callback_play_pending) = RingBuffer::new(2);
        let probe_bus = Arc::new(ProbeBus::new());
        let meter_bus = Arc::new(MeterBus::new());
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config,
                callback_patch_pending,
                callback_patch_retired,
                callback_track_pending,
                callback_track_retired,
                callback_play_pending,
                Arc::clone(&probe_bus),
                Arc::clone(&meter_bus),
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config,
                callback_patch_pending,
                callback_patch_retired,
                callback_track_pending,
                callback_track_retired,
                callback_play_pending,
                Arc::clone(&probe_bus),
                Arc::clone(&meter_bus),
            )?,
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config,
                callback_patch_pending,
                callback_patch_retired,
                callback_track_pending,
                callback_track_retired,
                callback_play_pending,
                Arc::clone(&probe_bus),
                Arc::clone(&meter_bus),
            )?,
            format => return Err(AudioStartError::UnsupportedFormat(format)),
        };
        stream.play().map_err(AudioStartError::PlayStream)?;
        Ok(Self {
            handle: Some(AudioHandle {
                patch_pending,
                patch_retired,
                track_pending,
                track_retired,
                sequence_waiting: None,
                play_pending,
                probe_bus,
                meter_bus,
                rate,
                patch_generation: 0,
            }),
            _stream: stream,
        })
    }

    pub fn take_handle(&mut self) -> Option<AudioHandle> {
        self.handle.take()
    }
}

impl AudioHandle {
    pub fn sample_rate(&self) -> SampleRate {
        self.rate
    }

    pub fn set_playing(&mut self, playing: bool) -> Result<(), AudioCommandRejected> {
        self.play_pending
            .push(playing)
            .map_err(|_| AudioCommandRejected)
    }

    pub fn set_latest_patch(&mut self, patch: CompiledPatch) {
        self.set_latest_bank(PatchBank::new(patch));
    }

    pub(crate) fn set_latest_patch_with_telemetry(
        &mut self,
        patch: CompiledPatch,
        probes: &[ProbeRoute],
        meters: &[MeterRoute],
        voice_mode: VoiceMode,
    ) {
        let probe_routes = ProbeRoutes::new(probes);
        let meter_routes = MeterRoutes::new(meters);
        self.set_latest_bank(PatchBank::new_with_telemetry(
            patch,
            probe_routes,
            meter_routes,
            voice_mode,
        ));
    }

    fn set_latest_bank(&mut self, mut bank: PatchBank) {
        self.patch_generation = self.patch_generation.wrapping_add(1);
        bank.generation = self.patch_generation;
        eprintln!(
            "[bw dbg audio] publish patch gen={} voice_mode={:?} probes={} meters={}",
            bank.generation,
            bank.voice_mode,
            bank.probes.iter().count(),
            bank.meters.iter().count()
        );
        loop {
            self.collect_retired();
            match self.patch_pending.push(bank) {
                Ok(()) => {
                    eprintln!("[bw dbg audio] queued patch gen={}", self.patch_generation);
                    return;
                }
                Err(rtrb::PushError::Full(returned)) => {
                    bank = returned;
                    eprintln!(
                        "[bw dbg audio] waiting to queue patch gen={} transport_full",
                        bank.generation
                    );
                    std::thread::yield_now();
                }
            }
        }
    }

    pub(crate) fn probe_history(&self, module: ModuleId, voice: usize, len: usize) -> Vec<f32> {
        self.probe_bus.history(module, voice, len)
    }

    pub(crate) fn meter_values(&self, module: ModuleId, voice: usize) -> Vec<f32> {
        self.meter_bus.values(module, voice)
    }

    pub(crate) fn playhead(&self) -> f32 {
        f32::from_bits(self.probe_bus.playhead.load(Ordering::Relaxed))
    }

    pub(crate) fn submit_sequence(
        &mut self,
        sequence: brainwash::sequence::Sequence,
        bpm: u16,
        scale: brainwash::scale::Scale,
    ) {
        self.sequence_waiting = Some(Player::new(
            sequence,
            NonZeroU16::new(bpm).unwrap(),
            self.rate,
            scale,
        ));
        self.collect_retired();
    }

    pub fn collect_retired(&mut self) {
        while self.patch_retired.pop().is_ok() {}
        while self.track_retired.pop().is_ok() {}
        if let Some(sequence) = self.sequence_waiting.take()
            && let Err(rtrb::PushError::Full(sequence)) = self.track_pending.push(sequence)
        {
            self.sequence_waiting = Some(sequence);
        }
    }
}

impl fmt::Debug for AudioHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioHandle")
            .field("rate", &self.rate)
            .finish()
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    patch_pending: Consumer<PatchBank>,
    patch_retired: Producer<PatchBank>,
    track_pending: Consumer<Player>,
    track_retired: Producer<Player>,
    play_pending: Consumer<bool>,
    probe_bus: Arc<ProbeBus>,
    meter_bus: Arc<MeterBus>,
) -> Result<cpal::Stream, AudioStartError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut engine = VoicePatchRuntime::new(
        PatchBank::new(CompiledPatch::silence()),
        patch_pending,
        patch_retired,
        probe_bus,
        meter_bus,
    );
    let mut track_pending = track_pending;
    let mut track_retired = track_retired;
    let mut play_pending = play_pending;
    let mut track = None;
    let mut retired_track = None;
    let mut playing = false;
    let mut gain = 0.0;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                write_output(
                    data,
                    channels,
                    &mut play_pending,
                    &mut playing,
                    &mut engine,
                    &mut track_pending,
                    &mut track_retired,
                    &mut track,
                    &mut retired_track,
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
    play_pending: &mut Consumer<bool>,
    playing: &mut bool,
    engine: &mut VoicePatchRuntime,
    track_pending: &mut Consumer<Player>,
    track_retired: &mut Producer<Player>,
    track: &mut Option<Player>,
    retired_track: &mut Option<Player>,
    gain: &mut f32,
) where
    T: Sample + FromSample<f32>,
{
    return_retired_track(track_retired, retired_track);
    accept_track(track_pending, track, retired_track);
    for _ in 0..PLAY_COMMAND_LIMIT {
        let Ok(next) = play_pending.pop() else {
            break;
        };
        *playing = next;
    }
    let target = if *playing { 1.0 } else { 0.0 };
    let step = 1.0 / PLAY_FADE_FRAMES;
    engine.begin_buffer();
    if !*playing && *gain == 0.0 {
        for output in data {
            *output = T::from_sample(0.0);
        }
        return;
    }
    for output in data.chunks_mut(channels) {
        if *gain < target {
            *gain = (*gain + step).min(1.0);
        } else if *gain > target {
            *gain = (*gain - step).max(0.0);
        }
        let controls = track
            .as_mut()
            .filter(|_| *playing)
            .map(Player::advance)
            .unwrap_or_default();
        write_frame(output, engine.next_frame(controls), *gain);
    }
    if let Some(track) = track {
        engine
            .probe_bus
            .playhead
            .store(track.playhead().to_bits(), Ordering::Relaxed);
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

#[derive(Clone, Copy, Debug)]
struct ProbeSlotRoute {
    source: AudioModuleId,
    slot: usize,
    target: ModuleId,
}

#[derive(Clone, Copy, Debug)]
struct ProbeRoutes {
    entries: [Option<ProbeSlotRoute>; PROBE_SLOTS],
}

impl ProbeRoutes {
    fn empty() -> Self {
        Self {
            entries: [None; PROBE_SLOTS],
        }
    }

    fn new(routes: &[ProbeRoute]) -> Self {
        let mut routes_out = Self::empty();
        for (slot, route) in routes.iter().take(PROBE_SLOTS).enumerate() {
            routes_out.entries[slot] = Some(ProbeSlotRoute {
                source: route.source,
                slot,
                target: route.target,
            });
        }
        routes_out
    }

    fn iter(&self) -> impl Iterator<Item = ProbeSlotRoute> + '_ {
        self.entries.iter().flatten().copied()
    }
}

#[derive(Clone, Copy, Debug)]
struct MeterSlotRoute {
    source: AudioModuleId,
    slot: usize,
    target: ModuleId,
    inputs: usize,
}

#[derive(Clone, Copy, Debug)]
struct MeterRoutes {
    entries: [Option<MeterSlotRoute>; METER_SLOTS],
}

impl MeterRoutes {
    fn empty() -> Self {
        Self {
            entries: [None; METER_SLOTS],
        }
    }

    fn new(routes: &[MeterRoute]) -> Self {
        let mut routes_out = Self::empty();
        for (slot, route) in routes.iter().take(METER_SLOTS).enumerate() {
            routes_out.entries[slot] = Some(MeterSlotRoute {
                source: route.source,
                slot,
                target: route.target,
                inputs: route.input_count.get() as usize,
            });
        }
        routes_out
    }

    fn iter(&self) -> impl Iterator<Item = MeterSlotRoute> + '_ {
        self.entries.iter().flatten().copied()
    }
}

#[derive(Debug)]
struct ProbeBus {
    ids: [AtomicU32; PROBE_SLOTS],
    generations: [AtomicU32; PROBE_SLOTS],
    values: Box<[AtomicU32]>,
    sample_generations: Box<[AtomicU32]>,
    cursor: AtomicUsize,
    playhead: AtomicU32,
}

impl ProbeBus {
    fn new() -> Self {
        let count = VOICES * PROBE_SLOTS * PROBE_HISTORY;
        let mut values = Vec::with_capacity(count);
        let mut sample_generations = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(AtomicU32::new(0.0f32.to_bits()));
            sample_generations.push(AtomicU32::new(0));
        }
        Self {
            ids: std::array::from_fn(|_| AtomicU32::new(0)),
            generations: std::array::from_fn(|_| AtomicU32::new(0)),
            values: values.into_boxed_slice(),
            sample_generations: sample_generations.into_boxed_slice(),
            cursor: AtomicUsize::new(0),
            playhead: AtomicU32::new(0),
        }
    }

    fn publish_routes(&self, routes: &ProbeRoutes) {
        for id in &self.ids {
            id.store(0, Ordering::Release);
        }
        for route in routes.iter() {
            let slot = route.slot;
            self.generations[slot].fetch_add(1, Ordering::AcqRel);
            self.ids[slot].store(route.target.value().saturating_add(1), Ordering::Release);
        }
    }

    fn write(&self, slot: usize, voice: usize, cursor: usize, value: f32) {
        let index = probe_value_index(voice, slot, cursor);
        let generation = self.generations[slot].load(Ordering::Acquire);
        self.sample_generations[index].store(generation | PROBE_WRITE_BIT, Ordering::Release);
        self.values[index].store(value.to_bits(), Ordering::Release);
        self.sample_generations[index].store(generation, Ordering::Release);
    }

    fn advance(&self, cursor: usize) {
        self.cursor
            .store((cursor + 1) % PROBE_HISTORY, Ordering::Release);
    }

    fn history(&self, module: ModuleId, voice: usize, len: usize) -> Vec<f32> {
        let slot_id = module.value().saturating_add(1);
        let Some(slot) = self
            .ids
            .iter()
            .position(|id| id.load(Ordering::Acquire) == slot_id)
        else {
            return Vec::new();
        };
        let generation = self.generations[slot].load(Ordering::Acquire);
        let voice = voice.min(VOICES - 1);
        let count = len.min(PROBE_HISTORY);
        let cursor = self.cursor.load(Ordering::Acquire);
        let start = (cursor + PROBE_HISTORY - count) % PROBE_HISTORY;
        let mut values = Vec::with_capacity(count);
        for offset in 0..count {
            let index = (start + offset) % PROBE_HISTORY;
            let value_index = probe_value_index(voice, slot, index);
            let sample_generation = self.sample_generations[value_index].load(Ordering::Acquire);
            let value = if sample_generation == generation {
                let bits = self.values[value_index].load(Ordering::Acquire);
                if self.sample_generations[value_index].load(Ordering::Acquire) == sample_generation
                {
                    f32::from_bits(bits)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            values.push(value);
        }
        values
    }
}

fn probe_value_index(voice: usize, slot: usize, cursor: usize) -> usize {
    ((voice * PROBE_SLOTS) + slot) * PROBE_HISTORY + cursor
}

#[derive(Debug)]
struct MeterBus {
    ids: [AtomicU32; METER_SLOTS],
    inputs: [AtomicUsize; METER_SLOTS],
    values: Box<[AtomicU32]>,
}

impl MeterBus {
    fn new() -> Self {
        let count = VOICES * METER_SLOTS * METER_INPUTS;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(AtomicU32::new(0.0f32.to_bits()));
        }
        Self {
            ids: std::array::from_fn(|_| AtomicU32::new(0)),
            inputs: std::array::from_fn(|_| AtomicUsize::new(0)),
            values: values.into_boxed_slice(),
        }
    }

    fn publish_routes(&self, routes: &MeterRoutes) {
        for slot in 0..METER_SLOTS {
            self.ids[slot].store(0, Ordering::Release);
            self.inputs[slot].store(0, Ordering::Release);
        }
        for route in routes.iter() {
            let slot = route.slot;
            self.clear_slot(slot);
            self.ids[slot].store(route.target.value().saturating_add(1), Ordering::Release);
            self.inputs[slot].store(route.inputs, Ordering::Release);
        }
    }

    fn clear_slot(&self, slot: usize) {
        for voice in 0..VOICES {
            for input in 0..METER_INPUTS {
                self.values[meter_value_index(voice, slot, input)]
                    .store(0.0f32.to_bits(), Ordering::Release);
            }
        }
    }

    fn clear(&self, routes: &MeterRoutes) {
        for route in routes.iter() {
            self.clear_slot(route.slot);
        }
    }

    fn write(&self, slot: usize, voice: usize, input: usize, value: f32) {
        if voice >= VOICES || input >= METER_INPUTS {
            return;
        }
        let index = meter_value_index(voice, slot, input);
        let peak = value.abs();
        if peak > f32::from_bits(self.values[index].load(Ordering::Acquire)) {
            self.values[index].store(peak.to_bits(), Ordering::Release);
        }
    }

    fn values(&self, module: ModuleId, voice: usize) -> Vec<f32> {
        let slot_id = module.value().saturating_add(1);
        let Some(slot) = self
            .ids
            .iter()
            .position(|id| id.load(Ordering::Acquire) == slot_id)
        else {
            return Vec::new();
        };
        let voice = voice.min(VOICES - 1);
        let inputs = self.inputs[slot].load(Ordering::Acquire);
        let mut values = Vec::with_capacity(inputs);
        for input in 0..inputs {
            values.push(f32::from_bits(
                self.values[meter_value_index(voice, slot, input)].load(Ordering::Acquire),
            ));
        }
        values
    }
}

fn meter_value_index(voice: usize, slot: usize, input: usize) -> usize {
    ((voice * METER_SLOTS) + slot) * METER_INPUTS + input
}

#[derive(Debug)]
struct PatchBank {
    voices: [CompiledPatch; VOICES],
    probes: ProbeRoutes,
    meters: MeterRoutes,
    voice_mode: VoiceMode,
    generation: u64,
}

struct VoicePatchRuntime {
    voices: [PatchEngine; VOICES],
    pending: Consumer<PatchBank>,
    retired_output: Producer<PatchBank>,
    pending_bank: Option<PatchBank>,
    probe_bus: Arc<ProbeBus>,
    meter_bus: Arc<MeterBus>,
    probes: ProbeRoutes,
    meters: MeterRoutes,
    voice_mode: VoiceMode,
    retired: Option<PatchBank>,
    probe_cursor: usize,
    blocked_generation: Option<u64>,
}

impl PatchBank {
    fn new(patch: CompiledPatch) -> Self {
        Self::new_with_telemetry(
            patch,
            ProbeRoutes::empty(),
            MeterRoutes::empty(),
            VoiceMode::Polyphonic,
        )
    }

    fn new_with_telemetry(
        patch: CompiledPatch,
        probes: ProbeRoutes,
        meters: MeterRoutes,
        voice_mode: VoiceMode,
    ) -> Self {
        Self {
            voices: std::array::from_fn(|_| patch.clone()),
            probes,
            meters,
            voice_mode,
            generation: 0,
        }
    }
}

impl VoicePatchRuntime {
    fn new(
        bank: PatchBank,
        pending: Consumer<PatchBank>,
        retired_output: Producer<PatchBank>,
        probe_bus: Arc<ProbeBus>,
        meter_bus: Arc<MeterBus>,
    ) -> Self {
        let probes = bank.probes;
        let meters = bank.meters;
        let voice_mode = bank.voice_mode;
        probe_bus.publish_routes(&probes);
        meter_bus.publish_routes(&meters);
        Self {
            voices: bank.voices.map(PatchEngine::new),
            pending,
            retired_output,
            pending_bank: None,
            probe_bus,
            meter_bus,
            probes,
            meters,
            voice_mode,
            retired: None,
            probe_cursor: 0,
            blocked_generation: None,
        }
    }

    #[cfg(test)]
    fn next_with_controls(&mut self, controls: [PatchControls; VOICES]) -> Frame {
        self.begin_buffer();
        self.next_frame(controls)
    }

    fn begin_buffer(&mut self) {
        self.return_retired();
        self.read_pending();
        self.accept_pending();
        self.meter_bus.clear(&self.meters);
    }

    fn next_frame(&mut self, controls: [PatchControls; VOICES]) -> Frame {
        let mut left = 0.0;
        let mut right = 0.0;
        let active_voices = self.voice_mode.count();
        for (voice, (engine, controls)) in self.voices.iter_mut().zip(controls).enumerate() {
            if voice >= active_voices && engine.can_replace() {
                continue;
            }
            let frame = engine.next_with_controls(controls);
            if voice < active_voices {
                record_probe_values(
                    &self.probe_bus,
                    &self.probes,
                    self.probe_cursor,
                    voice,
                    engine,
                );
                record_meter_values(&self.meter_bus, &self.meters, voice, engine);
                left += frame.left().value();
                right += frame.right().value();
            }
        }
        self.probe_bus.advance(self.probe_cursor);
        self.probe_cursor = (self.probe_cursor + 1) % PROBE_HISTORY;
        Frame::stereo(
            brainwash::sample::Sample::new(left).unwrap_or_default(),
            brainwash::sample::Sample::new(right).unwrap_or_default(),
        )
    }

    fn return_retired(&mut self) {
        if let Some(bank) = self.retired.take()
            && let Err(error) = self.retired_output.push(bank)
        {
            let rtrb::PushError::Full(bank) = error;
            self.retired = Some(bank);
            return;
        }
        if self.voices.iter().any(|engine| !engine.retired_pending()) {
            return;
        }
        let bank = PatchBank {
            voices: std::array::from_fn(|index| self.voices[index].take_retired().unwrap()),
            probes: ProbeRoutes::empty(),
            meters: MeterRoutes::empty(),
            voice_mode: self.voice_mode,
            generation: 0,
        };
        if let Err(error) = self.retired_output.push(bank) {
            let rtrb::PushError::Full(bank) = error;
            eprintln!("[bw dbg audio] retired output full while returning active bank");
            self.retired = Some(bank);
        }
    }

    fn read_pending(&mut self) {
        if self.retired.is_some() {
            return;
        }
        while let Ok(bank) = self.pending.pop() {
            let generation = bank.generation;
            eprintln!(
                "[bw dbg audio] callback received patch gen={} voice_mode={:?}",
                generation, bank.voice_mode
            );
            if let Some(old) = self.pending_bank.replace(bank)
                && let Err(error) = self.retired_output.push(old)
            {
                let rtrb::PushError::Full(old) = error;
                eprintln!(
                    "[bw dbg audio] retired output full while superseding patch gen={}",
                    old.generation
                );
                self.retired = Some(old);
                return;
            }
            eprintln!("[bw dbg audio] latest pending patch gen={generation}");
        }
    }

    fn accept_pending(&mut self) {
        if self.retired.is_some() || self.voices.iter().any(|engine| !engine.can_replace()) {
            if let Some(bank) = &self.pending_bank
                && self.blocked_generation != Some(bank.generation)
            {
                eprintln!(
                    "[bw dbg audio] install blocked gen={} retired={} replaceable={}",
                    bank.generation,
                    self.retired.is_some(),
                    self.voices.iter().all(|engine| engine.can_replace())
                );
                self.blocked_generation = Some(bank.generation);
            }
            return;
        }
        let Some(bank) = self.pending_bank.take() else {
            return;
        };
        let generation = bank.generation;
        let PatchBank {
            voices,
            probes,
            meters,
            voice_mode,
            generation: _,
        } = bank;
        for (engine, next) in self.voices.iter_mut().zip(voices) {
            if let Err(UpdateRejected::Busy(_) | UpdateRejected::RetiredPatchPending(_)) =
                engine.replace(next)
            {
                eprintln!("[bw dbg audio] install rejected unexpectedly gen={generation}");
                return;
            }
        }
        self.probe_bus.publish_routes(&probes);
        self.meter_bus.publish_routes(&meters);
        self.probes = probes;
        self.meters = meters;
        self.voice_mode = voice_mode;
        self.blocked_generation = None;
        eprintln!(
            "[bw dbg audio] installed patch gen={} voice_mode={:?} probes={} meters={}",
            generation,
            self.voice_mode,
            self.probes.iter().count(),
            self.meters.iter().count()
        );
    }
}

fn record_probe_values(
    bus: &ProbeBus,
    probes: &ProbeRoutes,
    cursor: usize,
    voice: usize,
    engine: &PatchEngine,
) {
    for (module, value) in engine.probe_values() {
        if let Some(route) = probes.iter().find(|route| route.source == module) {
            bus.write(route.slot, voice, cursor, value.value());
        }
    }
}

fn record_meter_values(bus: &MeterBus, meters: &MeterRoutes, voice: usize, engine: &PatchEngine) {
    for route in meters.iter() {
        engine.visit_module_input_values(route.source, |input, value| {
            bus.write(route.slot, voice, input, value.value());
        });
    }
}

fn return_retired_track(retired_output: &mut Producer<Player>, retired: &mut Option<Player>) {
    if let Some(track) = retired.take()
        && let Err(error) = retired_output.push(track)
    {
        let rtrb::PushError::Full(track) = error;
        *retired = Some(track);
    }
}

fn accept_track(
    pending: &mut Consumer<Player>,
    track: &mut Option<Player>,
    retired: &mut Option<Player>,
) {
    if retired.is_some() {
        return;
    }
    if let Ok(next) = pending.pop() {
        if let Some(player) = track {
            *retired = Some(player.replace(next));
        } else {
            *track = Some(next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GuiAction, GuiState, ModuleCategory};
    use assert_no_alloc::assert_no_alloc;
    use brainwash::patch::{BinaryOp, InputKind, Module, Patch, Resonance};
    use brainwash::sample::Sample as AudioSample;
    use brainwash::scale::cmin;
    use brainwash::sequence::parse_sequence;
    use brainwash::time::Hertz;

    #[test]
    fn painted_audio_sustains_glides_and_routes_expression_without_allocating() {
        use brainwash::sequence::{Sequence, Stroke};
        let rate = SampleRate::new(1000).unwrap();
        let sequence = Sequence::new(
            vec![Stroke::try_from(vec![[0.0, 60.0, -1.0], [1.0, 72.0, 1.0]]).unwrap()],
            1,
        )
        .unwrap();
        let mut runtime = Player::new(sequence, NonZeroU16::new(120).unwrap(), rate, cmin());
        let mut patch = Patch::new();
        let expression = patch.insert(Module::Expression);
        patch
            .output(patch.output_port(expression, 0).unwrap())
            .unwrap();
        let mut engine = CompiledPatch::new(&patch, rate).unwrap();
        let mut first = None;
        let mut last = None;
        assert_no_alloc(|| {
            for _ in 0..1000 {
                let controls = runtime.advance();
                assert_eq!(controls[0].gate, 1.0);
                let frame = engine.next_with_controls(controls[0]);
                assert_eq!(frame.left().value(), controls[0].expression);
                if first.is_none() {
                    first = Some(controls[0]);
                }
                last = Some(controls[0]);
            }
        });
        let first = first.unwrap();
        let last = last.unwrap();
        assert!(last.frequency.unwrap().value() > first.frequency.unwrap().value() * 1.4);
        assert!((last.expression - first.expression) > 0.99);
    }

    #[test]
    fn painted_edit_preserves_transport_and_sustained_gate() {
        use brainwash::sequence::{Sequence, Stroke};
        let mut sequence = Sequence::new(
            vec![Stroke::try_from(vec![[0.0, 60.0, 0.0], [1.0, 72.0, 1.0]]).unwrap()],
            1,
        )
        .unwrap();
        let rate = SampleRate::new(1000).unwrap();
        let mut playing = Player::new(
            sequence.clone(),
            NonZeroU16::new(120).unwrap(),
            rate,
            cmin(),
        );
        for _ in 0..800 {
            assert_eq!(playing.advance()[0].gate, 1.0);
        }
        assert_eq!(playing.advance()[0].gate, 1.0);
        let phase = playing.playhead();
        sequence = Sequence::new(
            vec![Stroke::try_from(vec![[0.0, 60.0, -1.0], [1.0, 72.0, -1.0]]).unwrap()],
            1,
        )
        .unwrap();
        let (mut input, mut output) = RingBuffer::new(1);
        assert!(
            input
                .push(Player::new(
                    sequence,
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin()
                ))
                .is_ok()
        );
        let mut playing = Some(playing);
        let mut retired = None;
        assert_no_alloc(|| accept_track(&mut output, &mut playing, &mut retired));
        let playing = playing.as_mut().unwrap();
        assert_eq!(playing.playhead(), phase);
        let controls = playing.advance();
        assert_eq!(controls[0].gate, 1.0);
        assert_eq!(controls[0].expression, -1.0);
    }

    #[test]
    fn set_playing_reports_rejection_when_queue_is_full() {
        let mut handle = test_handle();

        assert_eq!(handle.set_playing(true), Ok(()));
        assert_eq!(handle.set_playing(false), Ok(()));
        assert_eq!(handle.set_playing(true), Err(AudioCommandRejected));
    }

    #[test]
    fn patch_update_waits_for_transport_slot_instead_of_rejecting() {
        let rate = SampleRate::new(44_100).unwrap();
        let (patch_pending, mut callback_patch_pending) = RingBuffer::new(1);
        let (_patch_retired_callback, patch_retired) = RingBuffer::new(2);
        let (track_pending, _track_callback) = RingBuffer::new(2);
        let (_track_retired_callback, track_retired) = RingBuffer::new(2);
        let (play_pending, _play_callback) = RingBuffer::new(2);
        let mut handle = AudioHandle {
            patch_pending,
            patch_retired,
            track_pending,
            track_retired,
            sequence_waiting: None,
            play_pending,
            probe_bus: Arc::new(ProbeBus::new()),
            meter_bus: Arc::new(MeterBus::new()),
            rate,
            patch_generation: 0,
        };
        handle.set_latest_patch(compiled_saw_patch(110.0, rate));
        let release = std::thread::spawn(move || {
            while callback_patch_pending.pop().is_err() {
                std::thread::yield_now();
            }
            callback_patch_pending
        });

        handle.set_latest_patch(compiled_saw_patch(220.0, rate));

        let mut callback_patch_pending = release.join().unwrap();
        assert!(callback_patch_pending.pop().is_ok());
    }

    #[test]
    fn gui_compile_error_publishes_silence_after_valid_patch() {
        let rate = SampleRate::new(44_100).unwrap();
        let (patch_pending, mut callback_patch_pending) = RingBuffer::new(16);
        let (_patch_retired_callback, patch_retired) = RingBuffer::new(16);
        let (track_pending, _track_callback) = RingBuffer::new(2);
        let (_track_retired_callback, track_retired) = RingBuffer::new(2);
        let (play_pending, _play_callback) = RingBuffer::new(2);
        let mut state = GuiState::new(8, 8);
        state.set_audio(AudioHandle {
            patch_pending,
            patch_retired,
            track_pending,
            track_retired,
            sequence_waiting: None,
            play_pending,
            probe_bus: Arc::new(ProbeBus::new()),
            meter_bus: Arc::new(MeterBus::new()),
            rate,
            patch_generation: 0,
        });
        drain_latest_patch(&mut callback_patch_pending);

        state.apply(GuiAction::OpenPalette);
        state.apply(GuiAction::Confirm);
        state.apply(GuiAction::Right);
        state.apply(GuiAction::Palette(ModuleCategory::Composition));
        state.apply(GuiAction::Confirm);
        let mut valid = drain_latest_patch(&mut callback_patch_pending)
            .expect("valid gui patch should publish audio");
        let mut audible = false;
        for _ in 0..128 {
            let frame = valid.voices[0].next();
            audible |= frame.left().value().abs() > 0.001 || frame.right().value().abs() > 0.001;
        }
        assert!(audible);

        state.apply(GuiAction::Delete);
        assert_eq!(state.audio_status(), "Silent: connect signal to Output");
        let mut silent = drain_latest_patch(&mut callback_patch_pending)
            .expect("invalid gui patch should publish silence");
        for _ in 0..128 {
            let frame = silent.voices[0].next();
            assert_eq!(frame.left().value(), 0.0);
            assert_eq!(frame.right().value(), 0.0);
        }
    }

    #[test]
    fn sequence_queue_keeps_the_latest_edit_under_backpressure() {
        let mut handle = test_handle();
        for text in ["{0}", "{2}", "{4}", "{7}"] {
            handle.submit_sequence(parse_sequence(text, &cmin()).unwrap(), 120, cmin());
        }
        let latest = handle.sequence_waiting.as_mut().unwrap();
        assert_eq!(latest.advance()[0].degree, 7);
    }

    #[test]
    fn track_runtime_exposes_polyphonic_voice_controls() {
        let track = parse_sequence("{0&2}", &cmin()).unwrap();
        let mut runtime = Player::new(
            track,
            NonZeroU16::new(120).unwrap(),
            SampleRate::new(44_100).unwrap(),
            cmin(),
        );

        let controls = runtime.advance();
        let active = controls.iter().filter(|control| control.gate > 0.5).count();

        assert_eq!(active, 2);
    }

    #[test]
    fn overlapping_full_level_voices_are_hard_limited_at_output() {
        let rate = SampleRate::new(44_100).unwrap();
        let mut patch = Patch::new();
        let gate = patch.insert(Module::Gate);
        patch.output(patch.output_port(gate, 0).unwrap()).unwrap();
        let (_pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, _retired_output) = RingBuffer::new(2);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(CompiledPatch::new(&patch, rate).unwrap()),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        for count in [6, 1, 3, 0, 6] {
            let controls = std::array::from_fn(|index| PatchControls {
                gate: if index < count { 1.0 } else { 0.0 },
                ..Default::default()
            });
            let frame = runtime.next_with_controls(controls);
            assert_eq!(frame.left().value(), count as f32);
            let mut output = [0.0_f32; 2];
            write_frame(&mut output, frame, 1.0);
            assert_eq!(output, [(count as f32).min(1.0); 2]);
        }
        let frame = Frame::stereo(
            AudioSample::new(-6.0).unwrap(),
            AudioSample::new(6.0).unwrap(),
        );
        let mut output = [0.0_f32; 2];
        write_frame(&mut output, frame, 1.0);
        assert_eq!(output, [-1.0, 1.0]);
    }

    #[test]
    fn voice_patch_runtime_sums_independent_voices() {
        let rate = SampleRate::new(44_100).unwrap();
        let mut patch = Patch::new();
        let freq = patch.insert(Module::Freq);
        let osc = patch.insert(brainwash::preset::saw(Hertz::new(110.0).unwrap()));
        let port = patch.input_port(osc, InputKind::Freq).unwrap();
        patch
            .connect_input(patch.output_port(freq, 0).unwrap(), port)
            .unwrap();
        patch.output(patch.output_port(osc, 0).unwrap()).unwrap();
        let compiled = CompiledPatch::new(&patch, rate).unwrap();
        let (_pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, _retired_output) = RingBuffer::new(2);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        let mut controls = [PatchControls::default(); VOICES];
        controls[0] = PatchControls {
            frequency: Hertz::new(220.0),
            gate: 1.0,
            degree: 0,
            expression: 0.0,
        };
        controls[1] = PatchControls {
            frequency: Hertz::new(330.0),
            gate: 1.0,
            degree: 2,
            expression: 0.0,
        };

        let mut audible = false;
        for _ in 0..64 {
            let frame = runtime.next_with_controls(controls);
            audible |= frame.left().value().abs() > 0.001 || frame.right().value().abs() > 0.001;
        }

        assert!(audible);
    }

    #[test]
    fn single_voice_mode_does_not_stack_track_independent_patch() {
        let rate = SampleRate::new(44_100).unwrap();
        let (_single_pending_input, single_pending_output) = RingBuffer::new(2);
        let (single_retired_input, _single_retired_output) = RingBuffer::new(2);
        let (_gated_pending_input, gated_pending_output) = RingBuffer::new(2);
        let (gated_retired_input, _gated_retired_output) = RingBuffer::new(2);
        let mut single = VoicePatchRuntime::new(
            PatchBank::new_with_telemetry(
                unmodulated_osc_patch(rate),
                ProbeRoutes::empty(),
                MeterRoutes::empty(),
                VoiceMode::Single,
            ),
            single_pending_output,
            single_retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        let mut gated = VoicePatchRuntime::new(
            PatchBank::new(gated_osc_patch(rate)),
            gated_pending_output,
            gated_retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        let mut controls = [PatchControls::default(); VOICES];
        controls[0] = PatchControls {
            frequency: Hertz::new(330.0),
            gate: 1.0,
            degree: 0,
            expression: 0.0,
        };

        for _ in 0..256 {
            let default = single.next_with_controls(controls).left().value();
            let gated = gated.next_with_controls(controls).left().value();
            assert!((default - gated).abs() < 0.0001, "{default} {gated}");
        }
    }

    #[test]
    fn single_voice_mode_retires_inactive_engines_after_patch_swap() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, mut retired_output) = RingBuffer::new(2);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new_with_telemetry(
                CompiledPatch::silence(),
                ProbeRoutes::empty(),
                MeterRoutes::empty(),
                VoiceMode::Single,
            ),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        assert!(
            pending_input
                .push(PatchBank::new_with_telemetry(
                    compiled_saw_patch(220.0, rate),
                    ProbeRoutes::empty(),
                    MeterRoutes::empty(),
                    VoiceMode::Single,
                ))
                .is_ok()
        );

        assert_no_alloc(|| {
            for _ in 0..130 {
                runtime.next_with_controls([PatchControls::default(); VOICES]);
            }
        });

        assert!(retired_output.pop().is_ok());
    }

    #[test]
    fn patch_runtime_installs_latest_pending_patch_after_busy_swap() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, mut retired_output) = RingBuffer::new(8);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new_with_telemetry(
                constant_patch(0.0, rate),
                ProbeRoutes::empty(),
                MeterRoutes::empty(),
                VoiceMode::Single,
            ),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        let controls = [PatchControls::default(); VOICES];

        assert!(
            pending_input
                .push(PatchBank::new_with_telemetry(
                    constant_patch(0.25, rate),
                    ProbeRoutes::empty(),
                    MeterRoutes::empty(),
                    VoiceMode::Single,
                ))
                .is_ok()
        );
        runtime.next_with_controls(controls);
        assert!(
            pending_input
                .push(PatchBank::new_with_telemetry(
                    constant_patch(0.5, rate),
                    ProbeRoutes::empty(),
                    MeterRoutes::empty(),
                    VoiceMode::Single,
                ))
                .is_ok()
        );
        assert!(
            pending_input
                .push(PatchBank::new_with_telemetry(
                    constant_patch(0.75, rate),
                    ProbeRoutes::empty(),
                    MeterRoutes::empty(),
                    VoiceMode::Single,
                ))
                .is_ok()
        );

        assert_no_alloc(|| {
            for _ in 0..260 {
                runtime.next_with_controls(controls);
            }
        });

        while retired_output.pop().is_ok() {}
        let value = runtime.next_with_controls(controls).left().value();
        assert!((value - 0.75).abs() < 0.001, "{value}");
    }

    #[test]
    fn voice_patch_runtime_returns_retired_bank_after_crossfade() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, mut retired_output) = RingBuffer::new(2);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled_saw_patch(110.0, rate)),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        assert!(
            pending_input
                .push(PatchBank::new(compiled_saw_patch(220.0, rate)))
                .is_ok()
        );
        let controls = [PatchControls {
            frequency: Hertz::new(330.0),
            gate: 1.0,
            degree: 0,
            expression: 0.0,
        }; VOICES];

        runtime.next_with_controls(controls);
        assert!(retired_output.pop().is_err());

        for _ in 0..127 {
            runtime.next_with_controls(controls);
        }
        runtime.next_with_controls(controls);

        assert!(retired_output.pop().is_ok());
    }

    #[test]
    fn voice_patch_runtime_swaps_and_records_meters_without_allocation() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, mut retired_output) = RingBuffer::new(2);
        let meter_bus = Arc::new(MeterBus::new());
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled_saw_patch(110.0, rate)),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::clone(&meter_bus),
        );
        let (patch, route) = metered_patch(rate);
        assert!(
            pending_input
                .push(PatchBank::new_with_telemetry(
                    patch,
                    ProbeRoutes::empty(),
                    MeterRoutes::new(&[route]),
                    VoiceMode::Polyphonic,
                ))
                .is_ok()
        );
        let controls = [PatchControls::default(); VOICES];

        assert_no_alloc(|| {
            for _ in 0..130 {
                runtime.next_with_controls(controls);
            }
        });

        assert!(retired_output.pop().is_ok());
        assert_eq!(meter_bus.values(route.target, 0).len(), 1);
    }

    #[test]
    fn voice_patch_runtime_records_meter_values() {
        let rate = SampleRate::new(44_100).unwrap();
        let (_pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, _retired_output) = RingBuffer::new(2);
        let meter_bus = Arc::new(MeterBus::new());
        let (patch, route) = metered_patch(rate);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new_with_telemetry(
                patch,
                ProbeRoutes::empty(),
                MeterRoutes::new(&[route]),
                VoiceMode::Polyphonic,
            ),
            pending_output,
            retired_input,
            Arc::new(ProbeBus::new()),
            Arc::clone(&meter_bus),
        );
        let controls = [PatchControls::default(); VOICES];

        runtime.begin_buffer();
        runtime.next_frame(controls);

        assert_ne!(meter_bus.values(route.target, 0), vec![0.0]);
    }

    #[test]
    fn meter_bus_keeps_the_highest_magnitude_in_a_buffer() {
        let bus = MeterBus::new();

        bus.write(0, 0, 0, -0.75);
        bus.write(0, 0, 0, 0.25);

        let value = f32::from_bits(bus.values[meter_value_index(0, 0, 0)].load(Ordering::Acquire));
        assert_eq!(value, 0.75);
    }

    #[test]
    fn probe_meter_records_the_value_at_its_input_port() {
        let rate = SampleRate::new(44_100).unwrap();
        let target = telemetry_module_id();
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(AudioSample::new(0.25).unwrap()));
        let probe = patch.insert(Module::Probe {
            input: AudioSample::ZERO,
        });
        let port = patch.input_port(probe, InputKind::In).unwrap();
        patch
            .connect_input(patch.output_port(source, 0).unwrap(), port)
            .unwrap();
        patch.output(patch.output_port(probe, 0).unwrap()).unwrap();
        let probe_bus = Arc::new(ProbeBus::new());
        let meter_bus = Arc::new(MeterBus::new());
        let (_pending_input, pending_output) = RingBuffer::new(2);
        let (retired_input, _retired_output) = RingBuffer::new(2);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new_with_telemetry(
                CompiledPatch::new(&patch, rate).unwrap(),
                ProbeRoutes::new(&[ProbeRoute {
                    source: probe,
                    target,
                }]),
                MeterRoutes::new(&[MeterRoute::new(probe, target, 1).unwrap()]),
                VoiceMode::Single,
            ),
            pending_output,
            retired_input,
            Arc::clone(&probe_bus),
            Arc::clone(&meter_bus),
        );

        runtime.next_frame([PatchControls::default(); VOICES]);

        assert_eq!(probe_bus.history(target, 0, 1), meter_bus.values(target, 0));
    }

    #[test]
    fn track_swap_accepts_and_retires_without_allocation() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut pending_input, mut pending_output) = RingBuffer::new(2);
        let (mut retired_input, mut retired_output) = RingBuffer::new(2);
        let mut track = None;
        let mut retired = None;
        assert!(
            pending_input
                .push(Player::new(
                    parse_sequence("{0&2}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );

        assert_no_alloc(|| {
            return_retired_track(&mut retired_input, &mut retired);
            accept_track(&mut pending_output, &mut track, &mut retired);
            let controls = track.as_mut().unwrap().advance();
            assert_eq!(
                controls
                    .iter()
                    .filter(|control| control.gate == 1.0)
                    .count(),
                2
            );
        });
        assert!(track.is_some());

        assert!(
            pending_input
                .push(Player::new(
                    parse_sequence("{4}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );

        assert_no_alloc(|| {
            return_retired_track(&mut retired_input, &mut retired);
            accept_track(&mut pending_output, &mut track, &mut retired);
            return_retired_track(&mut retired_input, &mut retired);
        });

        assert!(retired_output.pop().is_ok());
    }

    #[test]
    fn write_output_processes_swaps_and_telemetry_without_allocation() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut patch_input, patch_output) = RingBuffer::new(2);
        let (patch_retired_input, mut patch_retired_output) = RingBuffer::new(2);
        let (mut track_input, mut track_output) = RingBuffer::new(2);
        let (mut track_retired_input, mut track_retired_output) = RingBuffer::new(2);
        let (mut play_input, mut play_output) = RingBuffer::new(2);
        let meter_bus = Arc::new(MeterBus::new());
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled_saw_patch(110.0, rate)),
            patch_output,
            patch_retired_input,
            Arc::new(ProbeBus::new()),
            Arc::clone(&meter_bus),
        );
        let (patch, route) = metered_patch(rate);
        assert!(
            patch_input
                .push(PatchBank::new_with_telemetry(
                    patch,
                    ProbeRoutes::empty(),
                    MeterRoutes::new(&[route]),
                    VoiceMode::Polyphonic,
                ))
                .is_ok()
        );
        assert!(
            track_input
                .push(Player::new(
                    parse_sequence("{0&2}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );
        assert!(
            track_input
                .push(Player::new(
                    parse_sequence("{4}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );
        assert!(play_input.push(true).is_ok());
        let mut track = None;
        let mut retired_track = None;
        let mut gain = 0.0;
        let mut playing = false;
        let mut data = [0.0f32; 128];

        assert_no_alloc(|| {
            for _ in 0..130 {
                write_output(
                    &mut data,
                    2,
                    &mut play_output,
                    &mut playing,
                    &mut runtime,
                    &mut track_output,
                    &mut track_retired_input,
                    &mut track,
                    &mut retired_track,
                    &mut gain,
                );
            }
        });

        assert!(patch_retired_output.pop().is_ok());
        assert!(track_retired_output.pop().is_ok());
        assert_eq!(meter_bus.values(route.target, 0).len(), 1);
    }

    #[test]
    fn write_output_does_not_process_when_paused() {
        let rate = SampleRate::new(44_100).unwrap();
        let (_patch_input, patch_output) = RingBuffer::new(2);
        let (patch_retired_input, _patch_retired_output) = RingBuffer::new(2);
        let (_track_input, mut track_output) = RingBuffer::new(2);
        let (mut track_retired_input, _track_retired_output) = RingBuffer::new(2);
        let (_play_input, mut play_output) = RingBuffer::new(2);
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled_saw_patch(110.0, rate)),
            patch_output,
            patch_retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        let mut track = None;
        let mut retired_track = None;
        let mut gain = 0.0;
        let mut playing = false;
        let mut data = [1.0f32; 128];

        write_output(
            &mut data,
            2,
            &mut play_output,
            &mut playing,
            &mut runtime,
            &mut track_output,
            &mut track_retired_input,
            &mut track,
            &mut retired_track,
            &mut gain,
        );

        assert_eq!(runtime.probe_cursor, 0);
        assert!(data.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn write_output_handles_retired_backpressure_without_allocation() {
        let rate = SampleRate::new(44_100).unwrap();
        let (mut patch_input, patch_output) = RingBuffer::new(2);
        let (mut patch_retired_input, mut patch_retired_output) = RingBuffer::new(1);
        let (mut track_input, mut track_output) = RingBuffer::new(2);
        let (mut track_retired_input, mut track_retired_output) = RingBuffer::new(1);
        let (mut play_input, mut play_output) = RingBuffer::new(2);
        assert!(
            patch_retired_input
                .push(PatchBank::new(compiled_saw_patch(55.0, rate)))
                .is_ok()
        );
        assert!(
            track_retired_input
                .push(Player::new(
                    parse_sequence("{7}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );
        let mut runtime = VoicePatchRuntime::new(
            PatchBank::new(compiled_saw_patch(110.0, rate)),
            patch_output,
            patch_retired_input,
            Arc::new(ProbeBus::new()),
            Arc::new(MeterBus::new()),
        );
        assert!(
            patch_input
                .push(PatchBank::new(compiled_saw_patch(220.0, rate)))
                .is_ok()
        );
        assert!(
            track_input
                .push(Player::new(
                    parse_sequence("{0&2}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );
        assert!(
            track_input
                .push(Player::new(
                    parse_sequence("{4}", &cmin()).unwrap(),
                    NonZeroU16::new(120).unwrap(),
                    rate,
                    cmin(),
                ))
                .is_ok()
        );
        assert!(play_input.push(true).is_ok());
        let mut track = None;
        let mut retired_track = None;
        let mut gain = 0.0;
        let mut playing = false;
        let mut data = [0.0f32; 128];

        assert_no_alloc(|| {
            for _ in 0..4 {
                write_output(
                    &mut data,
                    2,
                    &mut play_output,
                    &mut playing,
                    &mut runtime,
                    &mut track_output,
                    &mut track_retired_input,
                    &mut track,
                    &mut retired_track,
                    &mut gain,
                );
            }
        });

        assert!(patch_retired_output.pop().is_ok());
        assert!(track_retired_output.pop().is_ok());

        assert_no_alloc(|| {
            write_output(
                &mut data,
                2,
                &mut play_output,
                &mut playing,
                &mut runtime,
                &mut track_output,
                &mut track_retired_input,
                &mut track,
                &mut retired_track,
                &mut gain,
            );
        });

        assert!(patch_retired_output.pop().is_ok());
        assert!(track_retired_output.pop().is_ok());
    }

    #[test]
    fn audio_realtime_source_excludes_forbidden_primitives() {
        let source = include_str!("audio.rs");
        for token in [
            concat!("un", "safe"),
            concat!("Unsafe", "Cell"),
            concat!("Atomic", "Ptr"),
            concat!("into", "_raw"),
            concat!("from", "_raw"),
            concat!("Mut", "ex"),
            concat!("Rw", "Lock"),
            concat!(".lo", "ck("),
            concat!("try", "_lock"),
        ] {
            assert!(!source.contains(token), "{token}");
        }
    }

    #[test]
    fn audio_callback_functions_exclude_allocator_shapes() {
        let source = include_str!("audio.rs");
        for name in [
            "fn write_output",
            "fn return_retired_track",
            "fn accept_track",
            "fn next_with_controls",
            "fn begin_buffer",
            "fn next_frame",
            "fn return_retired",
            "fn accept_pending",
            "fn record_probe_values",
            "fn record_meter_values",
        ] {
            let body = function_body(source, name);
            for token in [
                "Vec::",
                "vec!",
                "Box::",
                ".collect(",
                concat!("while ", "let Ok"),
                concat!("un", "safe"),
                concat!("Mut", "ex"),
                concat!("Rw", "Lock"),
                concat!(".lo", "ck("),
                concat!("try", "_lock"),
            ] {
                assert!(!body.contains(token), "{name} {token}");
            }
        }
    }

    fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
        let start = source.find(name).unwrap();
        let open = source[start..].find('{').unwrap() + start;
        let mut depth = 0usize;
        for (offset, byte) in source[open..].bytes().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &source[open..=open + offset];
                    }
                }
                _ => {}
            }
        }
        panic!("{name}");
    }

    fn drain_latest_patch(pending: &mut Consumer<PatchBank>) -> Option<PatchBank> {
        let mut latest = None;
        while let Ok(bank) = pending.pop() {
            latest = Some(bank);
        }
        latest
    }

    fn metered_patch(rate: SampleRate) -> (CompiledPatch, MeterRoute) {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(AudioSample::new(0.5).unwrap()));
        let lowpass = patch.insert(Module::Filter {
            input: AudioSample::ZERO,
            cutoff: Hertz::new(1000.0).unwrap(),
            resonance: Resonance::new(0.707).unwrap(),
        });
        let port = patch.input_port(lowpass, InputKind::In).unwrap();
        patch
            .connect_input(patch.output_port(source, 0).unwrap(), port)
            .unwrap();
        patch
            .output(patch.output_port(lowpass, 0).unwrap())
            .unwrap();
        (
            CompiledPatch::new(&patch, rate).unwrap(),
            MeterRoute::new(lowpass, telemetry_module_id(), 1).unwrap(),
        )
    }

    fn test_handle() -> AudioHandle {
        let (patch_pending, _patch_callback) = RingBuffer::new(2);
        let (_patch_retired_callback, patch_retired) = RingBuffer::new(2);
        let (track_pending, _track_callback) = RingBuffer::new(2);
        let (_track_retired_callback, track_retired) = RingBuffer::new(2);
        let (play_pending, _play_callback) = RingBuffer::new(2);
        AudioHandle {
            patch_pending,
            patch_retired,
            track_pending,
            track_retired,
            sequence_waiting: None,
            play_pending,
            probe_bus: Arc::new(ProbeBus::new()),
            meter_bus: Arc::new(MeterBus::new()),
            rate: SampleRate::new(44_100).unwrap(),
            patch_generation: 0,
        }
    }

    fn telemetry_module_id() -> ModuleId {
        let mut state = GuiState::new(4, 4);
        state.apply(GuiAction::OpenPalette);
        state.apply(GuiAction::Confirm);
        state.modules()[0].id()
    }

    fn compiled_saw_patch(frequency: f32, rate: SampleRate) -> CompiledPatch {
        let mut patch = Patch::new();
        let freq = patch.insert(Module::Freq);
        let osc = patch.insert(brainwash::preset::saw(Hertz::new(frequency).unwrap()));
        let port = patch.input_port(osc, InputKind::Freq).unwrap();
        patch
            .connect_input(patch.output_port(freq, 0).unwrap(), port)
            .unwrap();
        patch.output(patch.output_port(osc, 0).unwrap()).unwrap();
        CompiledPatch::new(&patch, rate).unwrap()
    }

    fn constant_patch(value: f32, rate: SampleRate) -> CompiledPatch {
        let mut patch = Patch::new();
        let source = patch.insert(Module::Constant(AudioSample::new(value).unwrap()));
        patch.output(patch.output_port(source, 0).unwrap()).unwrap();
        CompiledPatch::new(&patch, rate).unwrap()
    }

    fn unmodulated_osc_patch(rate: SampleRate) -> CompiledPatch {
        let mut patch = Patch::new();
        let osc = patch.insert(brainwash::preset::saw(Hertz::new(440.0).unwrap()));
        patch.output(patch.output_port(osc, 0).unwrap()).unwrap();
        CompiledPatch::new(&patch, rate).unwrap()
    }

    fn gated_osc_patch(rate: SampleRate) -> CompiledPatch {
        let mut patch = Patch::new();
        let gate = patch.insert(Module::Gate);
        let osc = patch.insert(brainwash::preset::saw(Hertz::new(440.0).unwrap()));
        let output = patch.insert(Module::Binary {
            op: BinaryOp::Multiply,
            a: AudioSample::ZERO,
            b: AudioSample::ZERO,
        });
        patch
            .connect_input(
                patch.output_port(osc, 0).unwrap(),
                patch.input_port(output, InputKind::A).unwrap(),
            )
            .unwrap();
        patch
            .connect_input(
                patch.output_port(gate, 0).unwrap(),
                patch.input_port(output, InputKind::B).unwrap(),
            )
            .unwrap();
        patch.output(patch.output_port(output, 0).unwrap()).unwrap();
        CompiledPatch::new(&patch, rate).unwrap()
    }
}
