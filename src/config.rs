use crate::config::{
    arguments::{GemonArgument, GemonArguments},
    types::{GemonMethodType, GemonType},
};
use std::io;

pub mod arguments;
pub mod parser;
pub mod types;

struct GemonConfigBuilder {
    gemon_type: GemonType,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
}

impl GemonConfigBuilder {
    fn new() -> GemonConfigBuilder {
        GemonConfigBuilder {
            gemon_type: GemonType::REST,
            gemon_method_type: None,
            url: None,
        }
    }

    fn process_argument(&mut self, argument: &GemonArgument) {
        match argument {
            GemonArgument::Type(t) => self.gemon_type = t.clone(),
            GemonArgument::Method {
                gemon_method_type: t,
            } => self.gemon_method_type = Some(t.clone()),
            GemonArgument::Uri(t) => self.url = Some(String::from(t)),
        }
    }

    fn build(self) -> GemonConfig {
        GemonConfig {
            gemon_type: self.gemon_type,
            gemon_method_type: self.gemon_method_type,
            url: self.url,
        }
    }
}

#[derive(Debug)]
pub struct GemonConfig {
    gemon_type: GemonType,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
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
}
