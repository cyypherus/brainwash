const DEFAULT_MAX_DELAY_SAMPLES: usize = 44100;

pub struct Delay {
    buffer: Vec<f32>,
    buffer_size: usize,
    write_index: usize,
    delay_samples: f32,
}

impl Default for Delay {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_DELAY_SAMPLES)
    }
}

impl Delay {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples + 2],
            buffer_size: max_delay_samples + 2,
            write_index: 0,
            delay_samples: 0.0,
        }
    }

    pub fn delay(&mut self, delay_samples: f32) -> &mut Self {
        self.delay_samples = delay_samples.clamp(0.0, (self.buffer_size - 2) as f32);
        self
    }

    pub fn tap(&self) -> f32 {
        self.read(self.delay_samples)
    }

    pub fn output(&mut self, input: f32) -> f32 {
        let output = self.tap();
        self.write(input);
        output
    }

    pub(crate) fn read(&self, delay_samples: f32) -> f32 {
        let delay_int = delay_samples as usize;
        let delay_frac = delay_samples.fract();

        let read_index = (self.write_index + self.buffer_size - delay_int) % self.buffer_size;
        let next_index = (read_index + 1) % self.buffer_size;

        let delayed_sample = self.buffer[read_index];
        let next_sample = self.buffer[next_index];

        delayed_sample + (next_sample - delayed_sample) * delay_frac
    }

    pub(crate) fn read_uninterpolated(&self, delay_samples: usize) -> f32 {
        let read_index = (self.write_index + self.buffer_size - delay_samples) % self.buffer_size;
        self.buffer[read_index]
    }

    pub(crate) fn write(&mut self, sample: f32) {
        self.buffer[self.write_index] = sample;
        self.write_index += 1;
        if self.write_index >= self.buffer_size {
            self.write_index = 0;
        }
    }

    pub fn copy_state_from(&mut self, other: &Delay) {
        let copy_len = self.buffer_size.min(other.buffer_size);
        for i in 0..copy_len {
            self.buffer[i] = other.buffer[i];
        }
        self.write_index = other.write_index % self.buffer_size;
    }
}
