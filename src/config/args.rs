use crate::config::parser::Arg;
use std::io;

#[derive(Debug)]
pub enum GemonType {
    REST,
    WEBSOCKET,
    PROTO,
}

#[derive(Debug)]
pub enum MethodType {
    GET,
    POST,
    DELETE,
    PUT,
}

#[derive(Debug)]
pub enum Argument {
    GemonType(GemonType),
    Method { method_type: MethodType },
    Url(String),
}

#[derive(Debug)]
pub struct Arguments {
    args: Vec<Argument>,
}

impl Arguments {
    pub fn parse(input_args: Vec<String>) -> Result<(), io::Error> {
        let mut args: Vec<Argument> = vec![];

        let method_or_type = input_args.get(1).parse_method_or_type();
        args.push(method_or_type);
        let url = input_args.get(2).parse_path();
        args.push(url);

        let arguments = Arguments { args };

        println!("{:?}", arguments);

        Ok(())
    }
}
