use crate::Signal;

pub struct ADSRState {
    trigger_time: Option<usize>,
    release_time: Option<usize>,
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

    pub fn output(&self, on: bool, note: i32, signal: &mut Signal) -> f32 {
        let id = self.id as i32 + note;
        let state = signal
            .adsr_state
            .entry(self.id as i32 + note)
            .or_insert(ADSRState {
                trigger_time: None,
                release_time: None,
            });

        let current_time = signal.position;
        let sample_rate = signal.sample_rate as f32;

        match (on, state.trigger_time, state.release_time) {
            (true, None, _) => {
                state.trigger_time = Some(current_time);
                state.release_time = None;
            }
            (false, Some(_), None) => {
                state.release_time = Some(current_time);
            }
            (true, _, Some(_)) => {
                state.trigger_time = Some(current_time);
                state.release_time = None;
            }
            _ => {}
        }

        match (state.trigger_time, state.release_time) {
            (None, _) => 0.0,
            (Some(trigger), None) => {
                let elapsed = (current_time - trigger) as f32 / sample_rate;
                self.calculate_envelope_value(elapsed)
            }
            (Some(trigger), Some(release)) => {
                let trigger_elapsed = (release - trigger) as f32 / sample_rate;
                let release_elapsed = (current_time - release) as f32 / sample_rate;
                let release_start_value = self.calculate_envelope_value(trigger_elapsed);

                if release_elapsed >= self.release {
                    state.trigger_time = None;
                    state.release_time = None;
                    0.0
                } else {
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
            self.apply_curve(t, self.attack_curve)
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
}
