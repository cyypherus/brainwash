use std::f32::consts::TAU;

const NUM_INPUT_DIFFUSERS: usize = 4;
const NUM_TANK_CHANNELS: usize = 8;

const INPUT_DIFFUSION_TIMES: [usize; NUM_INPUT_DIFFUSERS] = [142, 107, 379, 277];
const INPUT_DIFFUSION_COEFFS: [f32; NUM_INPUT_DIFFUSERS] = [0.75, 0.75, 0.625, 0.625];

const TANK_DELAY_TIMES: [usize; NUM_TANK_CHANNELS] = [672, 1572, 2356, 3163, 908, 1800, 2656, 3720];

const MOD_DEPTHS: [f32; NUM_TANK_CHANNELS] = [8.0, 7.0, 6.0, 5.0, 9.0, 8.0, 7.0, 6.0];
const MOD_RATES: [f32; NUM_TANK_CHANNELS] = [0.5, 0.6, 0.7, 0.8, 0.55, 0.65, 0.75, 0.85];

pub struct Reverb {
    input_diffusers: [AllpassDiffuser; NUM_INPUT_DIFFUSERS],
    tank_delays: [ModulatedDelay; NUM_TANK_CHANNELS],
    tank_damping: [OnePole; NUM_TANK_CHANNELS],
    decay: f32,
    damping: f32,
    mod_depth_scale: f32,
    diffusion_amount: f32,
}

impl Default for Reverb {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / 29761.0;

        let input_diffusers = core::array::from_fn(|i| {
            AllpassDiffuser::new(
                ((INPUT_DIFFUSION_TIMES[i] as f32 * scale) as usize).max(1),
                INPUT_DIFFUSION_COEFFS[i],
            )
        });

        let tank_delays = core::array::from_fn(|i| {
            ModulatedDelay::new(
                ((TANK_DELAY_TIMES[i] as f32 * scale) as usize).max(1) + 32,
                (TANK_DELAY_TIMES[i] as f32 * scale) as usize,
                MOD_DEPTHS[i] * scale,
                MOD_RATES[i],
                sample_rate,
                i as f32 * 0.1,
            )
        });

        let tank_damping = core::array::from_fn(|_| OnePole::new(0.0005));

        Self {
            input_diffusers,
            tank_delays,
            tank_damping,
            decay: 0.5,
            damping: 0.0005,
            mod_depth_scale: 1.0,
            diffusion_amount: 1.0,
        }
    }

    pub fn roomsize(&mut self, size: f32) -> &mut Self {
        self.decay = 0.3 + size.clamp(0.0, 1.0) * 0.69;
        self
    }

    pub fn damp(&mut self, damp: f32) -> &mut Self {
        self.damping = damp.clamp(0.0, 1.0) * 0.5;
        for filter in &mut self.tank_damping {
            filter.set_coefficient(self.damping);
        }
        self
    }

    pub fn mod_depth(&mut self, depth: f32) -> &mut Self {
        self.mod_depth_scale = depth.clamp(0.0, 1.0);
        for delay in &mut self.tank_delays {
            delay.set_mod_scale(self.mod_depth_scale);
        }
        self
    }

    pub fn diffusion(&mut self, amount: f32) -> &mut Self {
        self.diffusion_amount = amount.clamp(0.0, 1.0);
        for diffuser in &mut self.input_diffusers {
            diffuser.set_coefficient_scale(self.diffusion_amount);
        }
        self
    }

    pub fn output(&mut self, input: f32) -> f32 {
        let mut diffused = input * 0.5;
        for diffuser in &mut self.input_diffusers {
            diffused = diffuser.process(diffused);
        }

        let mut tank_outputs = [0.0f32; NUM_TANK_CHANNELS];
        for i in 0..NUM_TANK_CHANNELS {
            tank_outputs[i] = self.tank_delays[i].read();
        }

        householder(&mut tank_outputs);

        for i in 0..NUM_TANK_CHANNELS {
            let damped = self.tank_damping[i].process(tank_outputs[i]);
            let feedback = damped * self.decay;
            let tank_input = diffused + feedback;
            self.tank_delays[i].write(tank_input);
        }

        let left = tank_outputs[0] + tank_outputs[2] - tank_outputs[4] + tank_outputs[6];
        let right = tank_outputs[1] - tank_outputs[3] + tank_outputs[5] + tank_outputs[7];
        (left + right) * 0.25
    }

    pub fn copy_state_from(&mut self, other: &Reverb) {
        for (new_d, old_d) in self
            .input_diffusers
            .iter_mut()
            .zip(other.input_diffusers.iter())
        {
            new_d.copy_state_from(old_d);
        }
        for (new_d, old_d) in self.tank_delays.iter_mut().zip(other.tank_delays.iter()) {
            new_d.copy_state_from(old_d);
        }
        for (new_f, old_f) in self.tank_damping.iter_mut().zip(other.tank_damping.iter()) {
            new_f.state = old_f.state;
        }
    }
}

