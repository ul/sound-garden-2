//! # JACK Stack
//!
//! JACK clients and connections manager operated via a simple stack-based language.

#[macro_use]
extern crate clap;
extern crate fnv;
extern crate itertools;
extern crate jack;
extern crate regex;
extern crate rosc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
extern crate sloggers;
#[macro_use]
extern crate slog_scope;
extern crate toml;

mod config;
mod module;
mod stack;

use clap::{App, Arg};
use rosc::{OscPacket, OscType};
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;
use stack::Stack;
use std::net::{SocketAddrV4, UdpSocket};

fn main() {
    let matches = App::new("JACK Stack")
        .version(crate_version!())
        .author("Ruslan Prokopchuk <fer.obbee@gmail.com>")
        .about("Manage JACK client and connections with a simple stack-based language")
        .arg(
            Arg::with_name("CONFIG")
                .long("config")
                .help("Config file")
                .required(true)
                .takes_value(true),
        ).arg(
            Arg::with_name("ADDRESS")
                .long("address")
                .help("Address to listen for OSC messages")
                .required(true)
                .default_value("127.0.0.1:7770")
                .takes_value(true),
        ).arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        ).get_matches();

    // Configure global logger
    let verbosity = matches.occurrences_of("v") as u8;

    let level = match verbosity {
        0 => Severity::Error,
        1 => Severity::Warning,
        2 => Severity::Info,
        3 => Severity::Debug,
        _ => Severity::Trace,
    };

    let mut builder = TerminalLoggerBuilder::new();
    builder.level(level);
    builder.destination(Destination::Stderr);
    let logger = builder.build().unwrap();
    let _guard = slog_scope::set_global_logger(logger);

    // Read config.
    let config = matches.value_of("CONFIG").unwrap(); // ok to unwrap as option is required
    let config = std::fs::read_to_string(config).expect("Failed to read config file.");
    let config: config::Config = toml::from_str(&config).expect("Failed to parse config file.");

    // Connect to JACK and create Stack.
    // USE_EXACT_NAME name used to prevent two jack-stack clients managing the same server instance.
    // To support multiple jack-stack clients we need at least:
    // * use unique suffixes in module names (so if you eval `440 s` and `440 s` in both clients
    //   it wouldn't crash trying to create two `constant_440_0` modules in JACK);
    // * disconnect system ports only from current stack's ports when rebuilding graph.
    let (client, _status) = jack::Client::new("jack-stack", jack::ClientOptions::USE_EXACT_NAME)
        .expect("Failed to connect to JACK.");
    let mut stack = Stack::new();

    // Spin up OSC server.
    let address = matches.value_of("ADDRESS").unwrap(); // ok to unwrap as option is required
    let addr: SocketAddrV4 = address.parse().expect("Failed to parse address.");
    let sock = UdpSocket::bind(addr).expect("Failed to bind socket.");
    let mut buf = [0u8; rosc::decoder::MTU];

    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                let packet = rosc::decoder::decode(&buf[..size]);
                match packet {
                    Ok(packet) => handle_packet(packet, &client, &config, &mut stack),
                    Err(e) => error!("Failed to decode OSC packet: {:?}.", e),
                }
            }
            Err(e) => {
                error!("Error receiving from socket: {}.", e);
                break;
            }
        }
    }
}

/// OSC router which matches message to addresses and calls appropriate handlers for them,
/// passing down app state like client, config, stack.
fn handle_packet(
    packet: OscPacket,
    client: &jack::Client,
    config: &config::Config,
    stack: &mut Stack,
) {
    match packet {
        OscPacket::Message(msg) => match &msg.addr as &str {
            "/eval" => if let Some(args) = msg.args {
                if args.is_empty() {
                    debug!("No arguments given to eval.");
                    return;
                };
                if args.len() > 1 {
                    warn!("Extra arguments to eval will be ignored.");
                }
                if let OscType::String(ref s) = args[0] {
                    stack.eval(s, client, config);
                } else {
                    warn!("Expected string to eval, but got {:?}", args[0]);
                }
            },
            _ => {
                debug!("OSC address: {}.", msg.addr);
                match msg.args {
                    Some(args) => {
                        debug!("OSC arguments: {:?}.", args);
                    }
                    None => debug!("No arguments in message."),
                }
            }
        },
        OscPacket::Bundle(bundle) => {
            debug!("OSC Bundle: {:?}.", bundle);
        }
    }
}
