#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum DistortionType {
    #[default]
    Tube,
    Tape,
    Fuzz,
    Fold,
    Clip,
}

struct AllpassSection {
    a: f64,
    x1: f64,
    y1: f64,
}

impl AllpassSection {
    fn new(coefficient: f64) -> Self {
        Self {
            a: coefficient,
            x1: 0.0,
            y1: 0.0,
        }
    }

    fn process(&mut self, input: f64) -> f64 {
        let y = self.x1 + (input - self.y1) * self.a;
        self.x1 = input;
        self.y1 = y;
        y
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

struct HalfbandFilter {
    even: Vec<AllpassSection>,
    odd: Vec<AllpassSection>,
    prev_sample: f64,
}

impl HalfbandFilter {
    fn new(coeffs: &[f64]) -> Self {
        let even: Vec<_> = coeffs
            .iter()
            .step_by(2)
            .map(|&c| AllpassSection::new(c))
            .collect();
        let odd: Vec<_> = coeffs
            .iter()
            .skip(1)
            .step_by(2)
            .map(|&c| AllpassSection::new(c))
            .collect();
        Self {
            even,
            odd,
            prev_sample: 0.0,
        }
    }

    fn upsample(&mut self, input: f64) -> (f64, f64) {
        let mut even_out = input;
        for section in &mut self.even {
            even_out = section.process(even_out);
        }

        let mut odd_out = self.prev_sample;
        for section in &mut self.odd {
            odd_out = section.process(odd_out);
        }
        self.prev_sample = input;

        ((even_out + odd_out) * 0.5, (even_out - odd_out) * 0.5)
    }

    fn downsample(&mut self, in0: f64, in1: f64) -> f64 {
        let mut even_out = in0;
        for section in &mut self.even {
            even_out = section.process(even_out);
        }

        let mut odd_out = in1;
        for section in &mut self.odd {
            odd_out = section.process(odd_out);
        }

        (even_out + odd_out) * 0.5
    }

    fn reset(&mut self) {
        for s in &mut self.even {
            s.reset();
        }
        for s in &mut self.odd {
            s.reset();
        }
        self.prev_sample = 0.0;
    }
}

const HIIR_8: [f64; 4] = [
    0.07711507983241622,
    0.22823651466538192,
    0.421_978_042_944_980_2,
    0.690_592_758_682_615_8,
];

pub struct Distortion {
    drive: f32,
    asymmetry: f32,
    dist_type: DistortionType,
    up_filter: HalfbandFilter,
    down_filter: HalfbandFilter,
    dc_blocker_x1: f32,
    dc_blocker_y1: f32,
}

impl Default for Distortion {
    fn default() -> Self {
        Self {
            drive: 1.0,
            asymmetry: 0.0,
            dist_type: DistortionType::Tube,
            up_filter: HalfbandFilter::new(&HIIR_8),
            down_filter: HalfbandFilter::new(&HIIR_8),
            dc_blocker_x1: 0.0,
            dc_blocker_y1: 0.0,
        }
    }
}

#[inline]
fn fast_tanh(x: f32) -> f32 {
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

#[inline]
fn soft_clip_cubic(x: f32) -> f32 {
    if x <= -1.0 {
        -2.0 / 3.0
    } else if x >= 1.0 {
        2.0 / 3.0
    } else {
        x - x * x * x / 3.0
    }
}

impl Distortion {
    pub fn drive(&mut self, amount: f32) -> &mut Self {
        self.drive = amount.clamp(0.1, 100.0);
        self
    }

    pub fn asymmetry(&mut self, amount: f32) -> &mut Self {
        self.asymmetry = amount.clamp(-1.0, 1.0);
        self
    }

    pub fn dist_type(&mut self, t: DistortionType) -> &mut Self {
        self.dist_type = t;
        self
    }

    fn waveshape(&self, x: f32) -> f32 {
        match self.dist_type {
            DistortionType::Tube => self.tube_shape(x),
            DistortionType::Tape => self.tape_shape(x),
            DistortionType::Fuzz => self.fuzz_shape(x),
            DistortionType::Fold => self.fold_shape(x),
            DistortionType::Clip => self.clip_shape(x),
        }
    }

    fn tube_shape(&self, x: f32) -> f32 {
        let driven = x * self.drive;
        let shifted = driven + self.asymmetry * 0.3;
        let saturated = fast_tanh(shifted);
        saturated - fast_tanh(self.asymmetry * 0.3)
    }

    fn tape_shape(&self, x: f32) -> f32 {
        let driven = x * self.drive;
        let soft = soft_clip_cubic(driven * 0.8);
        let compressed = soft * (1.0 - 0.2 * soft.abs());
        compressed + self.asymmetry * 0.1 * driven * driven.abs()
    }

    fn fuzz_shape(&self, x: f32) -> f32 {
        let driven = x * self.drive * 2.0;
        let clipped = driven.clamp(-1.0, 1.0);
        let fuzzed = clipped.signum() * (1.0 - (-clipped.abs() * 4.0).exp());
        fuzzed + self.asymmetry * 0.2 * clipped * clipped
    }

    fn fold_shape(&self, x: f32) -> f32 {
        let driven = x * self.drive;
        let threshold = 1.0;
        if driven > threshold || driven < -threshold {
            ((driven - threshold).abs() % (threshold * 4.0) - threshold * 2.0).abs() - threshold
        } else {
            driven
        }
    }

    fn clip_shape(&self, x: f32) -> f32 {
        let driven = x * self.drive;
        let knee = 0.7 - self.asymmetry * 0.2;
        if driven.abs() < knee {
            driven
        } else {
            let sign = driven.signum();
            let excess = driven.abs() - knee;
            let range = 1.0 - knee;
            let compressed = knee + range * (1.0 - (-excess / range).exp());
            sign * compressed.min(1.0)
        }
    }

    fn dc_block(&mut self, x: f32) -> f32 {
        let r = 0.995;
        let y = x - self.dc_blocker_x1 + r * self.dc_blocker_y1;
        self.dc_blocker_x1 = x;
        self.dc_blocker_y1 = y;
        y
    }

    pub fn output(&mut self, input: f32) -> f32 {
        let input64 = input as f64;

        let (a, b) = self.up_filter.upsample(input64);

        let y0 = self.waveshape(a as f32);
        let y1 = self.waveshape(b as f32);

        let downsampled = self.down_filter.downsample(y0 as f64, y1 as f64) as f32;

        self.dc_block(downsampled) * 0.7
    }

    pub fn reset(&mut self) {
        self.up_filter.reset();
        self.down_filter.reset();
        self.dc_blocker_x1 = 0.0;
        self.dc_blocker_y1 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_tanh_approximation() {
        for i in -30..=30 {
            let x = i as f32 * 0.1;
            let approx = fast_tanh(x);
            let actual = x.tanh();
            assert!(
                (approx - actual).abs() < 0.05,
                "fast_tanh({}) = {}, expected {}",
                x,
                approx,
                actual
            );
        }
    }

    #[test]
    fn test_distortion_output_bounded() {
        let mut dist = Distortion::default();
        dist.drive(10.0);

        for i in -100..=100 {
            let input = i as f32 * 0.01;
            let output = dist.output(input);
            assert!(
                output.abs() < 2.0,
                "output {} for input {} is too large",
                output,
                input
            );
        }
    }

    #[test]
    fn test_all_distortion_types() {
        let types = [
            DistortionType::Tube,
            DistortionType::Tape,
            DistortionType::Fuzz,
            DistortionType::Fold,
            DistortionType::Clip,
        ];

        for dt in types {
            let mut dist = Distortion::default();
            dist.dist_type(dt).drive(5.0);

            let output = dist.output(0.5);
            assert!(output.is_finite(), "{:?} produced non-finite output", dt);
        }
    }

    #[test]
    fn test_dc_blocking() {
        let mut dist = Distortion::default();
        dist.asymmetry(0.5).drive(2.0);

        let mut sum = 0.0;
        for _ in 0..1000 {
            sum += dist.output(0.3);
        }

        let avg = sum / 1000.0;
        assert!(avg.abs() < 0.5, "DC offset too high: {}", avg);
    }
}
