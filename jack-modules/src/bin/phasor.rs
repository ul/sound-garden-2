//! # Phasor
//!
//! Oscillate as a saw wave in interval [-1, 1] with the frequency provided via the `frequency` port
//! and write current phase into the `phase` port.

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};
use synth_modules::prelude::*;

pub fn main() {
    let matches = App::new("Phasor")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Generate phase in [-1, 1] interval with the input frequency")
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

    let mut module = Phasor::new(client.sample_rate());

    let frequency = client
        .register_port("frequency", jack::AudioIn::default())
        .expect("Failed to register input port");

    let mut phase = client
        .register_port("phase", jack::AudioOut::default())
        .expect("Failed to register output port");

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        for (phase, frequency) in phase
            .as_mut_slice(ps)
            .into_iter()
            .zip(frequency.as_slice(ps))
        {
            *phase = module.sample(Sample::from(*frequency)) as f32;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
