use crate::config::arguments::GemonArgument;
use crate::config::types::{GemonMethodType, GemonType};

pub trait Arg {
    fn parse_method_or_type(&self) -> (GemonArgument, MethodOrType);
    fn parse_path(&self) -> GemonArgument;
}

#[derive(PartialEq, Eq)]
pub enum MethodOrType {
    Method,
    Type,
}

impl Arg for Option<&String> {
    fn parse_method_or_type(&self) -> (GemonArgument, MethodOrType) {
        match *self {
            Some(arg) => match arg.as_str() {
                "GET" => (GemonArgument::Method { gemon_method_type: GemonMethodType::GET }, MethodOrType::Method),
                "POST" => (GemonArgument::Method { gemon_method_type: GemonMethodType::POST }, MethodOrType::Method),
                "DELETE" => (GemonArgument::Method { gemon_method_type: GemonMethodType::DELETE }, MethodOrType::Method),
                "PUT" => (GemonArgument::Method { gemon_method_type: GemonMethodType::PUT }, MethodOrType::Method),
                "REST" => (GemonArgument::GemonType(GemonType::REST), MethodOrType::Type),
                "WEBSOCKET" => (GemonArgument::GemonType(GemonType::WEBSOCKET), MethodOrType::Type),
                "PROTO" => (GemonArgument::GemonType(GemonType::PROTO), MethodOrType::Type),
                _ =>  panic!("Argument `{}` invalid, expected method or type either REST method type like `GET`, `POST` or Reqest Type `REST`, `WEBSOCKET`, `PROTO`", arg),
            },
            None => panic!("Argument missing expected either REST method type like `GET`, `POST` or gt=[`REST`, `WEBSOCKET` or `PROTO`]"),
        }
    }

    fn parse_path(&self) -> GemonArgument {
        match *self {
            Some(arg) => GemonArgument::Uri(arg.clone()),
            None => panic!("Argument missing, URL expected"),
        }
    }
}
