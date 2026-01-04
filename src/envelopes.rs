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
                (-1.0..=1.0).contains(&point.value),
                "Envelope point value must be between -1.0 and 1.0"
            );
        }

        Envelope {
            points: sorted_points,
        }
    }

    pub fn points(&self) -> &[EnvelopePoint] {
        &self.points
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

                let eased_t = match (p1.point_type, p2.point_type) {
                    (PointType::Linear, PointType::Linear) => t,
                    (PointType::Curve, PointType::Linear) => {
                        // Ease out: smooth departure from p1, sharp arrival at p2
                        1.0 - (1.0 - t) * (1.0 - t)
                    }
                    (PointType::Linear, PointType::Curve) => {
                        // Ease in: sharp departure from p1, smooth arrival at p2
                        t * t
                    }
                    (PointType::Curve, PointType::Curve) => {
                        // Ease in-out: smooth on both ends
                        if t < 0.5 {
                            2.0 * t * t
                        } else {
                            1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                        }
                    }
                };
                return p1.value + (p2.value - p1.value) * eased_t;
            }
        }

        self.points[self.points.len() - 1].value
    }
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope::new(vec![point(0.0, 0.0), point(1.0, 1.0)])
    }
}

#[derive(Clone)]
pub struct ADSR {
    attack_ratio: f32,
    sustain: f32,
    release_start_value: f32,
    last_rise: f32,
    last_fall: f32,
}

impl Default for ADSR {
    fn default() -> Self {
        ADSR {
            attack_ratio: 0.5,
            sustain: 0.7,
            release_start_value: 0.0,
            last_rise: 0.0,
            last_fall: 1.0,
        }
    }
}

impl ADSR {
    pub fn att(&mut self, ratio: f32) -> &mut Self {
        self.attack_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    pub fn sus(&mut self, level: f32) -> &mut Self {
        self.sustain = level.clamp(0.0, 1.0);
        self
    }

    pub fn reset(&mut self) {
        self.release_start_value = 0.0;
        self.last_rise = 0.0;
        self.last_fall = 1.0;
    }

    pub fn output(&mut self, rise: f32, fall: f32) -> f32 {
        let rise = rise.clamp(0.0, 1.0);
        let fall = fall.clamp(0.0, 1.0);

        if fall < self.last_fall {
            self.release_start_value = self.ads_value(self.last_rise);
        }
        self.last_rise = rise;
        self.last_fall = fall;

        let ads = self.ads_value(rise);
        let release_multiplier = 1.0 - fall;

        if fall > 0.0 {
            self.release_start_value * release_multiplier
        } else {
            ads
        }
    }

    fn ads_value(&self, rise: f32) -> f32 {
        if self.attack_ratio <= 0.0 {
            self.sustain
        } else if rise < self.attack_ratio {
            rise / self.attack_ratio
        } else if self.attack_ratio >= 1.0 {
            1.0
        } else {
            let decay_progress = (rise - self.attack_ratio) / (1.0 - self.attack_ratio);
            1.0 + (self.sustain - 1.0) * decay_progress
        }
    }

    pub fn pluck(&mut self) -> &mut Self {
        self.att(0.01).sus(0.0)
    }

    pub fn stab(&mut self) -> &mut Self {
        self.att(0.01).sus(0.0)
    }

    pub fn lead(&mut self) -> &mut Self {
        self.att(0.15).sus(0.7)
    }

    pub fn pad(&mut self) -> &mut Self {
        self.att(0.5).sus(0.7)
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
    #[should_panic(expected = "Envelope point value must be between -1.0 and 1.0")]
    fn test_envelope_invalid_value_low() {
        Envelope::new(vec![point(0.5, -1.1)]);
    }

    #[test]
    #[should_panic(expected = "Envelope point value must be between -1.0 and 1.0")]
    fn test_envelope_invalid_value_high() {
        Envelope::new(vec![point(0.5, 1.1)]);
    }
}
