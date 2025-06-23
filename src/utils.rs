pub fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}
pub(crate) fn midi_to_freq(note: f32) -> f32 {
    440.0 * 2.0_f32.powf((note - 69.0) / 12.0)
}

pub(crate) fn note_to_freq(note: f32) -> f32 {
    midi_to_freq(note + 60.0)
}
