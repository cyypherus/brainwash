pub fn mix(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f32>() / samples.len() as f32
}
pub(crate) fn midi_to_freq(note: f32) -> f32 {
    440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
}

pub(crate) fn note_to_freq(note: f32) -> f32 {
    midi_to_freq(note + 60.0)
}
