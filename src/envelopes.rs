use crate::Signal;

pub struct ADSRState {
    phase: ADSRPhase,
    phase_position: u32,
    last_value: f32,
    release_start_value: f32,
    lookup_table: Option<ADSRLookupTable>,
    params_hash: u64,
}

#[derive(Clone, Debug, PartialEq)]
enum ADSRPhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

struct ADSRLookupTable {
    attack_table: Vec<f32>,
    decay_table: Vec<f32>,
    release_table: Vec<f32>,
}

impl ADSRLookupTable {
    fn new(
        attack_samples: usize,
        decay_samples: usize,
        release_samples: usize,
        sustain_level: f32,
        attack_curve: f32,
        decay_curve: f32,
        release_curve: f32,
    ) -> Self {
        let mut table = Self {
            attack_table: Vec::with_capacity(attack_samples),
            decay_table: Vec::with_capacity(decay_samples),
            release_table: Vec::with_capacity(release_samples),
        };

        for i in 0..attack_samples {
            let t = i as f32 / (attack_samples - 1).max(1) as f32;
            let curved_t = apply_curve(t, attack_curve);
            table.attack_table.push(curved_t);
        }

        for i in 0..decay_samples {
            let t = i as f32 / (decay_samples - 1).max(1) as f32;
            let curved_t = apply_curve(t, decay_curve);
            let value = 1.0 + (sustain_level - 1.0) * curved_t;
            table.decay_table.push(value);
        }

        for i in 0..release_samples {
            let t = i as f32 / (release_samples - 1).max(1) as f32;
            let curved_t = apply_curve(t, release_curve);
            table.release_table.push(1.0 - curved_t);
        }

        table
    }

    fn get_attack_value(&self, position: usize) -> f32 {
        if self.attack_table.is_empty() {
            return 1.0;
        }
        let idx = position.min(self.attack_table.len() - 1);
        self.attack_table[idx]
    }

    fn get_decay_value(&self, position: usize) -> f32 {
        if self.decay_table.is_empty() {
            return self.decay_table.get(0).copied().unwrap_or(1.0);
        }
        let idx = position.min(self.decay_table.len() - 1);
        self.decay_table[idx]
    }

    fn get_release_value(&self, position: usize, start_value: f32) -> f32 {
        if self.release_table.is_empty() {
            return 0.0;
        }
        let idx = position.min(self.release_table.len() - 1);
        let normalized = self.release_table[idx];
        start_value * normalized
    }
}

fn apply_curve(t: f32, curve: f32) -> f32 {
    if curve.abs() < 0.001 {
        return t;
    }

    let exp_curve = curve * 3.0;
    if exp_curve > 0.0 {
        (exp_curve * t).exp() / exp_curve.exp()
    } else {
        1.0 - ((-exp_curve) * (1.0 - t)).exp() / ((-exp_curve).exp())
    }
}

pub fn adsr(id: usize) -> ADSR {
    ADSR {
        id,
        attack: 0.01,
        decay: 0.1,
        sustain: 0.7,
        release: 0.3,
        attack_curve: 0.0,
        decay_curve: 0.0,
        release_curve: 0.0,
    }
}

pub struct ADSR {
    id: usize,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    attack_curve: f32,
    decay_curve: f32,
    release_curve: f32,
}

impl ADSR {
    pub fn att(mut self, time: f32) -> Self {
        self.attack = time.max(0.001);
        self
    }

    pub fn dec(mut self, time: f32) -> Self {
        self.decay = time.max(0.001);
        self
    }

    pub fn sus(mut self, level: f32) -> Self {
        self.sustain = level.clamp(0.0, 1.0);
        self
    }

    pub fn rel(mut self, time: f32) -> Self {
        self.release = time.max(0.001);
        self
    }

    pub fn att_curve(mut self, curve: f32) -> Self {
        self.attack_curve = curve.clamp(-1.0, 1.0);
        self
    }

    pub fn dec_curve(mut self, curve: f32) -> Self {
        self.decay_curve = curve.clamp(-1.0, 1.0);
        self
    }

    pub fn rel_curve(mut self, curve: f32) -> Self {
        self.release_curve = curve.clamp(-1.0, 1.0);
        self
    }

