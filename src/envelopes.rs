use crate::{KeyState, Signal};

#[derive(Clone, Debug, Copy)]
pub enum PointType {
    Linear,
    Curve,
}

#[derive(Clone, Debug)]
pub struct EnvelopePoint {
    pub time: f32,
    pub value: f32,
    pub point_type: PointType,
}

pub fn point(time: f32, value: f32) -> EnvelopePoint {
    EnvelopePoint {
        time,
        value,
        point_type: PointType::Linear,
    }
}

pub fn curve(time: f32, value: f32) -> EnvelopePoint {
    EnvelopePoint {
        time,
        value,
        point_type: PointType::Curve,
    }
}

#[derive(Clone, Debug)]
pub struct Envelope {
    points: Vec<EnvelopePoint>,
}

impl Envelope {
    pub fn new(points: Vec<EnvelopePoint>) -> Self {
        let mut sorted_points = points;
        sorted_points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        for point in &sorted_points {
            assert!(
                (0.0..=1.0).contains(&point.time),
                "Envelope point time must be between 0.0 and 1.0"
            );
            assert!(
                (0.0..=1.0).contains(&point.value),
                "Envelope point value must be between 0.0 and 1.0"
            );
        }

        Envelope {
            points: sorted_points,
        }
    }

    pub fn output(&self, time: f32) -> f32 {
        let time = time.clamp(0.0, 1.0);

        if self.points.is_empty() {
            return 0.0;
        }

        if self.points.len() == 1 {
            return self.points[0].value;
        }

        if time <= self.points[0].time {
            return self.points[0].value;
        }

        if time >= self.points[self.points.len() - 1].time {
            return self.points[self.points.len() - 1].value;
        }

        for i in 0..self.points.len() - 1 {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];

            if time >= p1.time && time <= p2.time {
                let segment_duration = p2.time - p1.time;
                if segment_duration < 1e-6 {
                    return p1.value;
                }

                let t = (time - p1.time) / segment_duration;

                return match (p1.point_type, p2.point_type) {
                    (PointType::Linear, PointType::Linear) => p1.value + (p2.value - p1.value) * t,
                    _ => {
                        let p0_val = if i > 0 {
                            self.points[i - 1].value
                        } else {
                            p1.value - (p2.value - p1.value) * 0.5
                        };

                        let p3_val = if i + 2 < self.points.len() {
                            self.points[i + 2].value
                        } else {
                            p2.value + (p2.value - p1.value) * 0.5
                        };

                        self.catmull_rom(p0_val, p1.value, p2.value, p3_val, t)
                            .clamp(0.0, 1.0)
                    }
                };
            }
        }

        self.points[self.points.len() - 1].value
    }

    fn catmull_rom(&self, p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;

        0.5 * ((2.0 * p1)
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope::new(vec![point(0.0, 0.0), point(1.0, 1.0)])
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_single_point() {
        let env = Envelope::new(vec![point(0.5, 0.7)]);
        assert_eq!(env.output(0.0), 0.7);
        assert_eq!(env.output(0.5), 0.7);
        assert_eq!(env.output(1.0), 0.7);
    }

    #[test]
    fn test_envelope_linear_interpolation() {
        let env = Envelope::new(vec![point(0.0, 0.0), point(1.0, 1.0)]);
        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.25), 0.25);
        assert_eq!(env.output(0.5), 0.5);
        assert_eq!(env.output(0.75), 0.75);
        assert_eq!(env.output(1.0), 1.0);
    }

    #[test]
    fn test_envelope_multiple_segments_linear() {
        let env = Envelope::new(vec![
            point(0.0, 0.0),
            point(0.2, 1.0),
            point(0.5, 0.7),
            point(1.0, 0.0),
        ]);

        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.1), 0.5);
        assert_eq!(env.output(0.2), 1.0);

        let mid = env.output(0.35);
        assert!((mid - 0.85).abs() < 0.01);

        assert_eq!(env.output(1.0), 0.0);
    }

    #[test]
    fn test_envelope_clamp_time() {
        let env = Envelope::new(vec![point(0.0, 0.2), point(1.0, 0.8)]);

        assert_eq!(env.output(-0.5), 0.2);
        assert_eq!(env.output(1.5), 0.8);
    }

    #[test]
    fn test_envelope_unsorted_points() {
        let env = Envelope::new(vec![point(0.5, 0.5), point(0.0, 0.0), point(1.0, 1.0)]);

        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.5), 0.5);
        assert_eq!(env.output(1.0), 1.0);
    }

    #[test]
    fn test_envelope_attack_decay_linear() {
        let env = Envelope::new(vec![
            point(0.0, 0.0),
            point(0.1, 1.0),
            point(0.4, 0.7),
            point(1.0, 0.7),
        ]);

        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.05), 0.5);
        assert_eq!(env.output(0.1), 1.0);

        let decay_mid = env.output(0.25);
        assert!((decay_mid - 0.85).abs() < 0.01);

        assert_eq!(env.output(0.7), 0.7);
        assert_eq!(env.output(1.0), 0.7);
    }

    #[test]
    fn test_envelope_curve_interpolation() {
        let env = Envelope::new(vec![
            curve(0.0, 0.0),
            curve(0.25, 1.0),
            curve(0.75, 0.5),
            curve(1.0, 0.0),
        ]);

        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.25), 1.0);
        assert_eq!(env.output(0.75), 0.5);
        assert_eq!(env.output(1.0), 0.0);

        let curve_val = env.output(0.5);
        assert!(curve_val > 0.5 && curve_val < 1.0);
    }

    #[test]
    fn test_envelope_mixed_linear_curve() {
        let env = Envelope::new(vec![point(0.0, 0.0), curve(0.5, 1.0), point(1.0, 0.0)]);

        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.5), 1.0);
        assert_eq!(env.output(1.0), 0.0);

        let before_peak = env.output(0.25);
        assert!(before_peak > 0.0 && before_peak < 1.0);
    }

    #[test]
    fn test_envelope_curve_smooth_attack() {
        let env = Envelope::new(vec![curve(0.0, 0.0), curve(0.1, 1.0), curve(1.0, 1.0)]);

        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.1), 1.0);

        let early = env.output(0.05);
        assert!(early > 0.0 && early < 1.0);
    }

    #[test]
    fn test_envelope_empty() {
        let env = Envelope::new(vec![]);
        assert_eq!(env.output(0.0), 0.0);
        assert_eq!(env.output(0.5), 0.0);
        assert_eq!(env.output(1.0), 0.0);
    }

    #[test]
    #[should_panic(expected = "Envelope point time must be between 0.0 and 1.0")]
    fn test_envelope_invalid_time_low() {
        Envelope::new(vec![point(-0.1, 0.5)]);
    }

    #[test]
    #[should_panic(expected = "Envelope point time must be between 0.0 and 1.0")]
    fn test_envelope_invalid_time_high() {
        Envelope::new(vec![point(1.1, 0.5)]);
    }

    #[test]
    #[should_panic(expected = "Envelope point value must be between 0.0 and 1.0")]
    fn test_envelope_invalid_value_low() {
        Envelope::new(vec![point(0.5, -0.1)]);
    }

    #[test]
    #[should_panic(expected = "Envelope point value must be between 0.0 and 1.0")]
    fn test_envelope_invalid_value_high() {
        Envelope::new(vec![point(0.5, 1.1)]);
    }
}
