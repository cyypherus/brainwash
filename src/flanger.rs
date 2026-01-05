use crate::{delay::Delay, oscillators::Osc, signal::Signal};

const MIN_DELAY_SAMPLES: usize = 2;
const MAX_DELAY_SAMPLES: usize = 800;
const DEFAULT_RATE: f32 = 0.5;
const DEFAULT_DEPTH: f32 = 0.5;
const DEFAULT_FEEDBACK: f32 = 0.3;

pub struct Flanger {
    delay: Delay,
    lfo: Osc,
    rate: f32,
    depth: f32,
    feedback: f32,
    min_delay_samples: f32,
    max_delay_samples: f32,
}

impl Default for Flanger {
    fn default() -> Self {
        let mut lfo = Osc::default();
        lfo.sin().freq(DEFAULT_RATE).unipolar();

        let mut flanger = Self {
            delay: Delay::new(MAX_DELAY_SAMPLES),
            lfo,
            rate: DEFAULT_RATE,
            depth: DEFAULT_DEPTH,
            feedback: DEFAULT_FEEDBACK,
            min_delay_samples: MIN_DELAY_SAMPLES as f32,
            max_delay_samples: MAX_DELAY_SAMPLES as f32,
        };
        flanger.update_delay_range();
        flanger
    }
}

impl Flanger {
    pub fn freq(&mut self, rate: f32) -> &mut Self {
        self.rate = rate.clamp(0.1, 10.0);
        self.lfo.freq(self.rate);
        self
    }

    pub fn depth(&mut self, depth: f32) -> &mut Self {
        self.depth = depth.clamp(0.0, 1.0);
        self.update_delay_range();
        self
    }

    pub fn min_delay(&mut self, samples: f32) -> &mut Self {
        self.min_delay_samples = samples.max(MIN_DELAY_SAMPLES as f32);
        self
    }

    pub fn max_delay(&mut self, samples: f32) -> &mut Self {
        self.max_delay_samples = samples.clamp(self.min_delay_samples, MAX_DELAY_SAMPLES as f32);
        self
    }

    pub fn feedback(&mut self, feedback: f32) -> &mut Self {
        self.feedback = feedback.clamp(0.0, 0.95);
        self
    }

    pub fn output(&mut self, input: f32, signal: &mut Signal) -> f32 {
        let lfo_value = self.lfo.output(signal);
        let delay_samples =
            self.min_delay_samples + (self.max_delay_samples - self.min_delay_samples) * lfo_value;

        let interpolated = self.delay.read(delay_samples);
        let delayed_sample = self.delay.read_uninterpolated(delay_samples as usize);

        self.delay.write(input + delayed_sample * self.feedback);

        input + interpolated
    }

    fn update_delay_range(&mut self) {
        self.min_delay_samples = MIN_DELAY_SAMPLES as f32;
        self.max_delay_samples =
            MIN_DELAY_SAMPLES as f32 + (MAX_DELAY_SAMPLES - MIN_DELAY_SAMPLES) as f32 * self.depth;
    }

    pub fn copy_state_from(&mut self, other: &Flanger) {
        self.delay.copy_state_from(&other.delay);
        self.lfo.copy_phase_from(&other.lfo);
    }
}
