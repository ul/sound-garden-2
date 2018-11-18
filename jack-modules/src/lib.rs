//! # JACK modules
//!
//! Harness to convert backend-agnostic DSP modules into JACK clients.
//! Also provides wrapping for all modules from synth-modules.
extern crate crossbeam_channel;
extern crate jack;
extern crate synth_modules;
extern crate void;

pub mod notification;
