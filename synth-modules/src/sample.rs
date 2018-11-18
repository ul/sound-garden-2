//! # Basic audio signal types

/// The type which modules talk to each other.
///
/// Rationale behind choosing f64 over f32 despite the fact that most of audio drivers work with
/// f32 is that when you create a composite module signal experiences a lot of transformations and
/// the less is rounding error accumulation is better. Regarding performance, The Book says: "The
/// default type is f64 because on modern CPUs it’s roughly the same speed as f32 but is capable of
/// more precision."
pub type Sample = f64;
