//! Filters
//!
//! Basic IIR low/high-pass filters.
//!
//! Sources to connect: input, cut-off frequency.
use sample::Sample;

pub struct LPF {
    output: Sample,
    sample_angular_period: Sample,
}

impl LPF {
    pub fn new(sample_rate: usize) -> Self {
        let sample_angular_period = 2.0 * std::f64::consts::PI / sample_rate as Sample;
        LPF {
            output: 0.0,
            sample_angular_period,
        }
    }

    pub fn sample(&mut self, input: Sample, frequency: Sample) -> Sample {
        let k = frequency * self.sample_angular_period;
        let a = k / (k + 1.0);
        self.output += a * (input - self.output);
        self.output
    }
}

pub struct HPF {
    input: Sample,
    output: Sample,
    sample_angular_period: Sample,
}

impl HPF {
    pub fn new(sample_rate: usize) -> Self {
        let sample_angular_period = 2.0 * std::f64::consts::PI / sample_rate as Sample;
        HPF {
            input: 0.0,
            output: 0.0,
            sample_angular_period,
        }
    }

    pub fn sample(&mut self, input: Sample, frequency: Sample) -> Sample {
        let k = frequency * self.sample_angular_period;
        let a = 1.0 / (k + 1.0);
        self.output = a * (self.output + input - self.input);
        self.input = input;
        self.output
    }
}
