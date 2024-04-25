use project::Project;
use std::error::Error;

use crate::{
    config::{arguments::GemonArguments, GemonConfig},
    printer::PrinterBuilder,
    request_builder::{GemonRequest, RequestBuilder},
};

mod config;
mod constants;
mod printer;
mod project;
mod request_builder;

pub async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Parse arguments
    let gemon_arguments = GemonArguments::new(args)?;
    // Create configuration based on arguments
    let gemon_config = GemonConfig::new(&gemon_arguments)?;
    // Execute scenario based on config
    match gemon_config.gemon_scenario() {
        config::types::GemonScenario::Request => request_scenario(&gemon_config).await?,
        config::types::GemonScenario::ProjectSetup => Project::init()?, //TODO this has to be expanded later since there will be more actions to do upon a project other than init
    }

    Ok(())
}

async fn request_scenario(gemon_config: &GemonConfig) -> Result<(), Box<dyn Error>> {
    // Build the request
    let request = RequestBuilder::build(gemon_config);
    // Execute the request
    let response = request.execute().await?;
    // Build printer
    let printer = PrinterBuilder::build(gemon_config);
    // Print response
    printer.print(response.data())?;

    Ok(())
}
