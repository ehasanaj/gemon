use crate::config::{types::GemonType, GemonConfig};

use self::rest_request::GemonRestRequestBuilder;

mod rest_request;

pub trait GemonRequest {
    async fn execute(&self) -> Result<(), Box<dyn std::error::Error>>; //TODO: return GemonResponse here instead
}
pub trait GemonResponse {}

pub struct RequestManager {}

impl RequestManager {
    pub fn build_request(gemon_config: &GemonConfig) -> Box<impl GemonRequest> {
        match gemon_config.gemon_type() {
            GemonType::REST => Box::new(
                GemonRestRequestBuilder::new()
                    .set_gemon_method_type(gemon_config.gemon_method_type())
                    .set_url(gemon_config.gemon_url())
                    .build(),
            ),
            GemonType::WEBSOCKET => todo!(),
            GemonType::PROTO => todo!(),
        }
    }
}
