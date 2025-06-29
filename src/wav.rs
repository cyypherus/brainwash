use crate::Signal;

pub fn save_wav(
    mut synth: impl FnMut(&mut Signal) -> f32 + Send + 'static,
    filename: &str,
    duration_seconds: f32,
    sample_rate: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(filename, spec)?;
    let mut signal = Signal::new(sample_rate);
    let total_samples = (duration_seconds * sample_rate as f32) as usize;

    for _ in 0..total_samples {
        let sample = synth(&mut signal).clamp(-1., 1.);
        writer.write_sample(sample)?; // Left channel
        writer.write_sample(sample)?; // Right channel
        signal.advance();
    }

    writer.finalize()?;
    Ok(())
}
