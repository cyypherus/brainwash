#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Sample(f32);

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Unit(f32);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frame {
    left: Sample,
    right: Sample,
}

impl Sample {
    pub const ZERO: Self = Self(0.0);

    pub fn new(value: f32) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    pub(crate) fn raw(value: f32) -> Self {
        Self::finite(value)
    }

    pub(crate) fn scale(self, value: f32) -> Self {
        Self::finite(self.0 * value)
    }

    pub fn value(self) -> f32 {
        self.0
    }

    pub fn clipped(self) -> Self {
        Self(self.0.clamp(-1.0, 1.0))
    }

    pub(crate) fn add(self, rhs: Self) -> Self {
        Self::finite(self.0 + rhs.0)
    }

    pub(crate) fn sub(self, rhs: Self) -> Self {
        Self::finite(self.0 - rhs.0)
    }

    pub(crate) fn attenuate(self, amount: Unit) -> Self {
        Self::finite(self.0 * amount.0)
    }

    fn finite(value: f32) -> Self {
        if value.is_finite() {
            Self(value)
        } else if value.is_nan() {
            Self::ZERO
        } else {
            Self(value.signum() * f32::MAX)
        }
    }
}

impl Unit {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(Self(value))
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

impl Frame {
    pub const SILENCE: Self = Self {
        left: Sample::ZERO,
        right: Sample::ZERO,
    };

    pub fn mono(sample: Sample) -> Self {
        Self {
            left: sample,
            right: sample,
        }
    }

    pub fn stereo(left: Sample, right: Sample) -> Self {
        Self { left, right }
    }

    pub fn left(self) -> Sample {
        self.left
    }

    pub fn right(self) -> Sample {
        self.right
    }
}
