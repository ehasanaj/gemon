use crate::config::{types::GemonType, GemonConfig};

use self::rest_request::GemonRestRequestBuilder;

mod rest_request;

pub trait GemonRequest {
    async fn execute(&self) -> Result<(), Box<dyn std::error::Error>>; //TODO: return GemonResponse here instead
}
pub trait GemonResponse {}

pub struct RequestManager {}

impl RequestManager {
    fn build_rest_request(gemon_config: &GemonConfig) -> Box<impl GemonRequest> {
        Box::new(
            GemonRestRequestBuilder::new()
                .set_gemon_method_type(gemon_config.gemon_method_type())
                .set_url(gemon_config.gemon_url())
                .set_headers(gemon_config.gemon_headers())
                .set_body(gemon_config.gemon_body())
                .set_form_data(gemon_config.gemon_form_data())
                .build(),
        )
    }

    pub fn build_request(gemon_config: &GemonConfig) -> Box<impl GemonRequest> {
        match gemon_config.gemon_type() {
            GemonType::REST => RequestManager::build_rest_request(gemon_config),
            GemonType::WEBSOCKET => todo!(),
            GemonType::PROTO => todo!(),
        }
    }
}
