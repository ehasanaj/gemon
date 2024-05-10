use crate::EmptyResult;

#[derive(Debug)]
pub enum Form {
    Short,
    Long,
}

#[derive(Debug)]
pub enum GemonCommand {
    Invalid,
    Help,
    Init(String),
    PrintEnvAll(String),
    PrintEnv(String),
    PrintLastCall(String),
    TypeRest(String),
    TypeWebsocket(String),
    TypeProto(String),
    MethodGet(String),
    MethodPost(String),
    MethodDelete(String),
    MethodPut(String),
    MethodPatch(String),
    File(String),
    LogResponse(String),
    AlsoPrintToTerminal(String),
    Uri(String, Form),
    Header(String, Form),
    Body(String, Form),
    FormData(String, Form),
    ResponseFile(String, Form),
    Save(String, Form),
    Call(String, Form),
    SaveAndCall(String, Form),
    Delete(String, Form),
    RemoveEnv(String, Form),
    AddEnv(String, Form),
    RemoveEnvValue(String, Form),
    SelectEnv(String, Form),
}

impl From<String> for GemonCommand {
    fn from(cmd: String) -> Self {
        match cmd.as_str() {
            "-h" | "--help" => GemonCommand::Help,
            "init" => GemonCommand::Init(cmd),
            "print-env-all" => GemonCommand::PrintEnvAll(cmd),
            "print-env" => GemonCommand::PrintEnv(cmd),
            "print" => GemonCommand::PrintLastCall(cmd),
            "-t=REST" | "--type=REST" => GemonCommand::TypeRest(cmd),
            "-t=WEBSOCKET" | "--type=WEBSOCKET" => GemonCommand::TypeWebsocket(cmd),
            "-t=PROTO" | "--type=PROTO" => GemonCommand::TypeProto(cmd),
            "-m=GET" | "--method=GET" => GemonCommand::MethodGet(cmd),
            "-m=POST" | "--method=POST" => GemonCommand::MethodPost(cmd),
            "-m=DELETE" | "--method=DELETE" => GemonCommand::MethodDelete(cmd),
            "-m=PUT" | "--method=PUT" => GemonCommand::MethodPut(cmd),
            "-m=PATCH" | "--method=PATCH" => GemonCommand::MethodPatch(cmd),
            "-f" | "--file" => GemonCommand::File(cmd),
            "-l" | "--log" => GemonCommand::LogResponse(cmd),
            "-p" | "--print" => GemonCommand::AlsoPrintToTerminal(cmd),
            s if s.starts_with("-u=") => GemonCommand::Uri(cmd, Form::Short),
            s if s.starts_with("--uri=") => GemonCommand::Uri(cmd, Form::Long),
            s if s.starts_with("-h=") => GemonCommand::Header(cmd, Form::Short),
            s if s.starts_with("--header=") => GemonCommand::Header(cmd, Form::Long),
            s if s.starts_with("-b=") => GemonCommand::Body(cmd, Form::Short),
            s if s.starts_with("--body=") => GemonCommand::Body(cmd, Form::Long),
            s if s.starts_with("-fd=") => GemonCommand::FormData(cmd, Form::Short),
            s if s.starts_with("--form-data=") => GemonCommand::FormData(cmd, Form::Long),
            s if s.starts_with("-rf=") => GemonCommand::ResponseFile(cmd, Form::Short),
            s if s.starts_with("--response-file=") => GemonCommand::ResponseFile(cmd, Form::Long),
            s if s.starts_with("-s=") => GemonCommand::Save(cmd, Form::Short),
            s if s.starts_with("--save=") => GemonCommand::Save(cmd, Form::Long),
            s if s.starts_with("-c=") => GemonCommand::Call(cmd, Form::Short),
            s if s.starts_with("--call=") => GemonCommand::Call(cmd, Form::Short),
            s if s.starts_with("-sc=") => GemonCommand::SaveAndCall(cmd, Form::Long),
            s if s.starts_with("--save-and-call=") => GemonCommand::SaveAndCall(cmd, Form::Long),
            s if s.starts_with("-d=") => GemonCommand::Delete(cmd, Form::Short),
            s if s.starts_with("--delete=") => GemonCommand::Delete(cmd, Form::Long),
            s if s.starts_with("-ed=") => GemonCommand::RemoveEnv(cmd, Form::Short),
            s if s.starts_with("--env-delete=") => GemonCommand::RemoveEnv(cmd, Form::Long),
            s if s.starts_with("-e=") => GemonCommand::AddEnv(cmd, Form::Short),
            s if s.starts_with("--env=") => GemonCommand::AddEnv(cmd, Form::Long),
            s if s.starts_with("-edv=") => GemonCommand::RemoveEnvValue(cmd, Form::Short),
            s if s.starts_with("-env-delete-value=") => {
                GemonCommand::RemoveEnvValue(cmd, Form::Long)
            }
            s if s.starts_with("-se=") => GemonCommand::SelectEnv(cmd, Form::Short),
            s if s.starts_with("--select-env=") => GemonCommand::SelectEnv(cmd, Form::Long),
            _ => GemonCommand::Invalid,
        }
    }
}

impl GemonCommand {
    pub fn print_all() -> EmptyResult {
        println!("      ------------------Gemon Commands--------------------");
        GemonCommand::print_command("-h | --h", "Print the list of command options in terminal");
        GemonCommand::print_command("init", "Initialize current forlder into a gemon project");
        GemonCommand::print_command("print-env-all", "Print all environments with their assosiated variables");
        println!("      ------------------Gemon Commands--------------------");
        Ok(())
    }

    fn print_command(cmd: &str, description: &str) {
        println!("      {} => {}", cmd, description);
    }
}