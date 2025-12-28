pub fn save_wav(
    filename: &str,
    duration_seconds: f32,
    sample_rate: usize,
    mut synth: impl FnMut(usize) -> f32 + Send + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(filename, spec)?;
    let total_samples = (duration_seconds * sample_rate as f32) as usize;
    for counter in 0..total_samples {
        // #[cfg(all(debug_assertions, feature = "no-alloc"))]
        // let sample = assert_no_alloc(|| synth(counter).clamp(-1., 1.));
        #[cfg(not(all(debug_assertions, feature = "no-alloc")))]
        let sample = synth(counter).clamp(-1., 1.);
        writer.write_sample(sample)?;
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    Ok(())
}
