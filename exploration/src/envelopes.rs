use crate::instrument::PressState;

#[derive(Clone, Debug)]
pub struct Adsr {
    attack_samples: usize,
    decay_samples: usize,
    sustain: f32,
    release_samples: usize,
}

impl Default for Adsr {
    fn default() -> Self {
        Adsr {
            attack_samples: 4410, // 0.1s at 44100Hz
            decay_samples: 8820,  // 0.2s at 44100Hz
            sustain: 0.7,
            release_samples: 13230, // 0.3s at 44100Hz
        }
    }
}

impl Adsr {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attack_ms(mut self, ms: f32) -> Self {
        self.attack_samples = ((ms / 1000.0) * 44100.0) as usize;
        self
    }

    pub fn decay_ms(mut self, ms: f32) -> Self {
        self.decay_samples = ((ms / 1000.0) * 44100.0) as usize;
        self
    }

    pub fn sustain(mut self, level: f32) -> Self {
        self.sustain = level.clamp(0.0, 1.0);
        self
    }

    pub fn release_ms(mut self, ms: f32) -> Self {
        self.release_samples = ((ms / 1000.0) * 44100.0) as usize;
        self
    }

    pub fn output(&self, press_state: PressState) -> f32 {
        match press_state {
            PressState::Pressed { time_in_state, .. } => {
                if time_in_state < self.attack_samples {
                    time_in_state as f32 / self.attack_samples as f32
                } else if time_in_state < self.attack_samples + self.decay_samples {
                    let decay_time = time_in_state - self.attack_samples;
                    let progress = decay_time as f32 / self.decay_samples as f32;
                    1.0 + (self.sustain - 1.0) * progress
                } else {
                    self.sustain
                }
            }
            PressState::Released {
                pressed_at,
                released_at,
                time_in_state,
            } => {
                if time_in_state >= self.release_samples {
                    0.0
                } else {
                    let start_value = {
                        let time_at_release = released_at.saturating_sub(pressed_at);
                        if time_at_release < self.attack_samples {
                            time_at_release as f32 / self.attack_samples as f32
                        } else if time_at_release < self.attack_samples + self.decay_samples {
                            let decay_time = time_at_release - self.attack_samples;
                            let progress = decay_time as f32 / self.decay_samples as f32;
                            1.0 + (self.sustain - 1.0) * progress
                        } else {
                            self.sustain
                        }
                    };
                    let progress = time_in_state as f32 / self.release_samples as f32;
                    start_value * (1.0 - progress)
                }
            }
            PressState::Idle => 0.0,
        }
    }
}

pub fn pluck() -> Adsr {
    Adsr::new()
        .attack_ms(5.0)
        .decay_ms(500.0)
        .sustain(0.0)
        .release_ms(200.0)
}

pub fn stab() -> Adsr {
    Adsr::new()
        .attack_ms(50.0)
        .decay_ms(200.0)
        .sustain(0.5)
        .release_ms(300.0)
}

pub fn lead() -> Adsr {
    Adsr::new()
        .attack_ms(50.0)
        .decay_ms(300.0)
        .sustain(0.7)
        .release_ms(400.0)
}

pub fn pad() -> Adsr {
    Adsr::new()
        .attack_ms(500.0)
        .decay_ms(500.0)
        .sustain(0.7)
        .release_ms(1000.0)
}

pub fn custom(attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32) -> Adsr {
    Adsr::new()
        .attack_ms(attack_ms)
        .decay_ms(decay_ms)
        .sustain(sustain)
        .release_ms(release_ms)
}

pub fn adsr(envelope: Adsr, press_state: PressState) -> f32 {
    envelope.output(press_state)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::instrument::PressState;

//     #[test]
//     fn test_adsr_released_note_fades_to_zero() {
//         let adsr = Adsr {
//             attack_samples: 1000,
//             decay_samples: 1000,
//             sustain: 0.7,
//             release_samples: 1000,
//         };

//         // Note pressed at sample 0
//         let pressed_at = 0;
//         let released_at = 2500; // released after attack + decay + some sustain

//         // At release time, envelope should be sustain level
//         let env_at_release = adsr.compute(PressState::Released {
//             pressed_at,
//             released_at,
//             time_in_state: 0,
//         });
//         assert!(
//             (env_at_release - 0.7).abs() < 0.01,
//             "At release, should be at sustain level"
//         );

//         // Halfway through release
//         let env_halfway = adsr.compute(PressState::Released {
//             pressed_at,
//             released_at,
//             time_in_state: 500,
//         });
//         assert!(
//             env_halfway < env_at_release && env_halfway > 0.0,
//             "Halfway through release should fade between sustain and 0"
//         );

//         // At end of release
//         let env_at_end = adsr.compute(PressState::Released {
//             pressed_at,
//             released_at,
//             time_in_state: 1000,
//         });
//         assert!(
//             env_at_end < 0.01,
//             "After release time, should be at or near 0"
//         );

//         // Past release time
//         let env_past = adsr.compute(PressState::Released {
//             pressed_at,
//             released_at,
//             time_in_state: 2000,
//         });
//         assert_eq!(env_past, 0.0, "Well past release time should be exactly 0");
//     }
// }
