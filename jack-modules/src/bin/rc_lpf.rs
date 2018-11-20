//! # RC LPF
//!
//! Apply low-pass filter to the `x` port signal with cut-off frequency passed via `frequency` port.
//! Write result into the `output` port.

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};
use synth_modules::prelude::*;

pub fn main() {
    let matches = App::new("RC LPF")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Simple IIR low-pass filter")
        .arg(
            Arg::with_name("NAME")
                .long("name")
                .help("Client name")
                .required(true)
                .takes_value(true),
        ).get_matches();

    let name = matches.value_of("NAME").unwrap();

    let (client, _status) = jack::Client::new(
        name,
        jack::ClientOptions::NO_START_SERVER | jack::ClientOptions::USE_EXACT_NAME,
    ).expect("Failed to connect to JACK");

    let mut module = LPF::new(client.sample_rate());

    let x = client
        .register_port("x", jack::AudioIn::default())
        .expect("Failed to register input port");

    let frequency = client
        .register_port("frequency", jack::AudioIn::default())
        .expect("Failed to register input port");

    let mut output = client
        .register_port("output", jack::AudioOut::default())
        .expect("Failed to register output port");

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        for ((output, x), frequency) in output
            .as_mut_slice(ps)
            .into_iter()
            .zip(x.as_slice(ps))
            .zip(frequency.as_slice(ps))
        {
            *output = module.sample(Sample::from(*x), Sample::from(*frequency)) as f32;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
