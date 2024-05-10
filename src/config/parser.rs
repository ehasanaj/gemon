use crate::command::{Form, GemonCommand};
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
        let cmd: GemonCommand = self.into();
        match cmd {
            GemonCommand::Help => Some(GemonArgument::ProjectSetup(GemonProjectScenario::Help)),
            GemonCommand::Init(_) => Some(GemonArgument::ProjectSetup(GemonProjectScenario::Init)),
            GemonCommand::PrintEnvAll(_) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintEnvAll,
            )),
            GemonCommand::PrintEnv(_) => {
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::PrintEnv))
            }
            GemonCommand::PrintLastCall(_) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintLastCall,
            )),
            GemonCommand::TypeRest(_) => Some(GemonArgument::Type(GemonType::Rest)),
            GemonCommand::TypeWebsocket(_) => Some(GemonArgument::Type(GemonType::Websocket)),
            GemonCommand::TypeProto(_) => Some(GemonArgument::Type(GemonType::Proto)),
            GemonCommand::MethodGet(_) => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Get,
            }),
            GemonCommand::MethodPost(_) => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Post,
            }),
            GemonCommand::MethodDelete(_) => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Delete,
            }),
            GemonCommand::MethodPut(_) => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Put,
            }),
            GemonCommand::MethodPatch(_) => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Patch,
            }),
            GemonCommand::File(_) => Some(GemonArgument::ResponseFilePath(None)),
            GemonCommand::LogResponse(_) => Some(GemonArgument::LogResponse),
            GemonCommand::AlsoPrintToTerminal(_) => Some(GemonArgument::AlsoPrintToTerminal),
            GemonCommand::Uri(s, Form::Short) => Some(GemonArgument::Uri(simple_arg_parser(&s, 3))),
            GemonCommand::Uri(s, Form::Long) => Some(GemonArgument::Uri(simple_arg_parser(&s, 6))),
            GemonCommand::Header(s, Form::Short) => {
                let arg = key_value_pair_arg_parser(&s, 3);
                Some(GemonArgument::Header(arg.0, arg.1))
            }
            GemonCommand::Header(s, Form::Long) => {
                let arg = key_value_pair_arg_parser(&s, 9);
                Some(GemonArgument::Header(arg.0, arg.1))
            }
            GemonCommand::Body(s, Form::Short) => {
                Some(GemonArgument::Body(simple_arg_parser(&s, 3)))
            }
            GemonCommand::Body(s, Form::Long) => {
                Some(GemonArgument::Body(simple_arg_parser(&s, 6)))
            }
            GemonCommand::FormData(s, Form::Short) => {
                let arg = key_value_pair_arg_parser(&s, 4);
                Some(GemonArgument::FormData(arg.0, arg.1))
            }
            GemonCommand::FormData(s, Form::Long) => {
                let arg = key_value_pair_arg_parser(&s, 12);
                Some(GemonArgument::FormData(arg.0, arg.1))
            }
            GemonCommand::ResponseFile(s, Form::Short) => Some(GemonArgument::ResponseFilePath(
                Some(simple_arg_parser(&s, 4)),
            )),
            GemonCommand::ResponseFile(s, Form::Long) => Some(GemonArgument::ResponseFilePath(
                Some(simple_arg_parser(&s, 16)),
            )),
            GemonCommand::Save(s, Form::Short) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Save(simple_arg_parser(&s, 3)),
            )),
            GemonCommand::Save(s, Form::Long) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Save(simple_arg_parser(&s, 7)),
            )),
            GemonCommand::Call(s, Form::Short) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Call(simple_arg_parser(&s, 3)),
            )),
            GemonCommand::Call(s, Form::Long) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Call(simple_arg_parser(&s, 7)),
            )),
            GemonCommand::SaveAndCall(s, Form::Short) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SaveAndCall(simple_arg_parser(&s, 3)),
            )),
            GemonCommand::SaveAndCall(s, Form::Long) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SaveAndCall(simple_arg_parser(&s, 7)),
            )),
            GemonCommand::Delete(s, Form::Short) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Delete(simple_arg_parser(&s, 3)),
            )),
            GemonCommand::Delete(s, Form::Long) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Delete(simple_arg_parser(&s, 9)),
            )),
            GemonCommand::RemoveEnv(s, Form::Short) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::RemoveEnv(simple_arg_parser(&s, 4)),
            )),
            GemonCommand::RemoveEnv(s, Form::Long) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::RemoveEnv(simple_arg_parser(&s, 13)),
            )),
            GemonCommand::AddEnv(s, Form::Short) => {
                let (one, two, three) = triple_value_arg_parser(&s, 3);
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::AddEnv(
                    one, two, three,
                )))
            }
            GemonCommand::AddEnv(s, Form::Long) => {
                let (one, two, three) = triple_value_arg_parser(&s, 6);
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::AddEnv(
                    one, two, three,
                )))
            }
            GemonCommand::RemoveEnvValue(s, Form::Short) => {
                let (one, two) = key_value_pair_arg_parser(&s, 5);
                Some(GemonArgument::ProjectSetup(
                    GemonProjectScenario::RemoveEnvValue(one, two),
                ))
            }
            GemonCommand::RemoveEnvValue(s, Form::Long) => {
                let (one, two) = key_value_pair_arg_parser(&s, 18);
                Some(GemonArgument::ProjectSetup(
                    GemonProjectScenario::RemoveEnvValue(one, two),
                ))
            }
            GemonCommand::SelectEnv(s, Form::Short) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SelectEnv(simple_arg_parser(&s, 4)),
            )),
            GemonCommand::SelectEnv(s, Form::Long) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SelectEnv(simple_arg_parser(&s, 13)),
            )),
            GemonCommand::Invalid => None,
        }
    }
}
