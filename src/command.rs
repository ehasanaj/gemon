use colored::Colorize;

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
    Version,
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
    AddAuthorization(String, Form),
    RemoveAuthorization,
    Secure,
}

impl From<String> for GemonCommand {
    fn from(cmd: String) -> Self {
        match cmd.as_str() {
            "-h" | "--help" => GemonCommand::Help,
            "-v" | "--version" => GemonCommand::Version,
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
            "-sec" | "--secure" => GemonCommand::Secure,
            "-r-auth" | "--remove-authorization" => GemonCommand::RemoveAuthorization,
            s if s.starts_with("-auth=") => GemonCommand::AddAuthorization(cmd, Form::Short),
            s if s.starts_with("--authorization=") => {
                GemonCommand::AddAuthorization(cmd, Form::Long)
            }
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
        let info_separator = "------------------Gemon Info--------------------";
        println!("{}", info_separator.red().bold());
        GemonCommand::print_info("* In the commands displayed below the parenthesis () are to show that instead of the values show here you would provide your own values. The parenthesis themselves are also not needed");
        GemonCommand::print_info("* If you need to pass values that have spaces you need to enclose the whole value in quotation marks \" or '");
        GemonCommand::print_info("* When saving a rest response which accepts a request body, inside the folder with the name of the request an empty body.json will be created where you can put the request body if needed");
        GemonCommand::print_info("* Gemon allows env variables to be saved and then used, if an variabled called 'base_uri' is saved it can be used: -u={base_uri}/path .Env variables can be used in uri, headers, form data and request body");
        let commands_separator = "------------------Gemon Commands--------------------";
        println!("{}", commands_separator.red().bold());
        GemonCommand::print_command(
            "-h | --help",
            "Print the list of command options in terminal",
        );
        GemonCommand::print_command("-v | --version", "Pring gemon version info");
        GemonCommand::print_command("init", "Initialize current folder into a gemon project");
        GemonCommand::print_command(
            "print-env-all",
            "Print all environments with their assosiated variables",
        );
        GemonCommand::print_command("print-env", "Print values of the current environment");
        GemonCommand::print_command(
            "print",
            "Print the last call response that was stored in the file",
        );
        GemonCommand::print_command("-t=(REST | WEBSOCKET | PROTO)", "Set the type of request");
        GemonCommand::print_command(
            "-m=(GET | POST | DELETE | PUT | PATCH)",
            "Set the type of REST method | Required: -t=REST",
        );
        GemonCommand::print_command(
            "-f | --file",
            "Save the response of the call to the default response.json file",
        );
        GemonCommand::print_command(
            "-l | --log",
            "If set the response file name is tagged with the timestamp",
        );
        GemonCommand::print_command(
            "-p | --print",
            "Save the response to the response file but also print it to terminal",
        );
        GemonCommand::print_command(
            "-u=(https://api.com:8080) | --uri=(https://api.com:8080)",
            "Set the URI of the request",
        );
        GemonCommand::print_command(
            "-h=(key::value) | --header=(key::value)",
            "Set a header to the request",
        );
        GemonCommand::print_command(
            "-b=('{\"name\": \"some name\"}') | --body=('{\"name\": \"some name\"}')",
            "Set the body of the request",
        );
        GemonCommand::print_command(
            "-fd=(key:value) | --form-data=(key:value)",
            "Set a form data parameter to the request",
        );
        GemonCommand::print_command(
            "-rf=(file_name.json) | --response-file=(file_name.json)",
            "Print the response to the provided file name",
        );
        GemonCommand::print_command("-sec | --secure", "Mark request that it needs to be authorized, authorization is either taken from project or from specific header");
        GemonCommand::print_command(
            "-s | --save",
            "Save request into the project so it can be called later",
        );
        GemonCommand::print_command(
            "-c=(login) | --call=(login)",
            "Calls a previously saved request by providing the name with which it was saved",
        );
        GemonCommand::print_command(
            "-sc=(login) | --save-and-call=(login)",
            "Simultaneusly it saves a new request and calles it",
        );
        GemonCommand::print_command(
            "-d=(login) | --delete=(login)",
            "Removes a previously saved request",
        );
        GemonCommand::print_command("-e=(int::base_uri::https://api.com) | --env=(int::base_uri::https://api.com)", "Saves a new env value into the project. If the env on which the new value it is being saved does not exist the environment is also created");
        GemonCommand::print_command(
            "-ed=(int) | --env-delete=(int)",
            "Removes a previously saved environment",
        );
        GemonCommand::print_command(
            "-edv=(int::key) | --env-delete-value=(int::key)",
            "Removes an env variable from one of the environments",
        );
        GemonCommand::print_command(
            "-se=(int) | --select-env=(int)",
            "Set an previously created environment as the current environment",
        );
        GemonCommand::print_command("-r-auth | --remove-authorization", "Removes authorziation set for current environment, if no environemnt is set removes the default authoriziation used without environment");
        GemonCommand::print_command("-auth='Bearer token...' | --authorization='Bearer token...'", "Set authorization for current environment, if no environment is selected it set the default authorization without environment");

        Ok(())
    }

    fn print_command(cmd: &str, description: &str) {
        println!("{} => {}", cmd.green().bold(), description.blue().italic());
    }

    fn print_info(info: &str) {
        println!("{}", info.yellow().italic());
    }
}
