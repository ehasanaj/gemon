use crate::config::{
    arguments::{GemonArgument, GemonArguments},
    types::{GemonMethodType, GemonPrinter, GemonScenario, GemonType},
};
use std::{collections::HashMap, io};

pub mod arguments;
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
}

impl GemonConfigBuilder {
    fn new() -> GemonConfigBuilder {
        GemonConfigBuilder {
            gemon_scenario: GemonScenario::Request,
            gemon_type: GemonType::REST,
            gemon_method_type: None,
            url: None,
            headers: HashMap::new(),
            body: None,
            form_data: HashMap::new(),
            response_file_path: None,
            write_to_request_response_file: false,
        }
    }

    fn process_argument(&mut self, argument: &GemonArgument) {
        match argument {
            GemonArgument::Type(t) => self.gemon_type = t.clone(),
            GemonArgument::Method {
                gemon_method_type: t,
            } => self.gemon_method_type = Some(t.clone()),
            GemonArgument::Uri(t) => self.url = Some(t.to_string()),
            GemonArgument::Header(key, value) => {
                self.headers.insert(key.clone(), value.clone());
            }
            GemonArgument::Body(b) => self.body = Some(b.to_string()),
            GemonArgument::FormData(key, value) => {
                self.form_data.insert(key.clone(), value.clone());
            }
            GemonArgument::ResponseFilePath(f) => match f {
                Some(path) => self.response_file_path = Some(path.to_owned()),
                None => self.write_to_request_response_file = true,
            },
            GemonArgument::ProjectSetup(scenario) => {
                self.gemon_scenario = GemonScenario::Project(scenario.clone())
            }
        }
    }

    fn build_response_file_path(
        write_to_request_response_file: bool,
        response_file_path: &Option<String>,
        gemon_scenario: &GemonScenario,
    ) -> Option<String> {
        match write_to_request_response_file {
            true => match gemon_scenario {
                GemonScenario::Project(project_scenario) => match project_scenario {
                    types::GemonProjectScenario::Call(name) => {
                        Some(format!("{name}/response.json"))
                    }
                    _ => None,
                },
                _ => None,
            },
            false => response_file_path.to_owned(),
        }
    }

    fn build(self) -> GemonConfig {
        let path = GemonConfigBuilder::build_response_file_path(
            self.write_to_request_response_file,
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
            response_file_path: path.unwrap_or_default(),
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
    response_file_path: String,
}

impl GemonConfig {
    pub fn new(gemon_arguments: &GemonArguments) -> Result<GemonConfig, io::Error> {
        let mut builder = GemonConfigBuilder::new();

        gemon_arguments
            .arguments()
            .into_iter()
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
        self.gemon_method_type.unwrap_or(GemonMethodType::GET)
    }

    pub fn gemon_url(&self) -> String {
        String::from(self.url.clone().unwrap_or_default())
    }

    pub fn gemon_headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn gemon_body(&self) -> Option<String> {
        self.body.clone()
    }

    pub fn gemon_form_data(&self) -> &HashMap<String, String> {
        &self.form_data
    }

    pub fn gemon_response_file_path(&self) -> String {
        self.response_file_path.clone()
    }
}