    fn hash_params(&self, sample_rate: usize) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        ((self.attack * 1000.0) as u32).hash(&mut hasher);
        ((self.decay * 1000.0) as u32).hash(&mut hasher);
        ((self.sustain * 1000.0) as u32).hash(&mut hasher);
        ((self.release * 1000.0) as u32).hash(&mut hasher);
        ((self.attack_curve * 1000.0) as i32).hash(&mut hasher);
        ((self.decay_curve * 1000.0) as i32).hash(&mut hasher);
        ((self.release_curve * 1000.0) as i32).hash(&mut hasher);
        sample_rate.hash(&mut hasher);
        hasher.finish()
    }

    fn ensure_lookup_table(&self, state: &mut ADSRState, sample_rate: usize) {
        let current_hash = self.hash_params(sample_rate);

        if state.lookup_table.is_none() || state.params_hash != current_hash {
            let attack_samples = (self.attack * sample_rate as f32) as usize;
            let decay_samples = (self.decay * sample_rate as f32) as usize;
            let release_samples = (self.release * sample_rate as f32) as usize;

            state.lookup_table = Some(ADSRLookupTable::new(
                attack_samples.max(1),
                decay_samples.max(1),
                release_samples.max(1),
                self.sustain,
                self.attack_curve,
                self.decay_curve,
                self.release_curve,
            ));
            state.params_hash = current_hash;
        }
    }

    pub fn output(&self, on: bool, note: i32, signal: &mut Signal) -> f32 {
        let state = signal
            .adsr_state
            .entry(self.id as i32 + note)
            .or_insert(ADSRState {
                phase: ADSRPhase::Idle,
                phase_position: 0,
                last_value: 0.0,
                release_start_value: 0.0,
                lookup_table: None,
                params_hash: 0,
            });

        self.ensure_lookup_table(state, signal.sample_rate);
        let lookup_table = state.lookup_table.as_ref().unwrap();

        match (state.phase.clone(), on) {
            (ADSRPhase::Idle, true) => {
                state.phase = ADSRPhase::Attack;
                state.phase_position = 0;
            }
            (ADSRPhase::Attack, false)
            | (ADSRPhase::Decay, false)
            | (ADSRPhase::Sustain, false) => {
                state.phase = ADSRPhase::Release;
                state.phase_position = 0;
                state.release_start_value = state.last_value;
            }
            (ADSRPhase::Release, true) => {
                state.phase = ADSRPhase::Attack;
                state.phase_position = 0;
            }
            _ => {}
        }

        let value = match state.phase {
            ADSRPhase::Idle => 0.0,
            ADSRPhase::Attack => {
                let value = lookup_table.get_attack_value(state.phase_position as usize);
                state.phase_position += 1;

                if state.phase_position >= lookup_table.attack_table.len() as u32 {
                    state.phase = ADSRPhase::Decay;
                    state.phase_position = 0;
                }
                value
            }
            ADSRPhase::Decay => {
                let value = lookup_table.get_decay_value(state.phase_position as usize);
                state.phase_position += 1;

                if state.phase_position >= lookup_table.decay_table.len() as u32 {
                    state.phase = ADSRPhase::Sustain;
                    state.phase_position = 0;
                }
                value
            }
            ADSRPhase::Sustain => self.sustain,
            ADSRPhase::Release => {
                let value = lookup_table
                    .get_release_value(state.phase_position as usize, state.release_start_value);
                state.phase_position += 1;

                if state.phase_position >= lookup_table.release_table.len() as u32 {
                    state.phase = ADSRPhase::Idle;
                    state.phase_position = 0;
                }
                value
            }
        };

        state.last_value = value;
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_signal() -> Signal {
        Signal {
            current_sample: 0.0,
            sample_rate: 44100,
            position: 0,
            adsr_state: HashMap::new(),
        }
    }

    #[test]
    fn test_adsr_idle_state() {
        let envelope = adsr(0);
        let mut signal = create_test_signal();

        let value = envelope.output(false, 0, &mut signal);
        assert_eq!(value, 0.0, "ADSR should start at 0.0 when off");
    }

    #[test]
    fn test_adsr_attack_phase() {
        let envelope = adsr(0).att(0.1);
        let mut signal = create_test_signal();

        let mut values = Vec::new();
        for _ in 0..4410 {
            let value = envelope.output(true, 0, &mut signal);
            values.push(value);
            signal.position += 1;
        }

        assert!(
            values[0] < values[values.len() - 1],
            "Attack should increase over time"
        );
        assert!(
            values[values.len() - 1] > 0.9,
            "Attack should reach near 1.0"
        );
    }

    #[test]
    fn test_adsr_decay_phase() {
        let envelope = adsr(0).att(0.01).dec(0.1).sus(0.5);
        let mut signal = create_test_signal();

        for _ in 0..441 {
            envelope.output(true, 0, &mut signal);
            signal.position += 1;
        }

        let mut decay_values = Vec::new();
        for _ in 0..1000 {
            let value = envelope.output(true, 0, &mut signal);
            decay_values.push(value);
            signal.position += 1;
        }

        assert!(
            decay_values[0] > decay_values[decay_values.len() - 1],
            "Decay should decrease over time"
        );
    }

    #[test]
    fn test_adsr_sustain_phase() {
        let envelope = adsr(0).att(0.01).dec(0.01).sus(0.7);
        let mut signal = create_test_signal();

        for _ in 0..2000 {
            envelope.output(true, 0, &mut signal);
            signal.position += 1;
        }

        let sustain_value = envelope.output(true, 0, &mut signal);
        assert!(
            (sustain_value - 0.7).abs() < 0.1,
            "Sustain should maintain level near 0.7"
        );
    }

    #[test]
    fn test_adsr_release_phase() {
        let envelope = adsr(0).att(0.01).dec(0.01).sus(0.8).rel(0.1);
        let mut signal = create_test_signal();

        for _ in 0..2000 {
            envelope.output(true, 0, &mut signal);
            signal.position += 1;
        }

        let mut release_values = Vec::new();
        for _ in 0..4410 {
            let value = envelope.output(false, 0, &mut signal);
            release_values.push(value);
            signal.position += 1;
        }

        assert!(
            release_values[0] > release_values[release_values.len() - 1],
            "Release should decrease over time"
        );
        assert!(
            release_values[release_values.len() - 1] < 0.1,
            "Release should reach near 0.0"
        );
    }

    #[test]
    fn test_adsr_no_spikes() {
        let envelope = adsr(0).att(0.01).dec(0.05).sus(0.6).rel(0.08);
        let mut signal = create_test_signal();

        let mut all_values = Vec::new();

        for _ in 0..2000 {
            let value = envelope.output(true, 0, &mut signal);
            all_values.push(value);
            signal.position += 1;
        }

        for _ in 0..3528 {
            let value = envelope.output(false, 0, &mut signal);
            all_values.push(value);
            signal.position += 1;
        }

        for i in 1..all_values.len() {
            let diff = (all_values[i] - all_values[i - 1]).abs();
            assert!(
                diff < 0.1,
                "No spike should exceed 0.1 difference between samples at index {}",
                i
            );
        }
    }

    #[test]
    fn test_adsr_retrigger() {
        let envelope = adsr(0).att(0.02).dec(0.05).sus(0.5).rel(0.1);
        let mut signal = create_test_signal();

        for _ in 0..1000 {
            envelope.output(true, 0, &mut signal);
            signal.position += 1;
        }

        for _ in 0..500 {
            envelope.output(false, 0, &mut signal);
            signal.position += 1;
        }

        let retrigger_value = envelope.output(true, 0, &mut signal);
        assert!(retrigger_value < 0.5, "Retrigger should start fresh attack");
    }

    #[test]
    fn test_curve_application() {
        let linear_curve = apply_curve(0.5, 0.0);
        assert!(
            (linear_curve - 0.5).abs() < 0.001,
            "Linear curve should be identity"
        );

        let exp_curve = apply_curve(0.5, 0.5);
        assert!(
            exp_curve != 0.5,
            "Exponential curve should modify the value"
        );

        let log_curve = apply_curve(0.5, -0.5);
        assert!(
            log_curve != 0.5,
            "Logarithmic curve should modify the value"
        );
    }
}
