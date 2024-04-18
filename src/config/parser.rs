use crate::config::arguments::GemonArgument;
use crate::config::types::{GemonMethodType, GemonType};

pub trait GemonArgumentParser {
    fn parse_argument(self) -> Option<GemonArgument>;
}

impl GemonArgumentParser for String {
    fn parse_argument(self) -> Option<GemonArgument> {
        let uri_parser = |s: &str, i: usize| {
            let uri = &s[i..];
            Some(GemonArgument::Uri(uri.to_string()))
        };
        match self.as_str() {
            "-t=REST" | "--type=REST" => Some(GemonArgument::Type(GemonType::REST)),
            "-t=WEBSOCKET" | "--type=WEBSOCKET" => Some(GemonArgument::Type(GemonType::WEBSOCKET)),
            "-t=PROTO" | "--type=PROTO" => Some(GemonArgument::Type(GemonType::PROTO)),
            "-m=GET" | "--method=GET" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::GET,
            }),
            "-m=POST" | "--method=POST" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::POST,
            }),
            "-m=DELETE" | "--method=DELETE" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::DELETE,
            }),
            "-m=PUT" | "--method=PUT" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::PUT,
            }),
            "-m=PATCH" | "--method=PATCH" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::PATCH,
            }),
            s if s.starts_with("-u=") => uri_parser(s, 3),
            s if s.starts_with("--uri=") => uri_parser(s, 6),
            _ => None,
        }
    }
}
