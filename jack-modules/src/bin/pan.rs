//! # Pan
//!
//! Intensity-preserving stereo panner. `input_1` (left) and `input_2` (right) ports panning is
//! controlled by `c` port (from -1 for the left to 1 for the right). Panned inputs are written to
//! `output_1` and `output_2` ports.

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};
use synth_modules::prelude::*;

pub fn main() {
    let matches = App::new("Pan")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Intensity-preserving stereo panner")
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

    let input_1 = client
        .register_port("input_1", jack::AudioIn::default())
        .expect("Failed to register input port");

    let input_2 = client
        .register_port("input_2", jack::AudioIn::default())
        .expect("Failed to register input port");

    let c = client
        .register_port("c", jack::AudioIn::default())
        .expect("Failed to register input port");

    let mut output_1 = client
        .register_port("output_1", jack::AudioOut::default())
        .expect("Failed to register output port");

    let mut output_2 = client
        .register_port("output_2", jack::AudioOut::default())
        .expect("Failed to register output port");

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        for ((((output_1, output_2), input_1), input_2), c) in output_1
            .as_mut_slice(ps)
            .into_iter()
            .zip(output_2.as_mut_slice(ps))
            .zip(input_1.as_slice(ps))
            .zip(input_2.as_slice(ps))
            .zip(c.as_slice(ps))
        {
            let (left, right) = pan(
                Sample::from(*input_1),
                Sample::from(*input_2),
                Sample::from(*c),
            );
            *output_1 = left as f32;
            *output_2 = right as f32;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
