//! # Feedback
//!
//! Feedback comb filter with variable delay time and gain.
//!
//! Sources to connect: input to delay, delay time, gain.
use delay::Delay;
use sample::Sample;

pub struct Feedback {
    delay: Delay,
    output: Sample,
}

impl Feedback {
    pub fn new(sample_rate: usize, max_delay: Sample) -> Self {
        let delay = Delay::new(sample_rate, max_delay);
        Feedback { delay, output: 0.0 }
    }

    pub fn sample(&mut self, input: Sample, delay: Sample, gain: Sample) -> Sample {
        let delayed = self.delay.sample(self.output, delay);
        self.output = input + gain * delayed;
        self.output
    }
}
