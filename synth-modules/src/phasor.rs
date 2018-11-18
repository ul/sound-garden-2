//! # Phasor
//!
//! ```
//!  1     /|    /|    /|    /|
//!       / |   / |   / |   / |
//!  0   /  |  /  |  /  |  /  |
//!     /   | /   | /   | /   |
//! -1 /    |/    |/    |/    |
//! ```
//!
//! Phasor module generates a saw wave in the range -1..1.
//!
//! It is called phasor because it could be used as input phase for other oscillators, which become
//! just pure transformations then and are not required to care about handling varying frequency by
//! themselves anymore.
//!
//! Sources to connect: frequency.

use sample::Sample;

pub struct Phasor {
    phase: Sample,
    sample_period: Sample,
}

impl Phasor {
    pub fn new(sample_rate: usize) -> Self {
        Phasor {
            phase: 0.0,
            sample_period: (sample_rate as Sample).recip(),
        }
    }

    pub fn sample(&mut self, frequency: Sample) -> Sample {
        let dx = frequency * self.sample_period;
        self.phase = (self.phase + dx + 1.0) % 2.0 - 1.0;
        self.phase
    }
}
