use crate::Signal;

#[derive(Clone)]
pub struct ADSRState {
    on: bool,
    time: u32,
}

pub fn adsr(id: usize, a: f32, d: f32, s: f32, r: f32) -> ADSR {
    ADSR { id, a, d, s, r }
}

pub struct ADSR {
    id: usize,
    a: f32,
    d: f32,
    s: f32,
    r: f32,
}

impl ADSR {
    pub fn new(id: usize, a: f32, d: f32, s: f32, r: f32) -> Self {
        ADSR { id, a, d, s, r }
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

        fn lerp(a: f32, b: f32, t: f32) -> f32 {
            a + (b - a) * t
        }

        let attack_time = time_to_samples(self.a, signal.sample_rate);
        let decay_time = time_to_samples(self.s, signal.sample_rate);
        let release_time = time_to_samples(self.r, signal.sample_rate);

        if *current_on {
            if duration < attack_time {
                lerp(0., 1., duration as f32 / attack_time as f32)
            } else if duration <= attack_time + decay_time {
                lerp(
                    1.,
                    self.d,
                    (duration - attack_time) as f32 / decay_time as f32,
                )
            } else {
                self.d
            }
        } else {
            if duration <= release_time {
                lerp(self.d, 0., duration as f32 / release_time as f32)
            } else {
                0.
            }
        }
    }
}
