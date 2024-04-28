use super::request_builder::{GemonRequest, GemonResponse};
use crate::config::types::GemonMethodType;
use crate::constants;
use reqwest::{
    self,
    header::{self, HeaderMap, ACCEPT, CONTENT_TYPE},
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

trait HeaderMapConverter {
    fn to_header_map(self) -> HeaderMap;
}

impl HeaderMapConverter for HashMap<String, String> {
    fn to_header_map(self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in self {
            headers.insert(
                header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                header::HeaderValue::from_str(&value).unwrap(),
            );
        }
        headers
    }
}

pub struct GemonRestRequestBuilder {
    gemon_method_type: Option<GemonMethodType>,
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    form_data: HashMap<String, String>,
}

impl GemonRestRequestBuilder {
    pub fn new() -> GemonRestRequestBuilder {
        GemonRestRequestBuilder {
            gemon_method_type: None,
            url: None,
            headers: HashMap::new(),
            body: None,
            form_data: HashMap::new(),
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

    pub fn set_headers(self, headers: &HashMap<String, String>) -> GemonRestRequestBuilder {
        GemonRestRequestBuilder {
            headers: headers.clone(),
            ..self
        }
    }

    pub fn set_body(self, body: Option<String>) -> GemonRestRequestBuilder {
        GemonRestRequestBuilder { body, ..self }
    }

    pub fn set_form_data(self, form_data: &HashMap<String, String>) -> GemonRestRequestBuilder {
        GemonRestRequestBuilder {
            form_data: form_data.clone(),
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
            headers: self.headers.clone(),
            body: self.body.clone(),
            form_data: self.form_data.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GemonRestRequest {
    gemon_method_type: GemonMethodType,
    uri: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    form_data: HashMap<String, String>,
}

impl GemonRequest for GemonRestRequest {
    async fn execute(&self) -> Result<GemonResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let mut request = match self.gemon_method_type {
            GemonMethodType::GET => client.get(&self.uri),
            GemonMethodType::POST => client.post(&self.uri),
            GemonMethodType::DELETE => client.delete(&self.uri),
            GemonMethodType::PUT => client.put(&self.uri),
            GemonMethodType::PATCH => client.patch(&self.uri),
        };

        request = request
            .header(CONTENT_TYPE, constants::DEFAULT_CONTENT_TYPE)
            .header(ACCEPT, constants::DEFAULT_ACCEPT)
            .headers(self.headers.clone().to_header_map());

        if self.form_data.len() > 0 {
            request = request.form(&self.form_data);
        }

        if let Some(body) = self.body.as_ref() {
            request = request.body(body.to_string());
        }

        let response = request.send().await?;

        let status = response.error_for_status_ref();
        if let Some(err) = status.err() {
            panic!("Request failed with error code {:?}", err.status().unwrap());
        }

        let response_bytes = response.bytes().await?;
        Ok(GemonResponse::new(response_bytes))
    }

    fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("Could not parse GemonRestRequest to json string")
    }
}
