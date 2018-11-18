//! # Stack
//!
//! Manage JACK clients and connections with style using a simple stack-based language.
//!
//! TODO Always connect to the system:playback_* ports via [-1, 1] clip module.

use config::{Config, WordDefinition};
use module::Module;

pub struct Stack {
    /// Collection of used modules.
    /// When modules is not used anymore, just replace its vec entry with `None`
    /// and corresponding process will be killed.
    /// This is not the most compact way to bookkeep modules, but it's simple
    /// and in practice not that wasteful (even thousands of `None`s accumulating during session
    /// are nothing in comparison with one-minute delay buffer for example).
    modules: Vec<Option<Module>>,
    /// Stack of ports. Top ones belonging to the same module are connected to the system:playback_*
    /// When new word is evaluated its module inputs consume from stack and outputs are put back to stack.
    stack: Vec<Element>,
}

/// Stack element corresponding to specific output port.
struct Element {
    /// Index of the module to which this port belongs to.
    idx: usize,
    /// Name of the port (without module name).
    port: String,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            modules: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Evaluate `s` and manage JACK clients and connections via `client` according to `config`.
    pub fn eval(&mut self, s: &str, client: &jack::Client, config: &Config) {
        for token in s.split_whitespace() {
            debug!("{}", token);
            let mut token = token.to_string();
            if token.parse::<f64>().is_ok() {
                token = format!("constant/{}", token);
            }
            let args = token.split('/').collect::<Vec<_>>();
            let word = args[0];
            match config.words.get(word) {
                Some(definition) => match definition {
                    WordDefinition::Primitive(definition) => {
                        if self.stack.len() < definition.inputs.len() {
                            error!("Not enough inputs on the stack.");
                            return;
                        }
                        let idx = self.modules.len();
                        let name = format!("{}_{}", token, idx);
                        let module = Module::spawn(client, definition, &name, &args[1..]);
                        if module.is_none() {
                            error!("Failed to spawn a module.");
                            return;
                        }
                        let module = module.unwrap();
                        self.modules.push(Some(module));
                        // Inputs are iterated in the reverse order to support following convention.
                        // Let word A has module outputs defined as ["a", "b"] and word X has module
                        // inputs defined as ["x", "y"]. Evaluating word A should put port "a" onto
                        // the stack first and then port "b". Evaluating word B should connect
                        // "a" to "x" and "b" to "y". As we pop from stack starting from the end,
                        // A's outputs appear in the reverse order. To match it, B's inputs must be
                        // iterated in the reverse order as well.
                        for input in definition.inputs.iter().rev() {
                            // Ok to unwrap as we checked stack len against inputs len.
                            let elem = self.stack.pop().unwrap();
                            client
                                .connect_ports_by_name(
                                    &format!(
                                        "{}:{}",
                                        // Should be ok to unwrap as long as we have robust stack GC,
                                        // or don't remove modules at all.
                                        self.modules[elem.idx].as_ref().unwrap().name,
                                        elem.port
                                    ),
                                    &format!("{}:{}", name, input),
                                ).expect("Failed to connect ports");
                        }
                        for output in &definition.outputs {
                            let element = Element {
                                idx,
                                port: output.to_owned(),
                            };
                            self.stack.push(element);
                        }
                    }
                    WordDefinition::Compound(definition) => {
                        self.eval(&definition.expansion, client, config)
                    }
                },
                None => {
                    error!("Word `{}` is not defined.", word);
                }
            }
        }
        let playback_ports = client.ports(
            Some("^system:playback_[0-9]+$"),
            None,
            jack::PortFlags::empty(),
        );
        for port in &playback_ports {
            client
                .disconnect(&client.port_by_name(port).unwrap())
                .expect("Failed to disconnect port");
        }
        if !self.stack.is_empty() {
            let top_module_idx = self.stack[self.stack.len() - 1].idx;
            let top_module_name = self.modules[top_module_idx]
                .as_ref()
                .unwrap()
                .name
                .to_owned();
            let out_ports = self
                .stack
                .iter()
                .rev()
                .take_while(|e| e.idx == top_module_idx)
                .map(|e| format!("{}:{}", top_module_name, e.port))
                .cycle();
            for (system_port, out_port) in playback_ports.iter().zip(out_ports.cycle()) {
                client
                    .connect_ports_by_name(&out_port, system_port)
                    .expect("Failed to connect ports");
            }
        }
        // TODO GC modules w/o connections
    }
}
