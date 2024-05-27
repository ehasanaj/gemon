use crate::config::{arguments::GemonArguments, types::GemonScenario, GemonConfig};
use config::effector::Effector;
use misc::Misc;
use project::Project;
use request::Request;
use std::error::Error;

mod command;
mod config;
mod constants;
mod printer;
mod project;
mod request;
mod misc;

type EmptyResult = Result<(), Box<dyn Error>>;

pub async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Apply env
    let args = Effector::apply_env_to_args(args);
    // Parse arguments
    let gemon_arguments = GemonArguments::new(args)?;
    // Create configuration based on arguments
    let gemon_config = GemonConfig::new(&gemon_arguments)?;
    // Execute scenario based on config
    match gemon_config.gemon_scenario() {
        GemonScenario::Request => Request::execute(&gemon_config).await,
        GemonScenario::Project(project_scenario) => {
            Project::execute(&gemon_config, &project_scenario).await
        }
        GemonScenario::Misc(misc_scenario) => Misc::execute(misc_scenario) 
    }
}
