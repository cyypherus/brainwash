use crate::{Signal, Time, Vol};

pub struct EnvelopePoint(pub Vol, pub Time);

pub struct ADSR {
    attack: EnvelopePoint,
    decay: EnvelopePoint,
    sustain: EnvelopePoint,
    release: EnvelopePoint,
    current_value: f32,
}

pub fn adsr(
    attack: (Vol, Time),
    decay: (Vol, Time),
    sustain: (Vol, Time),
    release: (Vol, Time),
) -> ADSR {
    ADSR {
        attack: EnvelopePoint(attack.0, attack.1),
        decay: EnvelopePoint(decay.0, decay.1),
        sustain: EnvelopePoint(sustain.0, sustain.1),
        release: EnvelopePoint(release.0, release.1),
        current_value: 0.0,
    }
}

impl ADSR {
    pub fn output(&self) -> f32 {
        self.attack.0.0
    }

    pub fn apply(&mut self, signal: &mut Signal, sample_index: usize) -> f32 {
        let sample_rate = signal.sample_rate as f32;
        let time = sample_index as f32 / sample_rate;

        let attack_time = self.attack.1.0;
        let decay_time = self.decay.1.0;
        let sustain_time = self.sustain.1.0;
        let release_time = self.release.1.0;

        let attack_vol = self.attack.0.0;
        let decay_vol = self.decay.0.0;
        let sustain_vol = self.sustain.0.0;
        let release_vol = self.release.0.0;

        let total_time = attack_time + decay_time + sustain_time + release_time;

        if time < attack_time {
            self.current_value = attack_vol * (time / attack_time);
        } else if time < (attack_time + decay_time) {
            let decay_phase = (time - attack_time) / decay_time;
            self.current_value = attack_vol + (decay_vol - attack_vol) * decay_phase;
        } else if time < (attack_time + decay_time + sustain_time) {
            self.current_value = sustain_vol;
        } else if time < total_time {
            let release_phase = (time - attack_time - decay_time - sustain_time) / release_time;
            self.current_value = sustain_vol + (release_vol - sustain_vol) * release_phase;
        } else {
            self.current_value = 0.0;
        }

        self.current_value
    }
}
