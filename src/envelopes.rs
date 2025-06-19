use crate::{Signal, Time, Vol};

pub struct AttackStage {
    pub time: Time,
}

pub struct DecaySustainStage {
    pub vol: Vol,
    pub time: Time,
}

pub struct ReleaseStage {
    pub time: Time,
}

pub struct ADSR {
    attack: AttackStage,
    decay_sustain: DecaySustainStage,
    release: ReleaseStage,
}

pub fn a(time: Time) -> AttackStage {
    AttackStage { time }
}

pub fn ds(vol: Vol, time: Time) -> DecaySustainStage {
    DecaySustainStage { vol, time }
}

pub fn r(time: Time) -> ReleaseStage {
    ReleaseStage { time }
}

pub fn adsr(attack: AttackStage, decay_sustain: DecaySustainStage, release: ReleaseStage) -> ADSR {
    ADSR {
        attack,
        decay_sustain,
        release,
    }
}

impl ADSR {
    pub fn output(&self, note: i32, signal: &Signal) -> f32 {
        let (on, duration) = signal.pitch_triggers[note as usize];
        let pos = signal.position as u32;

        fn time_to_samples(secs: f32) -> u32 {
            (secs * 44100.0) as u32
        }

        fn lerp(a: f32, b: f32, t: f32) -> f32 {
            a + (b - a) * t
        }

        let (note_on, note_time) = (on, duration);
        let note_duration = pos - note_time;
        let attack_time = time_to_samples(self.attack.time.0);
        let decay_time = time_to_samples(self.decay_sustain.time.0);
        let release_time = time_to_samples(self.release.time.0);

        if note_on {
            if note_duration < attack_time {
                lerp(0., 1., note_duration as f32 / attack_time as f32)
            } else if note_duration <= attack_time + decay_time {
                lerp(
                    1.,
                    self.decay_sustain.vol.0,
                    (note_duration - attack_time) as f32 / decay_time as f32,
                )
            } else {
                self.decay_sustain.vol.0
            }
        } else {
            if note_duration <= release_time {
                lerp(
                    self.decay_sustain.vol.0,
                    0.,
                    note_duration as f32 / release_time as f32,
                )
            } else {
                0.
            }
        }
    }
}
