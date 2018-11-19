//! # Constant
//!
//! Always write the same value provided as the `--value` argument to the `output` port.

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};
use synth_modules::prelude::*;

pub fn main() {
    let matches = App::new("Constant")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Generate constant value signal")
        .arg(
            Arg::with_name("VALUE")
                .long("value")
                .help("Value to output")
                .required(true)
                .takes_value(true),
        ).arg(
            Arg::with_name("NAME")
                .long("name")
                .help("Client name")
                .required(true)
                .takes_value(true),
        ).get_matches();

    let value: Sample = matches
        .value_of("VALUE")
        .unwrap()
        .parse()
        .expect("Value must be a number");

    let name = matches.value_of("NAME").unwrap();

    let (client, _status) = jack::Client::new(
        name,
        jack::ClientOptions::NO_START_SERVER | jack::ClientOptions::USE_EXACT_NAME,
    ).expect("Failed to connect to JACK");

    let module = Constant::new(value);

    let mut output = client
        .register_port("output", jack::AudioOut::default())
        .expect("Failed to register output port");

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        for sample in output.as_mut_slice(ps) {
            *sample = module.sample() as f32;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
