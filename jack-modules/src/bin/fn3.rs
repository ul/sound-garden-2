//! # Fn3
//!
//! Transform `x` and `y` and `z` ports signals with a ternary function and write result to the `output` port.
//! Function must be selected via `--fn` argument:
//!
//! * range -- [-1, 1] -> [y, z]

#[macro_use]
extern crate clap;
extern crate jack;
extern crate jack_modules;
extern crate synth_modules;

use clap::{App, Arg};
use synth_modules::prelude::*;

pub fn main() {
    let matches = App::new("Fn3")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Pure function of 3 arguments")
        .arg(
            Arg::with_name("FN")
                .long("fn")
                .help("Name of the function to apply")
                .required(true)
                .takes_value(true),
        ).arg(
            Arg::with_name("NAME")
                .long("name")
                .help("Client name")
                .required(true)
                .takes_value(true),
        ).get_matches();

    let f = match matches.value_of("FN").unwrap() {
        "range" => range,
        name => panic!("Unknown function: {}", name),
    };

    let name = matches.value_of("NAME").unwrap();

    let (client, _status) = jack::Client::new(
        name,
        jack::ClientOptions::NO_START_SERVER | jack::ClientOptions::USE_EXACT_NAME,
    ).expect("Failed to connect to JACK");

    let x = client
        .register_port("x", jack::AudioIn::default())
        .expect("Failed to register input port");

    let y = client
        .register_port("y", jack::AudioIn::default())
        .expect("Failed to register input port");

    let z = client
        .register_port("z", jack::AudioIn::default())
        .expect("Failed to register input port");

    let mut output = client
        .register_port("output", jack::AudioOut::default())
        .expect("Failed to register output port");

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
        for (((output, x), y), z) in output
            .as_mut_slice(ps)
            .into_iter()
            .zip(x.as_slice(ps))
            .zip(y.as_slice(ps))
            .zip(z.as_slice(ps))
        {
            *output = f(Sample::from(*x), Sample::from(*y), Sample::from(*z)) as f32;
        }
        jack::Control::Continue
    };
    let process = jack::ClosureProcessHandler::new(process_callback);

    let (notification, is_alive) = jack_modules::notification::Notification::new();
    let active_client = client.activate_async(notification, process).unwrap();

    assert!(is_alive.recv().is_err());

    active_client.deactivate().unwrap();
}
