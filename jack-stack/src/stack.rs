//! # Stack
//!
//! Manage JACK clients and connections with style using a simple stack-based language.

use config::{Config, WordDefinition};
use fnv::FnvHashSet;
use manager::Manager;
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
    /// Full name of the port (with module name).
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

    /// Evaluate `s` by tossing the stack, spawning required modules and making required connections.
    /// Then connect top module on the stack to the system playback.
    pub fn eval(&mut self, s: &str, manager: &Manager, config: &Config) {
        self.eval_internal(s, manager, config);
        self.reset_system_playback(manager);
        self.collect_garbage();
    }

    /// Evaluate `s` by tossing the stack, spawning required modules and making required connections.
    fn eval_internal(&mut self, s: &str, manager: &Manager, config: &Config) {
        for token in s.split_whitespace() {
            debug!("Token: {}", token);
            let mut token = token.to_string();
            // `constant` word can have custom definition, but its semantics are reserved.
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
                    if let Some(top_element) = self.stack.last().cloned() {
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
                _ => self.eval_custom_word(&token, manager, config),
            }
        }
    }

    /// Evaluate token by spawning required module and making required connections for primitive
    /// word, expand and evaluate compound one.
    fn eval_custom_word(&mut self, token: &str, manager: &Manager, config: &Config) {
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
                    let module = Module::spawn(manager, definition, &name, &args[1..]);
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
                        manager
                            .connect_ports(&elem.port, &format!("{}:{}", name, input))
                            .expect("Failed to connect ports");
                        connections.extend(self.connections[elem.idx].as_ref().unwrap());
                        connections.insert(elem.idx);
                    }
                    self.connections.push(Some(connections));
                    for output in &definition.outputs {
                        let element = Element {
                            idx,
                            port: format!("{}:{}", name, output),
                        };
                        self.stack.push(element);
                    }
                }
                WordDefinition::Compound(definition) => {
                    self.eval_internal(&definition.expansion, manager, config)
                }
            },
            None => {
                error!("Word `{}` is not defined.", word);
            }
        }
    }

    /// Connect ports on the top of the stack belonging to the same module to the system playback.
    fn reset_system_playback(&self, manager: &Manager) {
        if let Some(top_module_idx) = self.stack.last().and_then(|e| Some(e.idx)) {
            let outputs = self
                .stack
                .iter()
                .rev()
                .take_while(|e| e.idx == top_module_idx)
                .map(|e| e.port.to_owned());
            manager.reset_outputs(outputs);
        }
    }

    /// Drop modules which are not connected to the stack.
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
