//! # Stack
//!
//! Manage JACK clients and connections with style using a simple stack-based language.
//!
//! TODO Always connect to the system:playback_* ports via [-1, 1] clip module to protect hardware and ears.

use config::{Config, WordDefinition};
use fnv::FnvHashSet;
use module::Module;

pub struct Stack {
    /// Track module's inputs, immediate and transitive.
    /// Used by Stack GC to drop modules which are neither on stack nor inputs of modules on stack.
    connections: Vec<Option<FnvHashSet<usize>>>,
    /// Collection of used modules.
    /// When module is not used anymore, just replace its vec entry with `None`
    /// and corresponding process will be killed.
    /// This is not the most compact way to bookkeep modules, but it's simple
    /// and in practice not that wasteful (even thousands of `None`s accumulating during session
    /// are nothing in comparison with one-minute delay buffer for example).
    modules: Vec<Option<Module>>,
    /// Stack of ports. Top ones belonging to the same module are connected to the
    /// system:playback_*. When the new word is evaluated its module inputs are consumed from stack
    /// and its outputs are put back to stack.
    stack: Vec<Element>,
}

/// Stack element corresponding to specific output port.
#[derive(Clone)]
struct Element {
    /// Index of the module to which this port belongs to.
    idx: usize,
    /// Name of the port (without module name).
    port: String,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            connections: Vec::new(),
            modules: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Evaluate `s` and manage JACK clients and connections via `client` according to `config`.
    pub fn eval(&mut self, s: &str, client: &jack::Client, config: &Config) {
        self.eval_internal(s, client, config);
        self.reset_system_playback(client);
        self.collect_garbage();
    }

    fn eval_internal(&mut self, s: &str, client: &jack::Client, config: &Config) {
        for token in s.split_whitespace() {
            debug!("Token: {}", token);
            let mut token = token.to_string();
            if token.parse::<f64>().is_ok() {
                token = format!("constant/{}", token);
            }
            let args = token.split('/').collect::<Vec<_>>();
            let word = args[0];
            match word {
                "clear" => {
                    self.stack.clear();
                }
                // a -> ()
                "pop" => {
                    self.stack.pop();
                }
                // a -> a a
                "dup" => {
                    if let Some(top_element) = &self.stack.last().cloned() {
                        self.stack.push(top_element.clone());
                    }
                }
                // a b -> b a
                "swap" => {
                    let len = self.stack.len();
                    if len < 2 {
                        warn!("Not enough minerals.");
                        return;
                    }
                    self.stack.swap(len - 2, len - 1);
                }
                // a b c -> b c a
                "rot" => {
                    let len = self.stack.len();
                    if len < 3 {
                        warn!("You require more Vespene gas.");
                        return;
                    }
                    self.stack.swap(len - 2, len - 1);
                    self.stack.swap(len - 3, len - 1);
                }
                _ => self.eval_custom_word(&token, client, config),
            }
        }
    }

    fn eval_custom_word(&mut self, token: &str, client: &jack::Client, config: &Config) {
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
                    let mut connections: FnvHashSet<usize> = FnvHashSet::default();
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
                        connections.extend(self.connections[elem.idx].as_ref().unwrap());
                        connections.insert(elem.idx);
                    }
                    self.connections.push(Some(connections));
                    for output in &definition.outputs {
                        let element = Element {
                            idx,
                            port: output.to_owned(),
                        };
                        self.stack.push(element);
                    }
                }
                WordDefinition::Compound(definition) => {
                    self.eval_internal(&definition.expansion, client, config)
                }
            },
            None => {
                error!("Word `{}` is not defined.", word);
            }
        }
    }

    fn reset_system_playback(&self, client: &jack::Client) {
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
        if let Some(top_module_idx) = self.stack.last().and_then(|e| Some(e.idx)) {
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
    }

    fn collect_garbage(&mut self) {
        let all_modules = self
            .modules
            .iter()
            .enumerate()
            .filter_map(|(i, m)| m.as_ref().map(|_| i))
            .collect::<FnvHashSet<_>>();

        let mut stack_modules: FnvHashSet<usize> = FnvHashSet::default();
        for e in &self.stack {
            stack_modules.extend(self.connections[e.idx].as_ref().unwrap());
            stack_modules.insert(e.idx);
        }

        let garbage = &all_modules - &stack_modules;
        if !garbage.is_empty() {
            debug!("Collecting garbage: {:?}", garbage);
        }
        for idx in garbage {
            self.connections[idx] = None;
            self.modules[idx] = None;
        }
    }
}
