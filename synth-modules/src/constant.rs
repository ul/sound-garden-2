//! # Constant
//!
//! Signal which outputs the same value all the time.
use sample::Sample;

pub struct Constant {
    value: Sample,
}

impl Constant {
    pub fn new(value: Sample) -> Self {
        Constant { value }
    }

    pub fn sample(&self) -> Sample {
        self.value
    }
}
