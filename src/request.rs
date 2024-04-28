use self::request_builder::RequestBuilder;
use crate::{config::GemonConfig, printer::PrinterBuilder, EmptyResult};
use request_builder::GemonRequest;

pub mod request_builder;
pub mod rest_request;

pub struct Request;

impl Request {
    pub async fn call(request: Box<impl GemonRequest>, config: &GemonConfig) -> EmptyResult {
        // Execute the request
        let response = request.execute().await?;
        // Build printer
        let printer = PrinterBuilder::build(config);
        // Print response
        printer.print(response.data()).map_err(|err| err.into())
    }

    pub async fn execute(config: &GemonConfig) -> EmptyResult {
        // Build the request
        let request = RequestBuilder::build(config);
        // Call request
        Request::call(request, config).await
    }
}
