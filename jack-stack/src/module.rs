//! # Module
//!
//! Structures and routines dedicated to managing module instances.
use config::PrimitiveWord;
use itertools::Itertools;
use std::process::{Child, Command};

pub struct Module {
    /// JACK client name which corresponds to the module instance.
    pub name: String,
    /// Module instance handler used to kill it when module is dropped.
    process: Child,
}

impl Drop for Module {
    fn drop(&mut self) {
        if self.process.kill().is_err() {
            warn!("Module `{}` is already dead.", self.name);
        };
        if self.process.wait().is_err() {
            warn!("Module `{}` was not even started!", self.name);
        };
    }
}

impl Module {
    /// Spawn a new module process and wait until its client is active.
    /// Return None if starting process failed.
    pub fn spawn(
        client: &jack::Client,
        definition: &PrimitiveWord,
        name: &str,
        slash_args_values: &[&str],
    ) -> Option<Self> {
        let mut args = definition.extra_args.as_ref().cloned().unwrap_or_default();
        // TODO Support passing slash args positionally.
        if let Some(ref slash_args) = definition.slash_args {
            if slash_args.len() < slash_args_values.len() {
                warn!("Extra slash args values will be ignored.");
            }
            let slash_args = slash_args.iter().cloned();
            let slash_args_values = slash_args_values.iter().map(|s| s.to_string());
            args.extend(slash_args.interleave(slash_args_values));
        }
        args.push(definition.name_arg.to_owned());
        args.push(name.to_string());
        let process = Command::new(definition.cmd.to_owned()).args(args).spawn();
        if process.is_err() {
            return None;
        }
        let process = process.unwrap();
        // JACK doesn't provide any good way to ensure that specific client is active
        // (please let me know if it does and I just don't know).
        // To wait for the module's client being ready code below:
        // * polls server for client ports to be registered;
        // * waits additional 5 ms for client activation.
        // TODO Make it more robust (by trying to connect ports to some dummy port?).
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1));
            let ports = client.ports(
                Some(&format!("^{}:.*$", regex::escape(name))),
                None,
                jack::PortFlags::empty(),
            );
            if !ports.is_empty() {
                break;
            }
        }
        // Module's ports are registered now, but client activation requires additional time.
        // 5ms is just a wild guess of what "should be enough."
        std::thread::sleep(std::time::Duration::from_millis(5));
        Some(Module {
            name: name.to_string(),
            process,
        })
    }
}
