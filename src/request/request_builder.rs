use super::rest_request::{GemonRestRequest, GemonRestRequestBuilder};
use crate::config::{types::GemonType, GemonConfig};
use bytes::Bytes;
use std::error::Error;

pub trait GemonRequest {
    async fn execute(&self) -> Result<GemonResponse, Box<dyn Error>>;
    fn json_metadata(&self) -> String;
    fn json_body(&self) -> String;
}

pub struct GemonResponse {
    data: Bytes,
}

impl GemonResponse {
    pub fn new(data: Bytes) -> GemonResponse {
        GemonResponse { data }
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }
}

pub struct RequestBuilder;

impl RequestBuilder {
    fn build_rest_request(config: &GemonConfig) -> Box<GemonRestRequest> {
        Box::new(
            GemonRestRequestBuilder::new()
                .set_gemon_method_type(config.gemon_method_type())
                .set_url(config.gemon_url())
                .set_headers(config.gemon_headers())
                .set_body(config.gemon_body())
                .set_form_data(config.gemon_form_data())
                .build(),
        )
    }

    pub fn build(config: &GemonConfig) -> Box<impl GemonRequest> {
        match config.gemon_type() {
            GemonType::REST => RequestBuilder::build_rest_request(config),
            GemonType::WEBSOCKET => todo!(),
            GemonType::PROTO => todo!(),
        }
    }
}
