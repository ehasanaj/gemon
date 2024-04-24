use crate::config::arguments::GemonArgument;
use crate::config::types::{GemonMethodType, GemonType};

fn simple_arg_parser(s: &str, i: usize) -> String {
    let arg = &s[i..];
    arg.to_string()
}

fn key_value_pair_arg_parser(s: &str, i: usize) -> (String, String) {
    let key_value = &s[i..];
    let arg: Vec<&str> = key_value.split(':').collect();
    let key = arg
        .get(0)
        .expect("arg key not provided correctly e.x `-h=key:value`")
        .to_string();
    let value = arg
        .get(1)
        .expect("arg value not provided correctly e.x `-h=key:value`")
        .to_string();
    (key, value)
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
            s if s.starts_with("-u=") => Some(GemonArgument::Uri(simple_arg_parser(s, 3))),
            s if s.starts_with("--uri=") => Some(GemonArgument::Uri(simple_arg_parser(s, 6))),
            s if s.starts_with("-h=") => {
                let arg = key_value_pair_arg_parser(s, 3);
                Some(GemonArgument::Header(arg.0, arg.1))
            }
            s if s.starts_with("--header=") => {
                let arg = key_value_pair_arg_parser(s, 9);
                Some(GemonArgument::Header(arg.0, arg.1))
            }
            s if s.starts_with("-b=") => Some(GemonArgument::Body(simple_arg_parser(s, 3))),
            s if s.starts_with("--body=") => Some(GemonArgument::Body(simple_arg_parser(s, 6))),
            s if s.starts_with("-fd=") => {
                let arg = key_value_pair_arg_parser(s, 4);
                Some(GemonArgument::FormData(arg.0, arg.1))
            }
            s if s.starts_with("--form-data=") => {
                let arg = key_value_pair_arg_parser(s, 12);
                Some(GemonArgument::FormData(arg.0, arg.1))
            }
            _ => None,
        }
    }
}
