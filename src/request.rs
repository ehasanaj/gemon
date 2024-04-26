use std::error::Error;
use crate::{config::GemonConfig, printer::PrinterBuilder};

use self::request_builder::RequestBuilder;
use request_builder::GemonRequest;

mod request_builder;
pub mod rest_request;

pub struct Request;

impl Request {
    async fn call(request: Box<impl GemonRequest>, config: &GemonConfig) -> Result<(), Box<dyn Error>> {
        // Execute the request
        let response = request.execute().await?;
        // Build printer
        let printer = PrinterBuilder::build(config);
        // Print response
        printer.print(response.data()).map_err(|err| Box::new(err) as Box<dyn Error>)
    }

    pub async fn execute(config: &GemonConfig) -> Result<(), Box<dyn Error>> {
        // Build the request
        let request = RequestBuilder::build(config);
        // Call request
        Request::call(request, config).await
    }

    pub async fn save(config: &GemonConfig) -> Result<(), Box<dyn Error>> {
        // Build the request
        let request = RequestBuilder::build(config);

        //TODO Save request in folder if it does not exist, if exists update with the new changes

        // Call request
        Request::call(request, config).await
    }
}