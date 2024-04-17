use crate::config::parser::Arg;
use crate::config::types::{GemonMethodType, GemonType};
use std::io;

use super::parser::MethodOrType;

#[derive(Debug)]
pub enum GemonArgument {
    GemonType(GemonType),
    Method { gemon_method_type: GemonMethodType },
    Uri(String),
}

#[derive(Debug)]
pub struct GemonArguments {
    arguments: Vec<GemonArgument>,
}

impl GemonArguments {
    pub fn new(input_args: Vec<String>) -> Result<GemonArguments, io::Error> {
        let mut arguments: Vec<GemonArgument> = vec![];

        let mut next_index = 1;
        let method_or_type = input_args.get(next_index).parse_method_or_type();
        arguments.push(method_or_type.0);

        match method_or_type.1 {
            MethodOrType::Type => {
                next_index += 1;
                let method = input_args.get(next_index).parse_method_or_type();
                match method.1 {
                    MethodOrType::Method => {
                        arguments.push(method.0)
                    },
                    MethodOrType::Type => panic!("After the type argument there should be a method argument and not twice the type"),
                }
            }
            MethodOrType::Method => arguments.push(GemonArgument::GemonType(GemonType::REST)),
        }

        next_index += 1;
        let url = input_args.get(next_index).parse_path();
        arguments.push(url);

        let gemon_arguments = GemonArguments { arguments };

        Ok(gemon_arguments)
    }

    pub fn arguments(&self) -> &Vec<GemonArgument> {
        &self.arguments
    }
}
