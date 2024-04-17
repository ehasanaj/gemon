use crate::config::{
    arguments::{GemonArgument, GemonArguments},
    types::{GemonMethodType, GemonType},
};
use std::io;

pub mod arguments;
pub mod parser;
pub mod types;

#[derive(Debug)]
pub struct GemonConfig {
    gemon_type: GemonType,
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
}

impl GemonConfig {
    pub fn new(gemon_arguments: &GemonArguments) -> Result<GemonConfig, io::Error> {
        let mut gemon_type = GemonType::REST;
        let mut gemon_method_type = None;
        let mut url = None;

        gemon_arguments
            .arguments()
            .into_iter()
            .for_each(|argument| match argument {
                GemonArgument::GemonType(t) => gemon_type = t.clone(),
                GemonArgument::Method {
                    gemon_method_type: t,
                } => gemon_method_type = Some(t.clone()),
                GemonArgument::Uri(t) => url = Some(String::from(t)),
            });

        let config = GemonConfig {
            gemon_type,
            gemon_method_type,
            url: url,
        };
        Ok(config)
    }

    pub fn gemon_type(&self) -> &GemonType {
        &self.gemon_type
    }

    pub fn gemon_method_type(&self) -> GemonMethodType {
        self.gemon_method_type.expect("Gemon Method Type missing")
    }

    pub fn gemon_url(&self) -> String {
        String::from(self.url.as_ref().expect("Url expected"))
    }
}
