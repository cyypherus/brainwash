pub struct CombFilter {
    buffer: Vec<f32>,
    buffer_size: usize,
    buffer_index: usize,
    feedback: f32,
    filterstore: f32,
    damp1: f32,
    damp2: f32,
}

impl CombFilter {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            buffer_size: size,
            buffer_index: 0,
            feedback: 0.0,
            filterstore: 0.0,
            damp1: 0.0,
            damp2: 1.0,
        }
    }

    pub fn output(&mut self, input: f32) -> f32 {
        let mut output = self.buffer[self.buffer_index];
        undenormalise(&mut output);

        self.filterstore = (output * self.damp2) + (self.filterstore * self.damp1);
        undenormalise(&mut self.filterstore);

        self.buffer[self.buffer_index] = input + (self.filterstore * self.feedback);

        self.buffer_index += 1;
        if self.buffer_index >= self.buffer_size {
            self.buffer_index = 0;
        }

        output
    }

    pub fn feedback(&mut self, feedback: f32) -> &mut Self {
        self.feedback = feedback;
        self
    }

    pub fn damp(&mut self, damp: f32) -> &mut Self {
        self.damp1 = damp;
        self.damp2 = 1.0 - damp;
        self
    }

    pub fn copy_state_from(&mut self, other: &CombFilter) {
        let copy_len = self.buffer_size.min(other.buffer_size);
        for i in 0..copy_len {
            self.buffer[i] = other.buffer[i];
        }
        self.buffer_index = other.buffer_index % self.buffer_size;
        self.filterstore = other.filterstore;
    }
}

fn undenormalise(sample: &mut f32) {
    const DENORMAL_THRESHOLD: f32 = 1e-15;
    if sample.abs() < DENORMAL_THRESHOLD {
        *sample = 0.0;
    }
}
