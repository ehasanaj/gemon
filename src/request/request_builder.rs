use super::rest_request::{GemonRestRequest, GemonRestRequestBuilder};
use crate::{
    config::{types::GemonType, GemonConfig},
    constants::AUTHORIZATION,
    project::project_handler::authorization,
};
use bytes::Bytes;
use std::collections::HashMap;
use std::error::Error;

pub trait GemonRequest {
    async fn execute(&self) -> Result<GemonResponse, Box<dyn Error>>;
    fn json_metadata(&self) -> String;
    fn json_body(&self) -> String;
    fn request_type(&self) -> String;
    fn set_body(&mut self, body: Option<String>);
}

pub struct GemonResponse {
    data: Bytes,
    status: u16,
    headers: HashMap<String, String>,
}

impl GemonResponse {
    pub fn new(data: Bytes, status: u16, headers: HashMap<String, String>) -> GemonResponse {
        GemonResponse {
            data,
            status,
            headers,
        }
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

pub struct RequestBuilder;

impl RequestBuilder {
    fn build_rest_request(config: &GemonConfig) -> Box<GemonRestRequest> {
        let mut headers = config.gemon_headers().clone();
        if config.gemon_secure() && !headers.contains_key(AUTHORIZATION) {
            if let Some(authorization) = authorization() {
                headers.insert(AUTHORIZATION.to_string(), authorization.to_string());
            }
        }
        Box::new(
            GemonRestRequestBuilder::new()
                .set_gemon_method_type(config.gemon_method_type())
                .set_url(config.gemon_url())
                .set_headers(&headers)
                .set_body(config.gemon_body())
                .set_form_data(config.gemon_form_data())
                .build(),
        )
    }

    fn build_rest_request_from_string(content: &str) -> Box<GemonRestRequest> {
        Box::new(GemonRestRequestBuilder::build_from_string(content))
    }

    pub fn build_from_string(content: &str, request_type: &str) -> Box<impl GemonRequest> {
        match request_type {
            "REST" => RequestBuilder::build_rest_request_from_string(content),
            "WEBSOCKET" => todo!(),
            "PROTO" => todo!(),
            _ => panic!("Invalid request_type marker"),
        }
    }

    pub fn build(config: &GemonConfig) -> Box<impl GemonRequest> {
        match config.gemon_type() {
            GemonType::Rest => RequestBuilder::build_rest_request(config),
            GemonType::Websocket => todo!(),
            GemonType::Proto => todo!(),
        }
    }
}
