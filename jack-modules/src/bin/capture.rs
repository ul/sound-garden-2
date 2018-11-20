//! # Capture
//!
//! Proxy system capture to the `output_1` and `output_2` ports.

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};

pub fn main() {
    let matches = App::new("Capture")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Proxy system capture")
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

    let inputs = [input_1, input_2];
    let input_names = inputs
        .iter()
        .map(|p| p.name().unwrap().to_owned())
        .collect::<Vec<_>>();

    let output_1 = client
        .register_port("output_1", jack::AudioOut::default())
        .expect("Failed to register output port");

    let output_2 = client
        .register_port("output_2", jack::AudioOut::default())
        .expect("Failed to register output port");

    let mut outputs = [output_1, output_2];

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        let inputs = inputs.iter().flat_map(|inputs| inputs.as_slice(ps));
        let outputs = outputs
            .iter_mut()
            .flat_map(|outputs| outputs.as_mut_slice(ps));
        for (output, input) in outputs.zip(inputs.cycle()) {
            *output = *input;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    let (client, _status) = jack::Client::new(name, jack::ClientOptions::NO_START_SERVER)
        .expect("Failed to connect to JACK");

    // NOTE This relies on the assumption that system input ports are named
    // system:capture_1, system:capture_2 and so on.
    // TODO Ensure order.
    let capture_ports = client.ports(
        Some("^system:capture_[0-9]+$"),
        None,
        jack::PortFlags::empty(),
    );

    for (input, capture) in input_names.iter().zip(capture_ports.iter().cycle()) {
        client
            .connect_ports_by_name(capture, input)
            .expect("Failed to connect ports");
    }

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
