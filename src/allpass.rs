pub struct AllpassFilter {
    buffer: Vec<f32>,
    buffer_size: usize,
    buffer_index: usize,
    feedback: f32,
}

impl AllpassFilter {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            buffer_size: size,
            buffer_index: 0,
            feedback: 0.5,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let buffer_out = self.buffer[self.buffer_index];
        let output = -input + buffer_out;
        self.buffer[self.buffer_index] = input + (buffer_out * self.feedback);

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
}
