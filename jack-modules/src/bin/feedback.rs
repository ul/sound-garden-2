//! # Feedback
//!
//! Pass `x` port signal via feedback comb filter controller via `delay` and `gain` ports.
//! Result goes to the `output` port.
//! Max delay must be provided via `--max-delay` argument to allocate appropriate buffer on start.

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};
use synth_modules::prelude::*;

pub fn main() {
    let matches = App::new("Feedback")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Feedback comb filter with variable delay and gain")
        .arg(
            Arg::with_name("MAX_DELAY")
                .long("max-delay")
                .help("Max allowed delay (seconds)")
                .required(true)
                .takes_value(true),
        ).arg(
            Arg::with_name("NAME")
                .long("name")
                .help("Client name")
                .required(true)
                .takes_value(true),
        ).get_matches();

    let max_delay: Sample = matches
        .value_of("MAX_DELAY")
        .unwrap()
        .parse()
        .expect("Max delay must be a number");

    let name = matches.value_of("NAME").unwrap();

    let (client, _status) = jack::Client::new(
        name,
        jack::ClientOptions::NO_START_SERVER | jack::ClientOptions::USE_EXACT_NAME,
    ).expect("Failed to connect to JACK");

    let mut module = Feedback::new(client.sample_rate(), max_delay);

    let x = client
        .register_port("x", jack::AudioIn::default())
        .expect("Failed to register input port");

    let delay = client
        .register_port("delay", jack::AudioIn::default())
        .expect("Failed to register input port");

    let gain = client
        .register_port("gain", jack::AudioIn::default())
        .expect("Failed to register input port");

    let mut output = client
        .register_port("output", jack::AudioOut::default())
        .expect("Failed to register output port");

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        for (((output, x), delay), gain) in output
            .as_mut_slice(ps)
            .into_iter()
            .zip(x.as_slice(ps))
            .zip(delay.as_slice(ps))
            .zip(gain.as_slice(ps))
        {
            *output =
                module.sample(Sample::from(*x), Sample::from(*delay), Sample::from(*gain)) as f32;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
