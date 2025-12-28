pub fn mix(signals: Vec<f32>) -> f32 {
    if signals.is_empty() {
        return 0.0;
    }
    let sum: f32 = signals.iter().sum();
    sum / signals.len() as f32
}

pub fn gain(amount: f32, signal: f32) -> f32 {
    signal * amount
}
