//! # Connection manager
//!
//! Wraps a non-active (without DSP process) JACK client dedicated to other clients
//! connections management.
use gatekeeper::Gatekeeper;

pub struct Manager {
    /// JACK client instance used to manage other clients connections.
    client: jack::Client,
    /// Proxy JACK client for post-processing and isolating Stack outputs.
    gatekeeper: Gatekeeper,
}

impl Manager {
    /// Connect to the JACK and setup Gatekeeper.
    pub fn new() -> Self {
        let gatekeeper = Gatekeeper::new();

        // Create JACK client to manage JACK graph.
        // USE_EXACT_NAME name used to prevent two jack-stack clients managing the same server instance.
        // To support multiple jack-stack clients we need at least:
        // * Use unique suffixes in module names (so if you eval `440 s` and `440 s` in both clients
        //   it wouldn't crash trying to create two `constant_440_0` modules in JACK).
        let (client, _status) =
            jack::Client::new("jack-stack", jack::ClientOptions::USE_EXACT_NAME)
                .expect("Failed to connect to JACK.");

        // Connect gatekeeper to the system output.
        // NOTE This relies on the assumption that system output ports are named
        // system:playback_1, system:playback_2 and so on.
        for (i, output) in gatekeeper.outputs.iter().enumerate() {
            client
                .connect_ports_by_name(output, &format!("system:playback_{}", i + 1))
                .expect("Failed to connect ports");
        }

        Manager { client, gatekeeper }
    }

    /// Test if the given port belongs to an active client.
    pub fn output_port_is_ready(&self, port: &str) -> bool {
        if self
            .connect_ports(port, &self.gatekeeper.dummy_input)
            .is_ok()
        {
            self.client
                .disconnect_ports_by_name(port, &self.gatekeeper.dummy_input)
                .expect("Failed to disconnect from dummy port.");
            true
        } else {
            false
        }
    }

    /// Count how many ports satisfy given regular expression.
    pub fn count_ports(&self, p: &str) -> usize {
        self.client
            .ports(Some(p), None, jack::PortFlags::empty())
            .len()
    }

    /// Connect ports by name.
    pub fn connect_ports(&self, a: &str, b: &str) -> Result<(), jack::Error> {
        self.client.connect_ports_by_name(a, b)
    }

    /// Reset gatekeeper inputs to the given outputs.
    /// Outputs are cycled, e.g. if gatekeeper has two inputs and only one output is given then it
    /// will be connected to both inputs.
    pub fn reset_outputs<T>(&self, outputs: T)
    where
        T: Clone + Iterator<Item = String>,
    {
        for port in &self.gatekeeper.inputs {
            self.client
                .disconnect(&self.client.port_by_name(port).unwrap())
                .expect("Failed to disconnect port");
        }
        for (output, input) in outputs.cycle().zip(self.gatekeeper.inputs.iter()) {
            self.connect_ports(&output, input)
                .expect("Failed to connect ports");
        }
    }
}
