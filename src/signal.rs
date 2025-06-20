use crate::oscillators::OscillatorState;
use crate::{ADSRState, ClockState, SequenceState};
use std::collections::HashMap;

pub struct Signal {
    pub current_sample: f32,
    pub sample_rate: usize,
    pub position: usize,
    pub global_volume: f32,
    adsr_state: HashMap<(i32, i32), ADSRState>,
    sequence_state: HashMap<(i32, i32), SequenceState>,
    clock_state: HashMap<(i32, i32), ClockState>,
    oscillator_state: HashMap<(i32, i32), OscillatorState>,
}

impl Signal {
    pub fn new(sample_rate: usize) -> Self {
        Signal {
            current_sample: 0.0,
            sample_rate,
            position: 0,
            global_volume: 1.0,
            adsr_state: HashMap::new(),
            sequence_state: HashMap::new(),
            clock_state: HashMap::new(),
            oscillator_state: HashMap::new(),
        }
    }

    pub fn add_sample(&mut self, sample: f32) {
        let val = sample * self.global_volume;
        self.current_sample += val;
        if self.current_sample > 1. {
            self.current_sample = 0.;
        }
    }

    pub fn advance(&mut self) {
        self.position += 1;
        self.current_sample = 0.0;
    }

    pub fn reset(&mut self) {
        self.position = 0;
        self.current_sample = 0.0;
        self.adsr_state.clear();
        self.sequence_state.clear();
        self.oscillator_state.clear();
    }

    pub fn get_current_sample(&self) -> f32 {
        self.current_sample
    }

    pub fn set_global_volume(&mut self, volume: f32) {
        self.global_volume = volume.max(0.0);
    }

    pub fn get_global_volume(&self) -> f32 {
        self.global_volume
    }

    pub fn get_time_seconds(&self) -> f32 {
        self.position as f32 / self.sample_rate as f32
    }

    pub fn get_adsr_state(&mut self, id: i32, index: i32) -> &mut ADSRState {
        self.adsr_state.entry((id, index)).or_insert(ADSRState {
            trigger_time: None,
            release_time: None,
        })
    }

    pub(crate) fn get_sequence_state(&mut self, id: i32, index: i32) -> &mut SequenceState {
        self.sequence_state
            .entry((id, index))
            .or_insert(SequenceState {
                all_notes: Vec::new(),
                last_chord_index: usize::MAX,
                active_notes: std::collections::HashSet::new(),
                previous_notes: std::collections::HashSet::new(),
                params_hash: 0,
                current_bar: 0,
                last_clock_position: 0.0,
            })
    }

    pub(crate) fn get_clock_state(&mut self, id: i32, index: i32) -> &mut ClockState {
        self.clock_state
            .entry((id, index))
            .or_insert(ClockState { position: 0 })
    }

    pub(crate) fn get_oscillator_state(&mut self, id: i32, index: i32) -> &mut OscillatorState {
        self.oscillator_state
            .entry((id, index))
            .or_insert(OscillatorState {
                phase_accumulator: 0,
            })
    }
}