fn householder(arr: &mut [f32; NUM_TANK_CHANNELS]) {
    let sum: f32 = arr.iter().sum();
    let scale = -2.0 / NUM_TANK_CHANNELS as f32;
    let add = sum * scale;
    for x in arr.iter_mut() {
        *x += add;
    }
}

struct AllpassDiffuser {
    buffer: Vec<f32>,
    index: usize,
    base_coefficient: f32,
    coefficient: f32,
}

impl AllpassDiffuser {
    fn new(size: usize, coefficient: f32) -> Self {
        Self {
            buffer: vec![0.0; size],
            index: 0,
            base_coefficient: coefficient,
            coefficient,
        }
    }

    fn set_coefficient_scale(&mut self, scale: f32) {
        self.coefficient = self.base_coefficient * scale;
    }

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.index];
        let output = delayed - self.coefficient * input;
        self.buffer[self.index] = input + self.coefficient * delayed;

        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }

        output
    }

    fn copy_state_from(&mut self, other: &AllpassDiffuser) {
        let copy_len = self.buffer.len().min(other.buffer.len());
        for i in 0..copy_len {
            self.buffer[i] = other.buffer[i];
        }
        self.index = other.index % self.buffer.len();
    }
}

struct ModulatedDelay {
    buffer: Vec<f32>,
    write_index: usize,
    base_delay: usize,
    base_mod_depth: f32,
    mod_depth: f32,
    phase: f32,
    phase_inc: f32,
}

impl ModulatedDelay {
    fn new(
        buffer_size: usize,
        base_delay: usize,
        mod_depth: f32,
        mod_rate: f32,
        sample_rate: f32,
        initial_phase: f32,
    ) -> Self {
        Self {
            buffer: vec![0.0; buffer_size],
            write_index: 0,
            base_delay,
            base_mod_depth: mod_depth,
            mod_depth,
            phase: initial_phase,
            phase_inc: mod_rate / sample_rate,
        }
    }

    fn set_mod_scale(&mut self, scale: f32) {
        self.mod_depth = self.base_mod_depth * scale;
    }

    fn read(&mut self) -> f32 {
        let mod_offset = self.mod_depth * (self.phase * TAU).sin();
        let delay = self.base_delay as f32 + mod_offset;

        self.phase += self.phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        let read_pos = self.write_index as f32 - delay;
        self.interpolate(read_pos)
    }

    fn write(&mut self, sample: f32) {
        self.buffer[self.write_index] = sample;
        self.write_index += 1;
        if self.write_index >= self.buffer.len() {
            self.write_index = 0;
        }
    }

    fn interpolate(&self, pos: f32) -> f32 {
        let len = self.buffer.len() as f32;
        let pos = ((pos % len) + len) % len;
        let i = pos.floor() as usize;
        let frac = pos - i as f32;

        let len = self.buffer.len();
        let y0 = self.buffer[(i + len - 1) % len];
        let y1 = self.buffer[i % len];
        let y2 = self.buffer[(i + 1) % len];
        let y3 = self.buffer[(i + 2) % len];

        let c0 = y1;
        let c1 = 0.5 * (y2 - y0);
        let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
        let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

        ((c3 * frac + c2) * frac + c1) * frac + c0
    }

    fn copy_state_from(&mut self, other: &ModulatedDelay) {
        let copy_len = self.buffer.len().min(other.buffer.len());
        for i in 0..copy_len {
            self.buffer[i] = other.buffer[i];
        }
        self.write_index = other.write_index % self.buffer.len();
        self.phase = other.phase;
    }
}

struct OnePole {
    state: f32,
    coefficient: f32,
}

impl OnePole {
    fn new(coefficient: f32) -> Self {
        Self {
            state: 0.0,
            coefficient,
        }
    }

    fn set_coefficient(&mut self, coeff: f32) {
        self.coefficient = coeff;
    }

    fn process(&mut self, input: f32) -> f32 {
        self.state = input * (1.0 - self.coefficient) + self.state * self.coefficient;
        self.state
    }
}
