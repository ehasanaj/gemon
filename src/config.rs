use crate::config::{
    arguments::{GemonArgument, GemonArguments},
    types::{GemonMethodType, GemonType},
};
use std::{collections::HashMap, io};

pub mod arguments;
pub mod parser;
pub mod types;

struct GemonConfigBuilder {
    gemon_type: GemonType,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    form_data: HashMap<String, String>,
}

impl GemonConfigBuilder {
    fn new() -> GemonConfigBuilder {
        GemonConfigBuilder {
            gemon_type: GemonType::REST,
            gemon_method_type: None,
            url: None,
            headers: HashMap::new(),
            body: None,
            form_data: HashMap::new(),
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
        }
    }

    fn build(self) -> GemonConfig {
        GemonConfig {
            gemon_type: self.gemon_type,
            gemon_method_type: self.gemon_method_type,
            url: self.url,
            headers: self.headers,
            body: self.body,
            form_data: self.form_data,
        }
    }
}

#[derive(Debug)]
pub struct GemonConfig {
    gemon_type: GemonType,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    form_data: HashMap<String, String>,
}

impl GemonConfig {
    pub fn new(gemon_arguments: GemonArguments) -> Result<GemonConfig, io::Error> {
        let mut builder = GemonConfigBuilder::new();

        gemon_arguments
            .arguments()
            .into_iter()
            .for_each(|argument| builder.process_argument(argument));

        let config = builder.build();
        Ok(config)
    }

    pub fn gemon_type(&self) -> &GemonType {
        &self.gemon_type
    }

    pub fn gemon_method_type(&self) -> GemonMethodType {
        self.gemon_method_type
            .expect("-t=[type] or --type=[type] expected")
    }

    pub fn gemon_url(&self) -> String {
        String::from(self.url.as_ref().expect("-u=[uri] or --uri=[uri] expected"))
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
}
