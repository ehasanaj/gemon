use crate::config::parser::GemonArgumentParser;
use crate::config::types::{GemonMethodType, GemonType};
use std::io;

use super::types::{GemonProjectScenario, MiscScenario};

#[derive(Debug)]
pub enum GemonArgument {
    Type(GemonType),
    Method { gemon_method_type: GemonMethodType },
    Uri(String),
    Header(String, String),
    Body(String),
    FormData(String, String),
    ResponseFilePath(Option<String>),
    ProjectSetup(GemonProjectScenario),
    MiscScenario(MiscScenario),
    LogResponse,
    AlsoPrintToTerminal,
    Secure,
}

impl Default for GemonArgument {
    fn default() -> Self {
        GemonArgument::ProjectSetup(GemonProjectScenario::Help)
    }
}

#[derive(Debug)]
pub struct GemonArguments {
    arguments: Vec<GemonArgument>,
}

impl Default for GemonArguments {
    fn default() -> Self {
        GemonArguments {
            arguments: vec![GemonArgument::default()],
        }
    }
}

impl GemonArguments {
    pub fn new(input_args: Vec<String>) -> Result<GemonArguments, io::Error> {
        if input_args.len() < 2 {
            return Ok(GemonArguments::default());
        }

        let arguments = input_args
            .into_iter()
            .map(|arg| arg.parse_argument())
            .filter(|arg| !arg.is_none())
            .map(|arg| arg.unwrap())
            .collect();

        let gemon_arguments = GemonArguments { arguments };

        Ok(gemon_arguments)
    }

    pub fn arguments(&self) -> &Vec<GemonArgument> {
        &self.arguments
    }
}
