use crate::{
    config::{arguments::GemonArguments, GemonConfig},
    request_manager::{GemonRequest, RequestManager},
};

mod config;
mod request_manager;

pub async fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let gemon_arguments = GemonArguments::new(args)?;
    let gemon_config = GemonConfig::new(gemon_arguments)?;
    let request = RequestManager::build_request(&gemon_config);
    request.execute().await?;
    Ok(())
}
