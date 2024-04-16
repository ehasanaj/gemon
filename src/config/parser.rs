use crate::config::args::Argument;
use crate::config::args::GemonType;
use crate::config::args::MethodType;

pub trait Arg {
    fn parse_method_or_type(&self) -> Argument;
    fn parse_path(&self) -> Argument;
}

impl Arg for Option<&String> {
    fn parse_method_or_type(&self) -> Argument {
        match *self {
            Some(arg) => match arg.as_str() {
                "GET" => Argument::Method { method_type: MethodType::GET },
                "POST" => Argument::Method { method_type: MethodType::POST },
                "DELETE" => Argument::Method { method_type: MethodType::DELETE },
                "PUT" => Argument::Method { method_type: MethodType::PUT },
                "gt=REST" => Argument::GemonType(GemonType::REST),
                "gt=WEBSOCKET" => Argument::GemonType(GemonType::WEBSOCKET),
                "gt=PROTO" => Argument::GemonType(GemonType::PROTO),
                _ =>  panic!("Argument `{}` invalid, expected method or type either REST method type like `GET`, `POST` or gt=[`REST`, `WEBSOCKET` or `PROTO`]", arg),
            },
            None => panic!("Argument missing expected either REST method type like `GET`, `POST` or gt=[`REST`, `WEBSOCKET` or `PROTO`]"),
        }
    }
    
    fn parse_path(&self) -> Argument {
        match *self {
            Some(arg) => Argument::Url(arg.clone()),
            None => panic!("Argument missing, URL expected"),
        }
    }
}
