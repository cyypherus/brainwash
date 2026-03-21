pub struct Compressor {
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    makeup: f32,
    envelope: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            ratio: 4.0,
            attack: 0.01,
            release: 0.1,
            makeup: 1.0,
            envelope: 0.0,
        }
    }
}

impl Compressor {
    pub fn threshold(&mut self, t: f32) -> &mut Self {
        self.threshold = t.clamp(0.0, 1.0);
        self
    }

    pub fn ratio(&mut self, r: f32) -> &mut Self {
        self.ratio = r.clamp(1.0, 20.0);
        self
    }

    pub fn attack(&mut self, seconds: f32) -> &mut Self {
        self.attack = seconds.clamp(0.0001, 1.0);
        self
    }

    pub fn release(&mut self, seconds: f32) -> &mut Self {
        self.release = seconds.clamp(0.001, 2.0);
        self
    }

    pub fn makeup(&mut self, g: f32) -> &mut Self {
        self.makeup = g.clamp(0.0, 4.0);
        self
    }

    pub fn output(&mut self, input: f32, sample_rate: f32) -> f32 {
        let level = input.abs();

        let coeff = if level > self.envelope {
            (-1.0 / (self.attack * sample_rate)).exp()
        } else {
            (-1.0 / (self.release * sample_rate)).exp()
        };
        self.envelope = coeff * self.envelope + (1.0 - coeff) * level;

        let gain = if self.envelope > self.threshold {
            let over_db = 20.0 * (self.envelope / self.threshold).log10();
            let compressed_db = over_db / self.ratio;
            let target_db = 20.0 * self.threshold.log10() + compressed_db;
            let target_level = 10.0f32.powf(target_db / 20.0);
            target_level / self.envelope.max(0.0001)
        } else {
            1.0
        };

        input * gain * self.makeup
    }

    pub fn copy_state_from(&mut self, other: &Compressor) {
        self.envelope = other.envelope;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_below_threshold() {
        let mut comp = Compressor::default();
        comp.threshold(0.5);

        for _ in 0..1000 {
            let out = comp.output(0.1, 44100.0);
            assert!(out.abs() < 0.5);
        }
    }

    #[test]
    fn test_compressor_reduces_loud_signals() {
        let mut comp = Compressor::default();
        comp.threshold(0.3).ratio(4.0).attack(0.001).makeup(1.0);

        for _ in 0..4410 {
            comp.output(0.9, 44100.0);
        }

        let out = comp.output(0.9, 44100.0);
        assert!(out.abs() < 0.9, "output {} should be less than input", out);
    }

    #[test]
    fn test_compressor_bounded_output() {
        let mut comp = Compressor::default();
        comp.threshold(0.2).ratio(10.0).makeup(2.0);

        for _ in 0..10000 {
            let out = comp.output(1.0, 44100.0);
            assert!(out.is_finite());
            assert!(out.abs() < 10.0);
        }
    }
}
