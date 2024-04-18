use reqwest;
use serde_json::Value;

use super::GemonRequest;
use crate::config::types::GemonMethodType;

pub struct GemonRestRequestBuilder {
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
}

impl GemonRestRequestBuilder {
    pub fn new() -> GemonRestRequestBuilder {
        GemonRestRequestBuilder {
            gemon_method_type: None,
            url: None,
        }
    }

    pub fn set_gemon_method_type(
        self,
        gemon_method_type: GemonMethodType,
    ) -> GemonRestRequestBuilder {
        GemonRestRequestBuilder {
            gemon_method_type: Some(gemon_method_type),
            ..self
        }
    }

    pub fn set_url(self, url: String) -> GemonRestRequestBuilder {
        GemonRestRequestBuilder {
            url: Some(String::from(url)),
            ..self
        }
    }

    pub fn build(&self) -> GemonRestRequest {
        GemonRestRequest {
            gemon_method_type: self
                .gemon_method_type
                .expect("Method Type missing when building Rest request!"),
            uri: String::from(
                self.url
                    .as_ref()
                    .expect("Uri missing when building Rest request!"),
            ),
        }
    }
}

#[derive(Debug)]
pub struct GemonRestRequest {
    gemon_method_type: GemonMethodType,
    uri: String,
}

impl GemonRequest for GemonRestRequest {
    async fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let request = match self.gemon_method_type {
            GemonMethodType::GET => client.get(&self.uri),
            GemonMethodType::POST => client.post(&self.uri),
            GemonMethodType::DELETE => client.delete(&self.uri),
            GemonMethodType::PUT => client.put(&self.uri),
            GemonMethodType::PATCH => client.patch(&self.uri),
        };

        let response_bytes = request.send().await?.bytes().await?;

        let response: Value = serde_json::from_slice(&response_bytes)?;
        let pretty_response = serde_json::to_string_pretty(&response)?;

        println!("{}", pretty_response);

        Ok(())
    }
}
