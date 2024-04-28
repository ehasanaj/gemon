use crate::config::{arguments::GemonArguments, types::GemonScenario, GemonConfig};
use project::Project;
use request::Request;
use std::error::Error;

mod config;
mod constants;
mod printer;
mod project;
mod request;

type EmptyResult = Result<(), Box<dyn Error>>;

pub async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Parse arguments
    let gemon_arguments = GemonArguments::new(args)?;
    // Create configuration based on arguments
    let gemon_config = GemonConfig::new(&gemon_arguments)?;
    // Execute scenario based on config
    match gemon_config.gemon_scenario() {
        GemonScenario::Request => Request::execute(&gemon_config).await,
        GemonScenario::Project(project_scenario) => {
            Project::execute(&gemon_config, project_scenario).await
        }
    }
}
