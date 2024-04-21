use crate::config::arguments::GemonArgument;
use crate::config::types::{GemonMethodType, GemonType};

fn uri_parser(s: &str, i: usize) -> Option<GemonArgument> {
    let uri = &s[i..];
    Some(GemonArgument::Uri(uri.to_string()))
}

fn header_parser(s: &str, i: usize) -> Option<GemonArgument> {
    let header_key_value = &s[i..];
    let header: Vec<&str> = header_key_value.split(':').collect();
    let key = header
        .get(0)
        .expect("header key not provided correctly e.x `-h=key:value`")
        .to_string();
    let value = header
        .get(1)
        .expect("header value not provided correctly e.x `-h=key:value`")
        .to_string();
    Some(GemonArgument::Header(key, value))
}

pub trait GemonArgumentParser {
    fn parse_argument(self) -> Option<GemonArgument>;
}

impl GemonArgumentParser for String {
    fn parse_argument(self) -> Option<GemonArgument> {
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
            s if s.starts_with("-h=") => header_parser(s, 3),
            s if s.starts_with("--header=") => header_parser(s, 9),
            _ => None,
        }
    }
}
