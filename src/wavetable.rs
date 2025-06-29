#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum InterpolationMethod {
    None,
    Linear,
    Cubic,
    Hermite,
}

pub(crate) struct WaveTable<const N: usize> {
    table: [f32; N],
}

impl<const N: usize> WaveTable<N> {
    pub(crate) const fn new() -> Self {
        Self { table: [0.0; N] }
    }

    pub(crate) const fn from_array(table: [f32; N]) -> Self {
        Self { table }
    }

    pub(crate) fn from_slice(data: &[f32]) -> Self {
        let mut table = [0.0; N];
        let copy_len = data.len().min(N);
        table[..copy_len].copy_from_slice(&data[..copy_len]);
        Self { table }
    }

    pub(crate) fn from_fn<F>(f: F) -> Self
    where
        F: Fn(usize) -> f32,
    {
        let mut table = [0.0; N];
        for i in 0..N {
            table[i] = f(i);
        }
        Self { table }
    }

    pub(crate) const fn len(&self) -> usize {
        N
    }

    pub(crate) const fn is_empty(&self) -> bool {
        N == 0
    }

    #[inline]
    pub(crate) fn sample_at(&self, position: f32, method: InterpolationMethod) -> f32 {
        match method {
            InterpolationMethod::None => self.sample_no_interpolation(position),
            InterpolationMethod::Linear => self.sample_linear(position),
            InterpolationMethod::Cubic => self.sample_cubic(position),
            InterpolationMethod::Hermite => self.sample_hermite(position),
        }
    }

    #[inline]
    fn sample_no_interpolation(&self, position: f32) -> f32 {
        let index = (position * N as f32) as usize % N;
        self.table[index]
    }

    #[inline]
    fn sample_linear(&self, position: f32) -> f32 {
        let table_pos = position * N as f32;
        let index = table_pos as usize % N;
        let frac = table_pos - (index as f32);

        let sample0 = self.table[index];
        let sample1 = self.table[(index + 1) % N];

        sample0 + frac * (sample1 - sample0)
    }

    #[inline]
    fn sample_cubic(&self, position: f32) -> f32 {
        let table_pos = position * N as f32;
        let index = table_pos as usize % N;
        let frac = table_pos - (index as f32);

        let y0 = self.table[(index + N - 1) % N];
        let y1 = self.table[index];
        let y2 = self.table[(index + 1) % N];
        let y3 = self.table[(index + 2) % N];

        let a0 = y3 - y2 - y0 + y1;
        let a1 = y0 - y1 - a0;
        let a2 = y2 - y0;
        let a3 = y1;

        a0 * frac * frac * frac + a1 * frac * frac + a2 * frac + a3
    }

    #[inline]
    fn sample_hermite(&self, position: f32) -> f32 {
        let table_pos = position * N as f32;
        let index = table_pos as usize % N;
        let frac = table_pos - (index as f32);

        let y0 = self.table[(index + N - 1) % N];
        let y1 = self.table[index];
        let y2 = self.table[(index + 1) % N];
        let y3 = self.table[(index + 2) % N];

        let c0 = y1;
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

        ((c3 * frac + c2) * frac + c1) * frac + c0
    }
}

impl<const N: usize> Default for WaveTable<N> {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct WaveTableOscillator<const N: usize> {
    wavetable: WaveTable<N>,
    phase_accumulator: f32,
    interpolation: InterpolationMethod,
}

impl<const N: usize> WaveTableOscillator<N> {
    pub(crate) const fn new(wavetable: WaveTable<N>) -> Self {
        Self {
            wavetable,
            phase_accumulator: 0.0,
            interpolation: InterpolationMethod::Linear,
        }
    }

    pub(crate) fn set_interpolation(&mut self, method: InterpolationMethod) {
        self.interpolation = method;
    }

    pub(crate) fn set_wavetable(&mut self, wavetable: WaveTable<N>) {
        self.wavetable = wavetable;
    }

    pub(crate) fn reset_phase(&mut self) {
        self.phase_accumulator = 0.0;
    }

    pub(crate) fn set_phase(&mut self, phase: f32) {
        self.phase_accumulator = phase.fract();
    }

    #[inline]
    pub(crate) fn tick(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        let sample = self
            .wavetable
            .sample_at(self.phase_accumulator, self.interpolation);

        self.phase_accumulator += frequency / sample_rate;
        if self.phase_accumulator >= 1.0 {
            self.phase_accumulator -= 1.0;
        }

        sample
    }

    #[inline]
    pub(crate) fn sample_at_phase(&self, phase: f32) -> f32 {
        self.wavetable.sample_at(phase.fract(), self.interpolation)
    }
}

impl<const N: usize> Default for WaveTableOscillator<N> {
    fn default() -> Self {
        Self::new(WaveTable::default())
    }
}

const fn compute_dc_offset<const N: usize>(table: &[f32; N]) -> f32 {
    let mut sum = 0.0;
    let mut i = 0;
    while i < N {
        sum += table[i];
        i += 1;
    }
    sum / N as f32
}

const fn compute_max_abs<const N: usize>(table: &[f32; N]) -> f32 {
    let mut max_val = 0.0;
    let mut i = 0;
    while i < N {
        let abs_val = if table[i] < 0.0 { -table[i] } else { table[i] };
        if abs_val > max_val {
            max_val = abs_val;
        }
        i += 1;
    }
    max_val
}

const fn normalize_and_remove_dc<const N: usize>(mut table: [f32; N]) -> [f32; N] {
    let dc_offset = compute_dc_offset(&table);

    let mut i = 0;
    while i < N {
        table[i] -= dc_offset;
        i += 1;
    }

    let max_val = compute_max_abs(&table);
    if max_val > 0.0 {
        let scale = 1.0 / max_val;
        let mut i = 0;
        while i < N {
            table[i] *= scale;
            i += 1;
        }
    }

    table
}

pub(crate) mod presets {
    use super::*;

    const fn load_and_normalize_table(raw_table: [f32; 1024]) -> WaveTable<1024> {
        let normalized_table = normalize_and_remove_dc(raw_table);
        WaveTable::from_array(normalized_table)
    }

    // pub(crate) const SINE_1024: WaveTable<1024> =
    //     load_and_normalize_table(include!("../wavetables/sine_1024.dat"));
    // pub(crate) const COSINE_1024: WaveTable<1024> =
    //     load_and_normalize_table(include!("../wavetables/cosine_1024.dat"));
    // pub(crate) const SAW_1024: WaveTable<1024> =
    //     load_and_normalize_table(include!("../wavetables/sawtooth_1024.dat"));
    // pub(crate) const SQUARE_1024: WaveTable<1024> =
    //     load_and_normalize_table(include!("../wavetables/square_1024.dat"));
    // pub(crate) const TRIANGLE_1024: WaveTable<1024> =
    //     load_and_normalize_table(include!("../wavetables/triangle_1024.dat"));
}
