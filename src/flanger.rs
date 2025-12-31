use crate::{oscillators::Osc, signal::Signal};

const MIN_DELAY_SAMPLES: usize = 2;
const MAX_DELAY_SAMPLES: usize = 800;
const BUFFER_SIZE: usize = MAX_DELAY_SAMPLES + 2;
const DEFAULT_RATE: f32 = 0.5;
const DEFAULT_DEPTH: f32 = 0.5;
const DEFAULT_FEEDBACK: f32 = 0.3;

pub struct Flanger {
    buffer: Vec<f32>,
    buffer_size: usize,
    write_index: usize,
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
            buffer: vec![0.0; BUFFER_SIZE],
            buffer_size: BUFFER_SIZE,
            write_index: 0,
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
        let delay_frac = delay_samples.fract();
        let delay_int = delay_samples as usize;

        let read_index = (self.write_index + self.buffer_size - delay_int) % self.buffer_size;
        let next_index = (read_index + 1) % self.buffer_size;

        let delayed_sample = self.buffer[read_index];
        let next_sample = self.buffer[next_index];
        let interpolated = delayed_sample + (next_sample - delayed_sample) * delay_frac;

        self.buffer[self.write_index] = input + delayed_sample * self.feedback;

        self.write_index += 1;
        if self.write_index >= self.buffer_size {
            self.write_index = 0;
        }

        input + interpolated
    }

    fn update_delay_range(&mut self) {
        self.min_delay_samples = MIN_DELAY_SAMPLES as f32;
        self.max_delay_samples =
            MIN_DELAY_SAMPLES as f32 + (MAX_DELAY_SAMPLES - MIN_DELAY_SAMPLES) as f32 * self.depth;
    }
}
