use crate::config::parser::GemonArgumentParser;
use crate::config::types::{GemonMethodType, GemonType};
use std::io;

#[derive(Debug)]
pub enum GemonArgument {
    Type(GemonType),
    Method { gemon_method_type: GemonMethodType },
    Uri(String),
    Header(String, String),
}

#[derive(Debug)]
pub struct GemonArguments {
    arguments: Vec<GemonArgument>,
}

impl GemonArguments {
    pub fn new(input_args: Vec<String>) -> Result<GemonArguments, io::Error> {
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