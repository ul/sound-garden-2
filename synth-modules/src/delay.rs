//! # Delay
//!
//! Variable signal delay up to maximum period.
//!
//! Sources to connect: input to delay, delay time.
use sample::Sample;

pub struct Delay {
    buffer: Vec<Sample>,
    mask: usize,
    frame_number: usize,
    sample_rate: Sample,
}

impl Delay {
    pub fn new(sample_rate: usize, max_delay: Sample) -> Self {
        let sample_rate = sample_rate as Sample;
        // +1 because interpolation looks for the next sample
        // next_power_of_two to trade memory for speed by replacing `mod` with `&`
        let max_delay_frames = ((sample_rate * max_delay) as usize + 1).next_power_of_two();
        let mask = max_delay_frames - 1;
        let buffer = vec![0.0; max_delay_frames];
        Delay {
            buffer,
            frame_number: 0,
            mask,
            sample_rate,
        }
    }

    pub fn sample(&mut self, x: Sample, delay: Sample) -> Sample {
        let z = delay * self.sample_rate;
        let delay = z as usize;
        let k = z.fract();
        let output = if self.frame_number > delay {
            let i = self.frame_number - delay;
            let a = self.buffer[(i - 1) & self.mask];
            let b = self.buffer[i & self.mask];
            k * a + (1.0 - k) * b
        } else {
            0.0
        };
        self.buffer[(self.frame_number & self.mask)] = x;
        self.frame_number += 1;
        output
    }
}
