use crate::{allpass::AllpassFilter, comb::CombFilter};

const NUM_COMBS: usize = 8;
const NUM_ALLPASSES: usize = 4;
const FIXED_GAIN: f32 = 0.015;
const SCALE_DAMP: f32 = 0.4;
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;

const COMB_TUNINGS: [usize; NUM_COMBS] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNINGS: [usize; NUM_ALLPASSES] = [556, 441, 341, 225];

pub struct Reverb {
    combs: [CombFilter; NUM_COMBS],
    allpasses: [AllpassFilter; NUM_ALLPASSES],
    roomsize: f32,
    damp: f32,
}

impl Default for Reverb {
    fn default() -> Self {
        let mut reverb = Self {
            combs: [
                CombFilter::new(COMB_TUNINGS[0]),
                CombFilter::new(COMB_TUNINGS[1]),
                CombFilter::new(COMB_TUNINGS[2]),
                CombFilter::new(COMB_TUNINGS[3]),
                CombFilter::new(COMB_TUNINGS[4]),
                CombFilter::new(COMB_TUNINGS[5]),
                CombFilter::new(COMB_TUNINGS[6]),
                CombFilter::new(COMB_TUNINGS[7]),
            ],
            allpasses: [
                AllpassFilter::new(ALLPASS_TUNINGS[0]),
                AllpassFilter::new(ALLPASS_TUNINGS[1]),
                AllpassFilter::new(ALLPASS_TUNINGS[2]),
                AllpassFilter::new(ALLPASS_TUNINGS[3]),
            ],
            roomsize: 0.5,
            damp: 0.5,
        };

        for allpass in &mut reverb.allpasses {
            allpass.feedback(0.5);
        }

        reverb.update();
        reverb
    }
}

impl Reverb {
    pub fn roomsize(&mut self, size: f32) -> &mut Self {
        let clamped_size = size.clamp(0.0, 1.0);
        self.roomsize = (clamped_size * SCALE_ROOM) + OFFSET_ROOM;
        self.update();
        self
    }

    pub fn damp(&mut self, damp: f32) -> &mut Self {
        let clamped_damp = damp.clamp(0.0, 1.0);
        self.damp = clamped_damp * SCALE_DAMP;
        self.update();
        self
    }

    pub fn output(&mut self, input: f32) -> f32 {
        let mut output = 0.0;
        let scaled_input = input * FIXED_GAIN;

        for comb in &mut self.combs {
            output += comb.output(scaled_input);
        }

        for allpass in &mut self.allpasses {
            output = allpass.process(output);
        }

        output
    }

    fn update(&mut self) {
        for comb in &mut self.combs {
            comb.feedback(self.roomsize);
            comb.damp(self.damp);
        }
    }
}
