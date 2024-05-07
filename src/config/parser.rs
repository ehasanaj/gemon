use crate::config::arguments::GemonArgument;
use crate::config::types::{GemonMethodType, GemonProjectScenario, GemonType};

fn simple_arg_parser(s: &str, i: usize) -> String {
    let arg = &s[i..];
    arg.to_string()
}

fn key_value_pair_arg_parser(s: &str, i: usize) -> (String, String) {
    let key_value = &s[i..];
    let arg: Vec<&str> = key_value.split("::").collect();
    let key = arg
        .first()
        .expect("arg key not provided correctly e.x `-h=key:value`")
        .to_string();
    let value = arg
        .get(1)
        .expect("arg value not provided correctly e.x `-h=key:value`")
        .to_string();
    (key, value)
}

fn triple_value_arg_parser(s: &str, i: usize) -> (String, String, String) {
    let group = &s[i..];
    let arg: Vec<&str> = group.split("::").collect();
    let one = arg
        .first()
        .expect("arg one not provided correctly for triple touple e.x `-[e]=one:two:three`")
        .to_string();
    let two = arg
        .get(1)
        .expect("arg two not provided correctly for triple touple e.x `-[e]=one:two:three`")
        .to_string();
    let three = arg
        .get(2)
        .expect("arg three not provided correctly for triple touple e.x `-[e]=one:two:three`")
        .to_string();
    (one, two, three)
}

pub trait GemonArgumentParser {
    fn parse_argument(self) -> Option<GemonArgument>;
}

impl GemonArgumentParser for String {
    fn parse_argument(self) -> Option<GemonArgument> {
        match self.as_str() {
            "init" => Some(GemonArgument::ProjectSetup(GemonProjectScenario::Init)),
            "print-env-all" => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintEnvAll,
            )),
            "print-env" => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintEnv,
            )),
            "print" => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintLastCall,
            )),
            "-t=REST" | "--type=REST" => Some(GemonArgument::Type(GemonType::Rest)),
            "-t=WEBSOCKET" | "--type=WEBSOCKET" => Some(GemonArgument::Type(GemonType::Websocket)),
            "-t=PROTO" | "--type=PROTO" => Some(GemonArgument::Type(GemonType::Proto)),
            "-m=GET" | "--method=GET" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Get,
            }),
            "-m=POST" | "--method=POST" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Post,
            }),
            "-m=DELETE" | "--method=DELETE" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Delete,
            }),
            "-m=PUT" | "--method=PUT" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Put,
            }),
            "-m=PATCH" | "--method=PATCH" => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Patch,
            }),
            "-f" => Some(GemonArgument::ResponseFilePath(None)),
            "--file" => Some(GemonArgument::ResponseFilePath(None)),
            "-l" => Some(GemonArgument::LogResponse),
            "--log" => Some(GemonArgument::LogResponse),
            "-p" => Some(GemonArgument::AlsoPrintToTerminal),
            "--print" => Some(GemonArgument::AlsoPrintToTerminal),
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
            s if s.starts_with("-rf=") => Some(GemonArgument::ResponseFilePath(Some(
                simple_arg_parser(s, 4),
            ))),
            s if s.starts_with("--response-file=") => Some(GemonArgument::ResponseFilePath(Some(
                simple_arg_parser(s, 16),
            ))),
            s if s.starts_with("-s=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Save(simple_arg_parser(s, 3)),
            )),
            s if s.starts_with("--save=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Save(simple_arg_parser(s, 7)),
            )),
            s if s.starts_with("-c=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Call(simple_arg_parser(s, 3)),
            )),
            s if s.starts_with("--call=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Call(simple_arg_parser(s, 7)),
            )),
            s if s.starts_with("-sc=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SaveAndCall(simple_arg_parser(s, 3)),
            )),
            s if s.starts_with("--save-and-call=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SaveAndCall(simple_arg_parser(s, 7)),
            )),
            s if s.starts_with("-d=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Delete(simple_arg_parser(s, 3)),
            )),
            s if s.starts_with("--delete=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Delete(simple_arg_parser(s, 9)),
            )),
            s if s.starts_with("-ed=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::RemoveEnv(simple_arg_parser(s, 4)),
            )),
            s if s.starts_with("--env-delete=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::RemoveEnv(simple_arg_parser(s, 13)),
            )),
            s if s.starts_with("-e=") => {
                let (one, two, three) = triple_value_arg_parser(s, 3);
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::AddEnv(
                    one, two, three,
                )))
            }
            s if s.starts_with("--env=") => {
                let (one, two, three) = triple_value_arg_parser(s, 6);
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::AddEnv(
                    one, two, three,
                )))
            }
            s if s.starts_with("-edv=") => {
                let (one, two) = key_value_pair_arg_parser(s, 5);
                Some(GemonArgument::ProjectSetup(
                    GemonProjectScenario::RemoveEnvValue(one, two),
                ))
            }
            s if s.starts_with("-env-delete-value=") => {
                let (one, two) = key_value_pair_arg_parser(s, 18);
                Some(GemonArgument::ProjectSetup(
                    GemonProjectScenario::RemoveEnvValue(one, two),
                ))
            }
            s if s.starts_with("-se=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SelectEnv(simple_arg_parser(s, 4)),
            )),
            s if s.starts_with("--select-env=") => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SelectEnv(simple_arg_parser(s, 13)),
            )),
            _ => None,
        }
    }
}
