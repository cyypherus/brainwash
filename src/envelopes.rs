use crate::{Signal, Time, Vol};

pub struct AttackStage {
    pub vol: Vol,
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
    start_position: Option<usize>,
}

pub fn a(vol: Vol, time: Time) -> AttackStage {
    AttackStage { vol, time }
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
        start_position: None,
    }
}

impl ADSR {
    pub fn output(&mut self, signal: &Signal) -> f32 {
        if self.start_position.is_none() {
            self.start_position = Some(signal.position);
        }

        let start_pos = self.start_position.unwrap();
        let elapsed_samples = signal.position - start_pos;
        let elapsed_time = elapsed_samples as f32 / signal.sample_rate as f32;

        let attack_time = self.attack.time.0;
        let decay_time = self.decay_sustain.time.0;
        let total_ad_time = attack_time + decay_time;

        let attack_vol = self.attack.vol.0;
        let sustain_vol = self.decay_sustain.vol.0;

        if elapsed_time < attack_time {
            // Attack phase: 0 -> attack_vol
            attack_vol * (elapsed_time / attack_time)
        } else if elapsed_time < total_ad_time {
            // Decay phase: attack_vol -> sustain_vol
            let decay_progress = (elapsed_time - attack_time) / decay_time;
            attack_vol + (sustain_vol - attack_vol) * decay_progress
        } else {
            // Sustain phase: hold sustain_vol
            sustain_vol
        }
    }

    pub fn reset(&mut self) {
        self.start_position = None;
    }
}
