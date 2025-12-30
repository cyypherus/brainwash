use crate::{KeyState, Signal};

#[derive(Clone)]
pub struct ADSR {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    attack_curve: f32,
    decay_curve: f32,
    release_curve: f32,
    retrigger_start_value: f32,
}

impl Default for ADSR {
    fn default() -> Self {
        ADSR {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            attack_curve: 0.0,
            decay_curve: 0.0,
            release_curve: 0.0,
            retrigger_start_value: 0.0,
        }
    }
}

impl ADSR {
    pub fn att(&mut self, time: f32) -> &mut Self {
        self.attack = time.max(0.001);
        self
    }

    pub fn dec(&mut self, time: f32) -> &mut Self {
        self.decay = time.max(0.001);
        self
    }

    pub fn sus(&mut self, level: f32) -> &mut Self {
        self.sustain = level.clamp(0.0, 1.0);
        self
    }

    pub fn rel(&mut self, time: f32) -> &mut Self {
        self.release = time.max(0.001);
        self
    }

    pub fn att_curve(&mut self, curve: f32) -> &mut Self {
        self.attack_curve = curve.clamp(-1.0, 1.0);
        self
    }

    pub fn dec_curve(&mut self, curve: f32) -> &mut Self {
        self.decay_curve = curve.clamp(-1.0, 1.0);
        self
    }

    pub fn rel_curve(&mut self, curve: f32) -> &mut Self {
        self.release_curve = curve.clamp(-1.0, 1.0);
        self
    }

    pub fn output(&self, key_state: KeyState, signal: &Signal) -> f32 {
        let current_time = signal.position;
        let sample_rate = signal.sample_rate as f32;

        match key_state {
            KeyState::Idle => 0.0,
            KeyState::Pressed { pressed_at } => {
                let elapsed = (current_time - pressed_at) as f32 / sample_rate;
                self.calculate_envelope_value(elapsed)
            }
            KeyState::Released {
                pressed_at,
                released_at,
            } => {
                let trigger_elapsed = (released_at - pressed_at) as f32 / sample_rate;
                let release_elapsed = (current_time - released_at) as f32 / sample_rate;

                if release_elapsed >= self.release {
                    0.0
                } else {
                    let release_start_value = self.calculate_envelope_value(trigger_elapsed);
                    let release_progress = release_elapsed / self.release;
                    let curved_progress = self.apply_curve(release_progress, self.release_curve);
                    release_start_value * (1.0 - curved_progress)
                }
            }
        }
    }

    fn calculate_envelope_value(&self, elapsed: f32) -> f32 {
        if elapsed < self.attack {
            let t = elapsed / self.attack;
            let curved_t = self.apply_curve(t, self.attack_curve);
            self.retrigger_start_value + (1.0 - self.retrigger_start_value) * curved_t
        } else if elapsed < self.attack + self.decay {
            let decay_progress = (elapsed - self.attack) / self.decay;
            let curved_progress = self.apply_curve(decay_progress, self.decay_curve);
            1.0 + (self.sustain - 1.0) * curved_progress
        } else {
            self.sustain
        }
    }

    fn apply_curve(&self, t: f32, curve: f32) -> f32 {
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

    pub fn pluck(&mut self) -> &mut Self {
        self.att(0.005).dec(0.5).sus(0.0).rel(0.2)
    }

    pub fn stab(&mut self) -> &mut Self {
        self.att(0.001).dec(0.1).sus(0.0).rel(0.3)
    }

    pub fn lead(&mut self) -> &mut Self {
        self.att(0.05).dec(0.3).sus(0.7).rel(0.4)
    }

    pub fn pad(&mut self) -> &mut Self {
        self.att(0.5).dec(0.5).sus(0.7).rel(1.0)
    }
}
