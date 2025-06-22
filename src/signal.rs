use crate::oscillators::OscillatorState;
use crate::ramp::RampState;
use crate::{ADSRState, SequenceState};
use std::collections::HashMap;

#[cfg(feature = "assert")]
use std::collections::HashSet;

pub struct Signal {
    pub current_sample: f32,
    pub sample_rate: usize,
    pub position: usize,
    pub global_volume: f32,
    adsr_state: HashMap<(i32, i32), ADSRState>,
    sequence_state: HashMap<(i32, i32), SequenceState>,
    oscillator_state: HashMap<(i32, i32), OscillatorState>,
    ramp_state: HashMap<(i32, i32), RampState>,
    #[cfg(feature = "assert")]
    accesses: HashSet<(Access, i32, i32)>,
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
            oscillator_state: HashMap::new(),
            ramp_state: HashMap::new(),
            #[cfg(feature = "assert")]
            accesses: HashSet::new(),
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
        #[cfg(feature = "assert")]
        self.accesses.clear();
    }

    pub fn reset(&mut self) {
        self.position = 0;
        self.current_sample = 0.0;
        self.adsr_state.clear();
        self.sequence_state.clear();
        self.oscillator_state.clear();
        self.ramp_state.clear();
        #[cfg(feature = "assert")]
        self.accesses.clear();
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
        #[cfg(feature = "assert")]
        self.assert_unique_access(Access::Adsr, id, index, "ADSR");
        self.adsr_state.entry((id, index)).or_insert(ADSRState {
            trigger_time: None,
            release_time: None,
        })
    }

    pub(crate) fn get_sequence_state(&mut self, id: i32, index: i32) -> &mut SequenceState {
        #[cfg(feature = "assert")]
        self.assert_unique_access(Access::Sequence, id, index, "Sequence");
        self.sequence_state
            .entry((id, index))
            .or_insert(SequenceState {
                all_notes: Vec::new(),
                last_chord_index: usize::MAX,
                active_notes: std::collections::HashSet::new(),
                previous_notes: std::collections::HashSet::new(),
                current_bar: 0,
                last_clock_position: 0.0,
            })
    }

    pub(crate) fn get_oscillator_state(&mut self, id: i32, index: i32) -> &mut OscillatorState {
        #[cfg(feature = "assert")]
        self.assert_unique_access(Access::Oscillator, id, index, "Oscillator");
        self.oscillator_state
            .entry((id, index))
            .or_insert(OscillatorState {
                phase_accumulator: 0,
            })
    }

    pub(crate) fn get_ramp_state(&mut self, id: i32, index: i32) -> &mut RampState {
        #[cfg(feature = "assert")]
        self.assert_unique_access(Access::Ramp, id, index, "Ramp");
        self.ramp_state.entry((id, index)).or_insert(RampState {
            current_value: 0.0,
            target_value: 0.0,
            start_value: 0.0,
            start_time: None,
        })
    }

    #[cfg(feature = "assert")]
    fn assert_unique_access(&mut self, access_type: Access, id: i32, index: i32, type_name: &str) {
        assert!(
            self.accesses.insert((access_type, id, index)),
            r#"
            *********************************************
            Error: A module was used twice in one sample.
            If you use a module in a loop make sure it has a unique index by calling `module.index(key.index)`
            Module Info: type: {}, id: {}, index: {}
            *********************************************
            "#,
            type_name,
            id,
            index
        );
    }
}
#[cfg(feature = "assert")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Access {
    Adsr,
    Oscillator,
    Ramp,
    Sequence,
}
