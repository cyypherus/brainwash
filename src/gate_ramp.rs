use crate::Signal;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GateRampMode {
    #[default]
    Rise,
    Fall,
}

#[derive(Clone)]
pub struct GateRamp {
    mode: GateRampMode,
    time: f32,
    value: f32,
    last_gate: bool,
    active: bool,
    start_value: f32,
    start_time: Option<usize>,
}

impl Default for GateRamp {
    fn default() -> Self {
        Self {
            mode: GateRampMode::Rise,
            time: 0.1,
            value: 0.0,
            last_gate: false,
            active: false,
            start_value: 0.0,
            start_time: None,
        }
    }
}

impl GateRamp {
    pub fn mode(&mut self, mode: GateRampMode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn rise(&mut self) -> &mut Self {
        self.mode = GateRampMode::Rise;
        self
    }

    pub fn fall(&mut self) -> &mut Self {
        self.mode = GateRampMode::Fall;
        self
    }

    pub fn time(&mut self, seconds: f32) -> &mut Self {
        self.time = seconds.max(0.001);
        self
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.last_gate = false;
        self.active = false;
        self.start_value = 0.0;
        self.start_time = None;
    }

    pub fn output(&mut self, gate: f32, signal: &Signal) -> f32 {
        let pressed = gate > 0.5;
        let current_time = signal.position;
        let sample_rate = signal.sample_rate as f32;

        let active = match self.mode {
            GateRampMode::Rise => pressed,
            GateRampMode::Fall => !pressed,
        };

        if !active {
            self.last_gate = pressed;
            return 0.0;
        }

        let trigger = match self.mode {
            GateRampMode::Rise => pressed && !self.last_gate,
            GateRampMode::Fall => !pressed && self.last_gate,
        };

        if trigger {
            self.value = 0.0;
            self.start_time = Some(current_time);
        }
        self.last_gate = pressed;

        if let Some(start_time) = self.start_time {
            let elapsed = (current_time - start_time) as f32 / sample_rate;

            if elapsed >= self.time {
                self.value = 1.0;
            } else {
                self.value = elapsed / self.time;
            }
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signal(sample_rate: usize, position: usize) -> Signal {
        let mut s = Signal::new(sample_rate);
        s.position = position;
        s
    }

    #[test]
    fn test_gate_ramp_rise() {
        let mut ramp = GateRamp::default();
        ramp.rise().time(0.1);

        let sr = 1000;

        let v = ramp.output(0.0, &make_signal(sr, 0));
        assert!((v - 0.0).abs() < 0.01);

        ramp.output(1.0, &make_signal(sr, 1));

        let v = ramp.output(1.0, &make_signal(sr, 51));
        assert!((v - 0.5).abs() < 0.05);

        let v = ramp.output(1.0, &make_signal(sr, 101));
        assert!((v - 1.0).abs() < 0.01);

        let v = ramp.output(0.0, &make_signal(sr, 102));
        assert!((v - 0.0).abs() < 0.01);

        let v = ramp.output(0.0, &make_signal(sr, 200));
        assert!((v - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_gate_ramp_fall() {
        let mut ramp = GateRamp::default();
        ramp.fall().time(0.1);

        let sr = 1000;

        let v = ramp.output(0.0, &make_signal(sr, 0));
        assert!((v - 0.0).abs() < 0.01);

        ramp.output(1.0, &make_signal(sr, 1));
        let v = ramp.output(1.0, &make_signal(sr, 50));
        assert!((v - 0.0).abs() < 0.01);

        ramp.output(0.0, &make_signal(sr, 51));

        let v = ramp.output(0.0, &make_signal(sr, 101));
        assert!((v - 0.5).abs() < 0.1);

        let v = ramp.output(0.0, &make_signal(sr, 151));
        assert!((v - 1.0).abs() < 0.01);

        let v = ramp.output(1.0, &make_signal(sr, 152));
        assert!((v - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_gate_ramp_rise_retrigger() {
        let mut ramp = GateRamp::default();
        ramp.rise().time(0.1);

        let sr = 1000;

        ramp.output(1.0, &make_signal(sr, 0));
        let v = ramp.output(1.0, &make_signal(sr, 50));
        assert!((v - 0.5).abs() < 0.1);

        let v = ramp.output(0.0, &make_signal(sr, 51));
        assert!((v - 0.0).abs() < 0.1);

        ramp.output(1.0, &make_signal(sr, 61));
        let v = ramp.output(1.0, &make_signal(sr, 111));
        assert!((v - 0.5).abs() < 0.1);

        let v = ramp.output(1.0, &make_signal(sr, 161));
        assert!((v - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_gate_ramp_stays_at_one_while_held() {
        let mut ramp = GateRamp::default();
        ramp.rise().time(0.1);

        let sr = 1000;

        ramp.output(1.0, &make_signal(sr, 0));
        ramp.output(1.0, &make_signal(sr, 200));

        let v = ramp.output(1.0, &make_signal(sr, 500));
        assert!((v - 1.0).abs() < 0.01);

        let v = ramp.output(0.0, &make_signal(sr, 501));
        assert!((v - 0.0).abs() < 0.01);
    }
}
