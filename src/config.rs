use chrono::Local;

use crate::config::{
    arguments::{GemonArgument, GemonArguments},
    types::{GemonMethodType, GemonPrinter, GemonScenario, GemonType},
};
use std::{collections::HashMap, io};

use self::types::GemonProjectScenario;

pub mod arguments;
pub mod effector;
pub mod parser;
pub mod types;

pub struct GemonConfigBuilder {
    gemon_scenario: GemonScenario,
    gemon_type: GemonType,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    form_data: HashMap<String, String>,
    write_to_request_response_file: bool,
    response_file_path: Option<String>,
    log_response: bool,
    also_print_to_terminal: bool,
}

impl GemonConfigBuilder {
    fn new() -> GemonConfigBuilder {
        GemonConfigBuilder {
            gemon_scenario: GemonScenario::Request,
            gemon_type: GemonType::Rest,
            gemon_method_type: None,
            url: None,
            headers: HashMap::new(),
            body: None,
            form_data: HashMap::new(),
            response_file_path: None,
            write_to_request_response_file: false,
            log_response: false,
            also_print_to_terminal: false,
        }
    }

    fn process_argument(&mut self, argument: &GemonArgument) {
        match argument {
            GemonArgument::Type(t) => self.gemon_type = *t,
            GemonArgument::Method {
                gemon_method_type: t,
            } => self.gemon_method_type = Some(*t),
            GemonArgument::Uri(t) => self.url = Some(t.to_string()),
            GemonArgument::Header(key, value) => {
                self.headers.insert(key.into(), value.into());
            }
            GemonArgument::Body(b) => self.body = Some(b.to_string()),
            GemonArgument::FormData(key, value) => {
                self.form_data.insert(key.into(), value.into());
            }
            GemonArgument::ResponseFilePath(f) => match f {
                Some(path) => self.response_file_path = Some(path.to_owned()),
                None => self.write_to_request_response_file = true,
            },
            GemonArgument::ProjectSetup(scenario) => {
                self.gemon_scenario = GemonScenario::Project(scenario.clone())
            }
            GemonArgument::LogResponse => self.log_response = true,
            GemonArgument::AlsoPrintToTerminal => self.also_print_to_terminal = true,
            GemonArgument::MiscScenario(scenario) => {
                self.gemon_scenario = GemonScenario::Misc(scenario.clone())
            } 
        }
    }

    fn build_response_file_path(
        write_to_request_response_file: bool,
        log_response: bool,
        response_file_path: &Option<String>,
        gemon_scenario: &GemonScenario,
    ) -> Option<String> {
        match write_to_request_response_file {
            true => match gemon_scenario {
                GemonScenario::Project(GemonProjectScenario::Call(name)) if log_response => {
                    let now = Local::now();
                    Some(format!(
                        "{name}/response_{}.json",
                        now.format("%Y_%m_%d_%H_%M_%S")
                    ))
                }
                GemonScenario::Project(GemonProjectScenario::Call(name)) if !log_response => {
                    Some(format!("{name}/response.json"))
                }
                _ => None,
            },
            false => response_file_path.to_owned(),
        }
    }

    fn build(self) -> GemonConfig {
        let path = GemonConfigBuilder::build_response_file_path(
            self.write_to_request_response_file,
            self.log_response,
            &self.response_file_path,
            &self.gemon_scenario,
        );

        GemonConfig {
            gemon_scenario: self.gemon_scenario,
            gemon_type: self.gemon_type,
            gemon_printer: match path {
                Some(_) => GemonPrinter::File,
                None => GemonPrinter::Terminal,
            },
            gemon_method_type: self.gemon_method_type,
            url: self.url,
            headers: self.headers,
            body: self.body,
            form_data: self.form_data,
            response_file_path: path,
            also_print_to_terminal: self.also_print_to_terminal,
        }
    }
}

#[derive(Debug)]
pub struct GemonConfig {
    gemon_scenario: GemonScenario,
    gemon_type: GemonType,
    gemon_printer: GemonPrinter,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    form_data: HashMap<String, String>,
    response_file_path: Option<String>,
    also_print_to_terminal: bool,
}

impl GemonConfig {
    pub fn new(gemon_arguments: &GemonArguments) -> Result<GemonConfig, io::Error> {
        let mut builder = GemonConfigBuilder::new();

        gemon_arguments
            .arguments()
            .iter()
            .for_each(|argument| builder.process_argument(argument));

        let config = builder.build();
        Ok(config)
    }

    pub fn gemon_scenario(&self) -> &GemonScenario {
        &self.gemon_scenario
    }

    pub fn gemon_type(&self) -> &GemonType {
        &self.gemon_type
    }

    pub fn gemon_printer(&self) -> &GemonPrinter {
        &self.gemon_printer
    }

    pub fn gemon_method_type(&self) -> GemonMethodType {
        self.gemon_method_type.unwrap_or(GemonMethodType::Get)
    }

    pub fn gemon_url(&self) -> String {
        self.url.to_owned().unwrap_or_default()
    }

    pub fn gemon_headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn gemon_body(&self) -> Option<String> {
        self.body.to_owned()
    }

    pub fn gemon_form_data(&self) -> &HashMap<String, String> {
        &self.form_data
    }

    pub fn gemon_response_file_path(&self) -> Option<String> {
        self.response_file_path.to_owned()
    }

    pub fn gemon_also_print_to_terminal(&self) -> bool {
        self.also_print_to_terminal
    }
}
