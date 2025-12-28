use crate::SAMPLE_RATE;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct Lowpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    pub frequency: f32,
    pub q: f32,
}

impl Lowpass {
    pub fn new() -> Self {
        let mut filter = Lowpass {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            frequency: 0.5,
            q: 0.707,
        };
        filter.update_coefficients(0.5, 0.707, 44100.0);
        filter
    }

    pub fn freq(mut self, freq: f32) -> Self {
        self.frequency = freq;
        self
    }

    pub fn q(mut self, q: f32) -> Self {
        self.q = q;
        self
    }

    pub fn build(self) -> Filter {
        Filter::Lowpass(self)
    }

    pub fn run(&mut self, input: f32) -> f32 {
        self.update_coefficients(self.frequency, self.q, SAMPLE_RATE);

        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    fn update_coefficients(&mut self, frequency: f32, q: f32, sample_rate: f32) {
        let frequency_hz = frequency * (sample_rate / 2.0);
        let omega = 2.0 * PI * frequency_hz / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = (1.0 - cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Highpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    pub frequency: f32,
    pub q: f32,
}

impl Highpass {
    pub fn new() -> Self {
        let mut filter = Highpass {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            frequency: 0.2,
            q: 0.707,
        };
        filter.update_coefficients(0.2, 0.707, 44100.0);
        filter
    }

    pub fn freq(mut self, freq: f32) -> Self {
        self.frequency = freq;
        self
    }

    pub fn q(mut self, q: f32) -> Self {
        self.q = q;
        self
    }

    pub fn build(self) -> Filter {
        Filter::Highpass(self)
    }

    pub fn run(&mut self, input: f32) -> f32 {
        self.update_coefficients(self.frequency, self.q, SAMPLE_RATE);

        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    fn update_coefficients(&mut self, frequency: f32, q: f32, sample_rate: f32) {
        let frequency_hz = frequency * (sample_rate / 2.0);
        let omega = 2.0 * PI * frequency_hz / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 + cos_omega) / 2.0;
        let b1 = -(1.0 + cos_omega);
        let b2 = (1.0 + cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
}

#[derive(Clone, Debug)]
pub struct Allpass {
    pub(crate) buffer: Vec<f32>,
    buffer_index: usize,
    pub(crate) feedback: f32,
}

impl Allpass {
    pub fn new() -> Self {
        Allpass {
            buffer: vec![0.0; 1024],
            buffer_index: 0,
            feedback: 0.5,
        }
    }

    pub fn size(mut self, size: usize) -> Self {
        self.buffer = vec![0.0; size];
        self.buffer_index = 0;
        self
    }

    pub fn set_feedback(&mut self, feedback: f32) -> &mut Self {
        self.feedback = feedback;
        self
    }

    pub fn build(self) -> Filter {
        Filter::Allpass(self)
    }

    pub fn run(&mut self, input: f32) -> f32 {
        let buffer_out = self.buffer[self.buffer_index];
        let output = -input + buffer_out;
        self.buffer[self.buffer_index] = input + (buffer_out * self.feedback);

        self.buffer_index += 1;
        if self.buffer_index >= self.buffer.len() {
            self.buffer_index = 0;
        }

        output
    }
}

#[derive(Clone, Debug)]
pub struct Comb {
    pub(crate) buffer: Vec<f32>,
    buffer_index: usize,
    pub(crate) feedback: f32,
    filterstore: f32,
    pub(crate) damp1: f32,
    pub(crate) damp2: f32,
}

impl Comb {
    pub fn new() -> Self {
        Comb {
            buffer: vec![0.0; 1024],
            buffer_index: 0,
            feedback: 0.84,
            filterstore: 0.0,
            damp1: 0.2,
            damp2: 0.8,
        }
    }

    pub fn size(mut self, size: usize) -> Self {
        self.buffer = vec![0.0; size];
        self.buffer_index = 0;
        self
    }

    pub fn feedback(mut self, feedback: f32) -> Self {
        self.feedback = feedback;
        self
    }

    pub fn damp(mut self, damp: f32) -> Self {
        self.damp1 = damp;
        self.damp2 = 1.0 - damp;
        self
    }

    pub fn build(self) -> Filter {
        Filter::Comb(self)
    }

    pub fn run(&mut self, input: f32) -> f32 {
        let mut output = self.buffer[self.buffer_index];
        undenormalise(&mut output);

        self.filterstore = (output * self.damp2) + (self.filterstore * self.damp1);
        undenormalise(&mut self.filterstore);

        self.buffer[self.buffer_index] = input + (self.filterstore * self.feedback);

        self.buffer_index += 1;
        if self.buffer_index >= self.buffer.len() {
            self.buffer_index = 0;
        }

        output
    }
}

fn undenormalise(sample: &mut f32) {
    const DENORMAL_THRESHOLD: f32 = 1e-15;
    if sample.abs() < DENORMAL_THRESHOLD {
        *sample = 0.0;
    }
}

#[derive(Clone, Debug)]
pub enum Filter {
    Lowpass(Lowpass),
    Highpass(Highpass),
    Allpass(Allpass),
    Comb(Comb),
}

impl Filter {
    pub fn process(&mut self, input: f32) -> f32 {
        match self {
            Filter::Lowpass(f) => f.run(input),
            Filter::Highpass(f) => f.run(input),
            Filter::Allpass(f) => f.run(input),
            Filter::Comb(f) => f.run(input),
        }
    }
}

pub fn lowpass() -> Lowpass {
    Lowpass::new()
}

pub fn highpass() -> Highpass {
    Highpass::new()
}

pub fn allpass() -> Allpass {
    Allpass::new()
}

pub fn comb() -> Comb {
    Comb::new()
}
