//! # Module
//!
//! Structures and routines dedicated to managing module instances.
use config::PrimitiveWord;
use itertools::Itertools;
use manager::Manager;
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
        manager: &Manager,
        definition: &PrimitiveWord,
        name: &str,
        slash_args_values: &[&str],
    ) -> Option<Self> {
        // extra_args are passed unconditionally.
        let mut args = definition.extra_args.as_ref().cloned().unwrap_or_default();
        // TODO Support passing slash args as positional.
        if let Some(ref slash_args) = definition.slash_args {
            if slash_args.len() < slash_args_values.len() {
                warn!("Extra slash args values will be ignored.");
            }
            let slash_args = slash_args.iter().cloned();
            let slash_args_values = slash_args_values.iter().map(|s| s.to_string());
            args.extend(slash_args.interleave(slash_args_values));
        }
        // Set module's client name so jack-stack will be able to manipulate module's ports.
        // NOTE This relies on the assumption that module sets USE_EXACT_NAME and will fail if name
        // is already taken.
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
        // * then polls one of module's output ports to be connectable.
        // 1 ms timeout is completely made up.
        let expected_ports_count = definition.inputs.len() + definition.outputs.len();
        let ports_regex = format!("^{}:.+$", regex::escape(name));
        while expected_ports_count > manager.count_ports(&ports_regex) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        if let Some(port) = definition.outputs.first() {
            let port = format!("{}:{}", name, port);
            while !manager.output_port_is_ready(&port) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        Some(Module {
            name: name.to_string(),
            process,
        })
    }
}
