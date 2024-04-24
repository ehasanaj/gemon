use serde_json::Value;

use crate::{
    config::{arguments::GemonArguments, GemonConfig},
    request_builder::{GemonRequest, RequestBuilder},
};

mod config;
mod request_builder;
mod constants;

pub async fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let gemon_arguments = GemonArguments::new(args)?;
    let gemon_config = GemonConfig::new(gemon_arguments)?;
    let request = RequestBuilder::build(&gemon_config);
    let response = request.execute().await?;
    
    //TODO insert below logic in a printer functionality
    let response_value: Value = serde_json::from_slice(&response.data())?;
    let pretty_response = serde_json::to_string_pretty(&response_value)?;
    println!("{}", pretty_response);
    
    Ok(())
}
