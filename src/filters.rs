use crate::Signal;

pub struct LowpassFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    frequency: f32,
    q: f32,
}

impl Default for LowpassFilter {
    fn default() -> Self {
        let mut filter = Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            frequency: 0.045,
            q: 0.707,
        };
        filter.update_coefficients(44100.0);
        filter
    }
}

impl LowpassFilter {
    pub fn freq(&mut self, normalized_frequency: f32) -> &mut Self {
        self.frequency = normalized_frequency.clamp(0.001, 0.99);
        self
    }

    pub fn q(&mut self, q: f32) -> &mut Self {
        self.q = q.max(0.1);
        self
    }

    fn process(&mut self, input: f32, sample_rate: f32) -> f32 {
        self.update_coefficients(sample_rate);

        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    pub fn output(&mut self, input: f32, signal: &mut Signal) -> f32 {
        self.process(input, signal.sample_rate as f32)
    }

    fn update_coefficients(&mut self, sample_rate: f32) {
        let frequency_hz = self.frequency * (sample_rate / 2.0);
        let omega = 2.0 * std::f32::consts::PI * frequency_hz / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * self.q);

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

pub struct HighpassFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    frequency: f32,
    q: f32,
}

impl Default for HighpassFilter {
    fn default() -> Self {
        let mut filter = Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            frequency: 0.045,
            q: 0.707,
        };
        filter.update_coefficients(44100.0);
        filter
    }
}

impl HighpassFilter {
    pub fn freq(&mut self, normalized_frequency: f32) -> &mut Self {
        self.frequency = normalized_frequency.clamp(0.001, 0.99);
        self
    }

    pub fn q(&mut self, q: f32) -> &mut Self {
        self.q = q.max(0.1);
        self
    }

    fn process(&mut self, input: f32, sample_rate: f32) -> f32 {
        self.update_coefficients(sample_rate);

        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    pub fn output(&mut self, input: f32, signal: &mut Signal) -> f32 {
        self.process(input, signal.sample_rate as f32)
    }

    fn update_coefficients(&mut self, sample_rate: f32) {
        let frequency_hz = self.frequency * (sample_rate / 2.0);
        let omega = 2.0 * std::f32::consts::PI * frequency_hz / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * self.q);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_sine_wave(frequency: f32, sample_rate: f32, duration_samples: usize) -> Vec<f32> {
        let mut samples = Vec::with_capacity(duration_samples);
        for i in 0..duration_samples {
            let t = i as f32 / sample_rate;
            let sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
            samples.push(sample);
        }
        samples
    }

    fn calculate_rms(samples: &[f32]) -> f32 {
        let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    #[test]
    fn test_lowpass_filter_attenuates_high_frequencies() {
        let sample_rate = 44100.0;
        let duration_samples = 4410;
        let low_freq = 200.0;
        let high_freq = 5000.0;
        let cutoff_normalized = 1000.0 / (sample_rate / 2.0);

        let mut filter = LowpassFilter::default();
        filter.freq(cutoff_normalized).q(0.707);

        let low_freq_input = generate_sine_wave(low_freq, sample_rate, duration_samples);
        let high_freq_input = generate_sine_wave(high_freq, sample_rate, duration_samples);

        let mut low_freq_output = Vec::new();
        let mut high_freq_output = Vec::new();

        for &sample in &low_freq_input {
            low_freq_output.push(filter.process(sample, sample_rate));
        }

        let mut filter = LowpassFilter::default();
        filter.freq(cutoff_normalized).q(0.707);

        for &sample in &high_freq_input {
            high_freq_output.push(filter.process(sample, sample_rate));
        }

        let low_input_rms = calculate_rms(&low_freq_input[1000..]);
        let low_output_rms = calculate_rms(&low_freq_output[1000..]);
        let high_input_rms = calculate_rms(&high_freq_input[1000..]);
        let high_output_rms = calculate_rms(&high_freq_output[1000..]);

        let low_attenuation_db = 20.0 * (low_output_rms / low_input_rms).log10();
        let high_attenuation_db = 20.0 * (high_output_rms / high_input_rms).log10();

        assert!(
            low_attenuation_db > -6.0,
            "Low frequency should pass through with minimal attenuation"
        );
        assert!(
            high_attenuation_db < -20.0,
            "High frequency should be significantly attenuated"
        );
    }

    #[test]
    fn test_highpass_filter_attenuates_low_frequencies() {
        let sample_rate = 44100.0;
        let duration_samples = 4410;
        let low_freq = 200.0;
        let high_freq = 5000.0;
        let cutoff_normalized = 1000.0 / (sample_rate / 2.0);

        let mut filter = HighpassFilter::default();
        filter.freq(cutoff_normalized).q(0.707);

        let low_freq_input = generate_sine_wave(low_freq, sample_rate, duration_samples);
        let high_freq_input = generate_sine_wave(high_freq, sample_rate, duration_samples);

        let mut low_freq_output = Vec::new();
        let mut high_freq_output = Vec::new();

        for &sample in &low_freq_input {
            low_freq_output.push(filter.process(sample, sample_rate));
        }

        let mut filter = HighpassFilter::default();
        filter.freq(cutoff_normalized).q(0.707);

        for &sample in &high_freq_input {
            high_freq_output.push(filter.process(sample, sample_rate));
        }

        let low_input_rms = calculate_rms(&low_freq_input[1000..]);
        let low_output_rms = calculate_rms(&low_freq_output[1000..]);
        let high_input_rms = calculate_rms(&high_freq_input[1000..]);
        let high_output_rms = calculate_rms(&high_freq_output[1000..]);

        let low_attenuation_db = 20.0 * (low_output_rms / low_input_rms).log10();
        let high_attenuation_db = 20.0 * (high_output_rms / high_input_rms).log10();

        assert!(
            low_attenuation_db < -20.0,
            "Low frequency should be significantly attenuated"
        );
        assert!(
            high_attenuation_db > -6.0,
            "High frequency should pass through with minimal attenuation"
        );
    }

    #[test]
    fn test_normalized_frequency_range() {
        let sample_rate = 44100.0;
        let mut lpf = LowpassFilter::default();
        let mut hpf = HighpassFilter::default();

        lpf.freq(0.5);
        hpf.freq(0.25);

        let input = generate_sine_wave(1000.0, sample_rate, 100);

        for &sample in &input {
            let lp_out = lpf.process(sample, sample_rate);
            let hp_out = hpf.process(sample, sample_rate);

            assert!(lp_out.is_finite());
            assert!(hp_out.is_finite());
        }
    }
}
