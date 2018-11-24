//! # Numeric functions
//!
//! These functions could be passed to Fn1::new, Fn2::new and so on (depending on arity)
//! to create Modules which just pass their sources through pure transformation.
use prelude::*;
use std::f64::consts::PI;

// Arithmetics

pub fn add(x: Sample, y: Sample) -> Sample {
    x + y
}

pub fn mul(x: Sample, y: Sample) -> Sample {
    x * y
}

pub fn sub(x: Sample, y: Sample) -> Sample {
    x - y
}

pub fn div(x: Sample, y: Sample) -> Sample {
    x / y
}

// Trigonometry

pub fn sin(x: Sample) -> Sample {
    x.sin()
}

pub fn cos(x: Sample) -> Sample {
    x.cos()
}

// Projections

/// Assuming that x varies in the range a..b linearly project it into the range c..d
pub fn linlin(x: Sample, a: Sample, b: Sample, c: Sample, d: Sample) -> Sample {
    (d - c) * (x - a) / (b - a) + c
}

/// Assuming that x varies in the range -1..1 linearly project it into the range a..b
pub fn range(x: Sample, a: Sample, b: Sample) -> Sample {
    linlin(x, -1.0, 1.0, a, b)
}

/// Assuming that x varies in the range -1..1 linearly project it into the range 0..1
pub fn unit(x: Sample) -> Sample {
    range(x, 0.0, 1.0)
}

/// Assuming that x varies in the range -1..1 linearly project it into the range -PI..PI
pub fn circle(x: Sample) -> Sample {
    range(x, -PI, PI)
}

// Oscillators-ready

/// Connect Phasor to Fn1(sine) to generate sine wave
pub fn sine(phase: Sample) -> Sample {
    sin(2.0 * PI * phase)
}

/// Connect Phasor to Fn1(cosine) to generate cosine wave
pub fn cosine(phase: Sample) -> Sample {
    cos(2.0 * std::f64::consts::PI * phase)
}

/// Connect Phasor to Fn1(triangle) to generate symmetric triangle wave
pub fn triangle(phase: Sample) -> Sample {
    let x = 2.0 * phase;
    if x > 0.0 {
        1.0 - x
    } else {
        1.0 + x
    }
}

/// Connect Phasor and module which outputs pulse width (e.g. Constant(0.5))
/// to Fn2(rectangle) to generate rectangle wave
pub fn rectangle(phase: Sample, width: Sample) -> Sample {
    if unit(phase) <= width {
        1.0
    } else {
        -1.0
    }
}

/// Stereo intensity-preserving panner
pub fn pan(l: Sample, r: Sample, c: Sample) -> (Sample, Sample) {
    (
        1.0_f64.min(1.0 - c).sqrt() * l + 0.0_f64.max(-c).sqrt() * r,
        0.0_f64.max(c).sqrt() * l + 1.0_f64.min(1.0 + c).sqrt() * r,
    )
}
