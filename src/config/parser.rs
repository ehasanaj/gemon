use crate::command::GemonCommand;
use crate::config::arguments::GemonArgument;
use crate::config::types::{GemonMethodType, GemonProjectScenario, GemonType};

use super::types::MiscScenario;

/// Returns everything after the `=` that ends the flag, e.g. `-u=http://a?b=c` -> `http://a?b=c`.
fn arg_value(s: &str) -> &str {
    s.split_once('=').map(|(_, value)| value).unwrap_or_default()
}

fn simple_arg_parser(s: &str) -> String {
    arg_value(s).to_string()
}

fn key_value_pair_arg_parser(s: &str) -> (String, String) {
    let key_value = arg_value(s);
    let arg: Vec<&str> = key_value.split("::").collect();
    let key = arg
        .first()
        .expect("arg key not provided correctly e.x `-h=key::value`")
        .to_string();
    let value = arg
        .get(1)
        .expect("arg value not provided correctly e.x `-h=key::value`")
        .to_string();
    (key, value)
}

fn triple_value_arg_parser(s: &str) -> (String, String, String) {
    let group = arg_value(s);
    let arg: Vec<&str> = group.split("::").collect();
    let one = arg
        .first()
        .expect("arg one not provided correctly for triple touple e.x `-[e]=one::two::three`")
        .to_string();
    let two = arg
        .get(1)
        .expect("arg two not provided correctly for triple touple e.x `-[e]=one::two::three`")
        .to_string();
    let three = arg
        .get(2)
        .expect("arg three not provided correctly for triple touple e.x `-[e]=one::two::three`")
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
            GemonCommand::Version => Some(GemonArgument::MiscScenario(MiscScenario::Version)),
            GemonCommand::Init => Some(GemonArgument::ProjectSetup(GemonProjectScenario::Init)),
            GemonCommand::ImportOpenApi => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::ImportOpenApi,
            )),
            GemonCommand::PrintEnvAll => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintEnvAll,
            )),
            GemonCommand::PrintEnv => {
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::PrintEnv))
            }
            GemonCommand::PrintLastCall => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::PrintLastCall,
            )),
            GemonCommand::TypeRest => Some(GemonArgument::Type(GemonType::Rest)),
            GemonCommand::TypeWebsocket => Some(GemonArgument::Type(GemonType::Websocket)),
            GemonCommand::TypeProto => Some(GemonArgument::Type(GemonType::Proto)),
            GemonCommand::MethodGet => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Get,
            }),
            GemonCommand::MethodPost => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Post,
            }),
            GemonCommand::MethodDelete => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Delete,
            }),
            GemonCommand::MethodPut => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Put,
            }),
            GemonCommand::MethodPatch => Some(GemonArgument::Method {
                gemon_method_type: GemonMethodType::Patch,
            }),
            GemonCommand::File => Some(GemonArgument::ResponseFilePath(None)),
            GemonCommand::LogResponse => Some(GemonArgument::LogResponse),
            GemonCommand::AlsoPrintToTerminal => Some(GemonArgument::AlsoPrintToTerminal),
            GemonCommand::Uri(s) => Some(GemonArgument::Uri(simple_arg_parser(&s))),
            GemonCommand::Header(s) => {
                let (key, value) = key_value_pair_arg_parser(&s);
                Some(GemonArgument::Header(key, value))
            }
            GemonCommand::Body(s) => Some(GemonArgument::Body(simple_arg_parser(&s))),
            GemonCommand::FormData(s) => {
                let (key, value) = key_value_pair_arg_parser(&s);
                Some(GemonArgument::FormData(key, value))
            }
            GemonCommand::ResponseFile(s) => {
                Some(GemonArgument::ResponseFilePath(Some(simple_arg_parser(&s))))
            }
            GemonCommand::Save(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Save(simple_arg_parser(&s)),
            )),
            GemonCommand::Call(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Call(simple_arg_parser(&s)),
            )),
            GemonCommand::SaveAndCall(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SaveAndCall(simple_arg_parser(&s)),
            )),
            GemonCommand::Delete(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::Delete(simple_arg_parser(&s)),
            )),
            GemonCommand::RemoveEnv(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::RemoveEnv(simple_arg_parser(&s)),
            )),
            GemonCommand::AddEnv(s) => {
                let (one, two, three) = triple_value_arg_parser(&s);
                Some(GemonArgument::ProjectSetup(GemonProjectScenario::AddEnv(
                    one, two, three,
                )))
            }
            GemonCommand::RemoveEnvValue(s) => {
                let (one, two) = key_value_pair_arg_parser(&s);
                Some(GemonArgument::ProjectSetup(
                    GemonProjectScenario::RemoveEnvValue(one, two),
                ))
            }
            GemonCommand::SelectEnv(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::SelectEnv(simple_arg_parser(&s)),
            )),
            GemonCommand::Invalid => None,
            GemonCommand::AddAuthorization(s) => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::AddAuthorization(simple_arg_parser(&s)),
            )),
            GemonCommand::RemoveAuthorization => Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::RemoveAuthorization,
            )),
            GemonCommand::Secure => Some(GemonArgument::Secure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GemonArgumentParser;
    use crate::config::{arguments::GemonArgument, types::GemonProjectScenario};

    fn parse(arg: &str) -> Option<GemonArgument> {
        String::from(arg).parse_argument()
    }

    fn project_scenario(arg: &str) -> GemonProjectScenario {
        match parse(arg) {
            Some(GemonArgument::ProjectSetup(scenario)) => scenario,
            other => panic!("expected project scenario for {arg}, got {other:?}"),
        }
    }

    #[test]
    fn short_and_long_request_name_flags_parse_the_same_name() {
        for arg in ["-c=login", "--call=login"] {
            assert!(matches!(project_scenario(arg), GemonProjectScenario::Call(name) if name == "login"));
        }
        for arg in ["-sc=login", "--save-and-call=login"] {
            assert!(
                matches!(project_scenario(arg), GemonProjectScenario::SaveAndCall(name) if name == "login")
            );
        }
        for arg in ["-s=login", "--save=login"] {
            assert!(matches!(project_scenario(arg), GemonProjectScenario::Save(name) if name == "login"));
        }
        for arg in ["-d=login", "--delete=login"] {
            assert!(matches!(project_scenario(arg), GemonProjectScenario::Delete(name) if name == "login"));
        }
    }

    #[test]
    fn env_flags_parse_short_and_long_forms() {
        for arg in ["-edv=int::key", "--env-delete-value=int::key", "-env-delete-value=int::key"] {
            assert!(matches!(
                project_scenario(arg),
                GemonProjectScenario::RemoveEnvValue(env, key) if env == "int" && key == "key"
            ));
        }
        for arg in ["-e=int::base_uri::http://a", "--env=int::base_uri::http://a"] {
            assert!(matches!(
                project_scenario(arg),
                GemonProjectScenario::AddEnv(env, key, value)
                    if env == "int" && key == "base_uri" && value == "http://a"
            ));
        }
        for arg in ["-se=int", "--select-env=int"] {
            assert!(matches!(project_scenario(arg), GemonProjectScenario::SelectEnv(env) if env == "int"));
        }
        for arg in ["-ed=int", "--env-delete=int"] {
            assert!(matches!(project_scenario(arg), GemonProjectScenario::RemoveEnv(env) if env == "int"));
        }
        for arg in ["-auth=Bearer a=b", "--authorization=Bearer a=b"] {
            assert!(matches!(
                project_scenario(arg),
                GemonProjectScenario::AddAuthorization(value) if value == "Bearer a=b"
            ));
        }
    }

    #[test]
    fn values_keep_equals_signs_after_the_flag() {
        assert!(matches!(
            parse("--uri=http://api.test/items?a=1&b=2"),
            Some(GemonArgument::Uri(uri)) if uri == "http://api.test/items?a=1&b=2"
        ));
        assert!(matches!(
            parse("-h=X-Token::a=b"),
            Some(GemonArgument::Header(key, value)) if key == "X-Token" && value == "a=b"
        ));
        assert!(matches!(
            parse("--form-data=name::value"),
            Some(GemonArgument::FormData(key, value)) if key == "name" && value == "value"
        ));
        assert!(matches!(
            parse("-rf=out.json"),
            Some(GemonArgument::ResponseFilePath(Some(path))) if path == "out.json"
        ));
    }

    #[test]
    fn parses_openapi_import_command() {
        let argument = String::from("import-openapi").parse_argument();

        assert!(matches!(
            argument,
            Some(GemonArgument::ProjectSetup(
                GemonProjectScenario::ImportOpenApi
            ))
        ));
    }
}
