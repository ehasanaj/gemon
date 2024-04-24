use crate::{
    config::{arguments::GemonArguments, GemonConfig},
    printer::PrinterBuilder,
    request_builder::{GemonRequest, RequestBuilder},
};

mod config;
mod constants;
mod printer;
mod request_builder;

pub async fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let gemon_arguments = GemonArguments::new(args)?;
    // Create configuration based on arguments
    let gemon_config = GemonConfig::new(gemon_arguments)?;
    // Build the request
    let request = RequestBuilder::build(&gemon_config);
    // Execute the request
    let response = request.execute().await?;
    // Build printer
    let printer = PrinterBuilder::build(&gemon_config);
    // Print response
    printer.print(response.data())?;

    Ok(())
}
