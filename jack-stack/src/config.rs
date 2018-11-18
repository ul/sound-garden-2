//! # Config
//!
//! Structure which defines how words are interpreted in stack.
use fnv::FnvHashMap;

/// Define word which runs dedicated module.
#[derive(Deserialize)]
pub struct PrimitiveWord {
    /// Names of input ports of the module.
    pub inputs: Vec<String>,
    /// Names of output ports of the module.
    pub outputs: Vec<String>,
    /// Command to run the module.
    pub cmd: String,
    /// Argument to set JACK client name of the module.
    #[serde(default = "default_name_arg")]
    pub name_arg: String,
    /// Arguments which are set by constructing word with slashes,
    /// e.g. `delay/60` when `slash_args: ["--max-delay"]` would lead to passing
    /// `--max-delay 60` to the module command.
    /// TODO Support passing slash args positionally.
    pub slash_args: Option<Vec<String>>,
    /// Arbitrary arguments to pass to the module command.
    pub extra_args: Option<Vec<String>>,
}

/// Define word which is just a shortcut for series of other word,
/// e.g. `sin_osc` could be expanded as `phasor circle sin`.
#[derive(Deserialize)]
pub struct CompoundWord {
    /// How should the word be expanded.
    /// TODO Consider support for slash args templates.
    pub expansion: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum WordDefinition {
    Primitive(PrimitiveWord),
    Compound(CompoundWord),
}

pub type Vocabulary = FnvHashMap<String, WordDefinition>;

pub struct Config {
    pub words: Vocabulary,
}

fn default_name_arg() -> String {
    "--name".to_string()
}
