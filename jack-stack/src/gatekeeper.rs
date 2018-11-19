//! # Gatekeeper
//!
//! Specialized JACK client to manage jack-stack outputs.
//! Provides hard clipping to protect hardware and ears, dummy port to test readyness of other
//! clients, and isolates outputs reset.

pub struct Gatekeeper {
    /// Dummy input used to test other ports for being active.
    ///
    /// What problem does it solve?
    ///
    /// When Module spawns a module associated with a primitive word, that module creates JACK
    /// client, then registers ports, and then activates the client. When connecting ports JACK
    /// requires both source and destination to belong to active clients, otherwise it returns an
    /// error. Thus Module must wait until module will activate its client before returning to Stack
    /// which will connect module's ports to ports on the stack. The problem is that JACK doesn't
    /// expose an API to know when client is active.
    ///
    /// What is an implemented solution?
    ///
    /// When Module spawns a module it polls for module's ports to be registered first, and then
    /// polls trying to connect one of modules output ports to the dummy_input port. The moment it
    /// succeeds it means that module's client is active and ports are ready to be used by Stack.
    pub dummy_input: String,
    /// Full names of Gatekeeper's input ports.
    /// Stack output should be connected to those ports instead of using system:playback_* ports
    /// directly for two reasons:
    /// * Gatekeeper clips signal (and Stack outputs crazy amplitude sometimes!).
    /// * It's easier to reset Stack outputs without interference to connections made by other
    /// instances of jack-stack, other programs or user themselves.
    pub inputs: Vec<String>,
    /// Full names of Gatekeeper's output ports.
    /// To simplify things they are made public and are managed by Manager, not Gatekeeper itself.
    pub outputs: Vec<String>,
    /// JACK client instance with inputs -> clip -> outputs process.
    client: Option<jack::AsyncClient<(), ProcessHandler>>,
}

impl Gatekeeper {
    pub fn new() -> Self {
        let (client, _status) =
            jack::Client::new("jack-stack-output", jack::ClientOptions::empty())
                .expect("Failed to connect to JACK");

        let name = client.name().to_owned();

        // NOTE This relies on the assumption that system output ports are named
        // system:playback_1, system:playback_2 and so on.
        let playback_ports_count = client
            .ports(
                Some("^system:playback_[0-9]+$"),
                None,
                jack::PortFlags::empty(),
            ).len();

        let mut inputs = Vec::new();
        let mut input_names = Vec::new();
        let mut outputs = Vec::new();
        let mut output_names = Vec::new();
        // NOTE This relies on the assumption that system output ports are named
        // system:playback_1, system:playback_2 and so on.
        for i in 1..=playback_ports_count {
            let input_name = format!("input_{}", i);
            let input = client
                .register_port(&input_name, jack::AudioIn::default())
                .expect("Failed to register input port");
            inputs.push(input);
            input_names.push(format!("{}:{}", name, input_name));

            let output_name = format!("output_{}", i);
            let output = client
                .register_port(&output_name, jack::AudioOut::default())
                .expect("Failed to register output port");
            outputs.push(output);
            output_names.push(format!("{}:{}", name, output_name));
        }

        client
            .register_port("dummy_input", jack::AudioIn::default())
            .expect("Failed to register input port");

        let process = ProcessHandler { inputs, outputs };
        let client = Some(client.activate_async((), process).unwrap());

        Gatekeeper {
            dummy_input: format!("{}:dummy_input", name),
            inputs: input_names,
            outputs: output_names,
            client,
        }
    }
}

impl Drop for Gatekeeper {
    fn drop(&mut self) {
        self.client.take().unwrap().deactivate().unwrap();
    }
}

/// Clip signal to the [-1, 1] interval. Safety measure to protect hardware and ears.
fn clip(x: f32) -> f32 {
    x.max(-1.0).min(1.0)
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        // ♥♥♥ Rust iterators!
        let inputs = self.inputs.iter().flat_map(|inputs| inputs.as_slice(ps));
        let outputs = self
            .outputs
            .iter_mut()
            .flat_map(|outputs| outputs.as_mut_slice(ps));
        for (output, input) in outputs.zip(inputs) {
            *output = clip(*input);
        }
        jack::Control::Continue
    }
}

struct ProcessHandler {
    pub inputs: Vec<jack::Port<jack::AudioIn>>,
    pub outputs: Vec<jack::Port<jack::AudioOut>>,
}
