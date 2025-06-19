use crate::Signal;

#[derive(Clone)]
pub struct ADSRState {
    on: bool,
    time: u32,
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
        self.attack = time;
        self
    }

    pub fn dec(mut self, time: f32) -> Self {
        self.decay = time;
        self
    }

    pub fn sus(mut self, level: f32) -> Self {
        self.sustain = level;
        self
    }

    pub fn rel(mut self, time: f32) -> Self {
        self.release = time;
        self
    }

    pub fn att_curve(mut self, curve: f32) -> Self {
        self.attack_curve = curve;
        self
    }

    pub fn dec_curve(mut self, curve: f32) -> Self {
        self.decay_curve = curve;
        self
    }

    pub fn rel_curve(mut self, curve: f32) -> Self {
        self.release_curve = curve;
        self
    }

    pub fn output(&self, on: bool, note: i32, signal: &mut Signal) -> f32 {
        let state = signal
            .adsr_state
            .entry(self.id + note as usize)
            .or_insert(ADSRState { on: false, time: 0 });

        let ADSRState {
            on: current_on,
            time,
        } = state;

        let duration = signal.position as u32 - *time;

        if on != *current_on {
            *current_on = on;
            *time = signal.position as u32;
        }

        fn time_to_samples(secs: f32, sample_rate: usize) -> u32 {
            (secs * sample_rate as f32) as u32
        }

        fn curve_lerp(a: f32, b: f32, t: f32, curve: f32) -> f32 {
            let curved_t = if curve == 0.0 {
                t
            } else {
                let exp_curve = curve * 5.0;
                if exp_curve > 0.0 {
                    (exp_curve * t).exp() / exp_curve.exp()
                } else {
                    1.0 - ((-exp_curve) * (1.0 - t)).exp() / ((-exp_curve).exp())
                }
            };
            a + (b - a) * curved_t
        }

        let attack_time = time_to_samples(self.attack, signal.sample_rate);
        let decay_time = time_to_samples(self.decay, signal.sample_rate);
        let release_time = time_to_samples(self.release, signal.sample_rate);

        if *current_on {
            if duration < attack_time {
                curve_lerp(
                    0.0,
                    1.0,
                    duration as f32 / attack_time as f32,
                    self.attack_curve,
                )
            } else if duration <= attack_time + decay_time {
                curve_lerp(
                    1.0,
                    self.sustain,
                    (duration - attack_time) as f32 / decay_time as f32,
                    self.decay_curve,
                )
            } else {
                self.sustain
            }
        } else {
            if duration <= release_time {
                curve_lerp(
                    self.sustain,
                    0.0,
                    duration as f32 / release_time as f32,
                    self.release_curve,
                )
            } else {
                0.0
            }
        }
    }
}
