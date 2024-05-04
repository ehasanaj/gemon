use super::Project;
use crate::{
    constants::PROJECT_ROOT_FILE,
    request::request_builder::{GemonRequest, RequestBuilder},
    EmptyResult,
};
use std::fs;

fn validate_prject() {
    get_project().expect("Valid Gemon project not found!");
}

pub fn get_project() -> Option<Project> {
    let project_str = match fs::read_to_string(PROJECT_ROOT_FILE) {
        Ok(ps) => ps,
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => return None,
            _ => panic!("Error reading project file"),
        },
    };

    let project: Project = serde_json::from_str(&project_str).expect("Error parsing project file");
    Some(project)
}

pub fn save_request(request: Box<impl GemonRequest>, name: &String) -> Box<impl GemonRequest> {
    validate_prject();
    let json_metadata = request.json_metadata();
    let json_body = request.json_body();
    let request_type_marker = request.request_type();
    if let Err(err) = fs::read_dir(name) {
        match err.kind() {
            std::io::ErrorKind::NotFound => fs::create_dir(name).expect("Create dir failed!"),
            std::io::ErrorKind::PermissionDenied => {
                panic!("User does not have permissions to write to dir!")
            }
            _ => panic!("Error while trying to create dir for the request!"),
        };
    }
    fs::write(format!("{}/metadata.json", name), json_metadata)
        .expect("Could not create metadata file!");
    fs::write(format!("{}/body.json", name), json_body).expect("Could not create body file!");
    fs::write(format!("{}/.marker", name), request_type_marker)
        .expect("Failed to mark request dir");
    request
}

pub fn get_request(name: &String) -> Box<impl GemonRequest> {
    validate_prject();
    let _ = fs::read_dir(name)
        .unwrap_or_else(|_| panic!("Could not find saved request with name: {}", name));
    let request_type =
        fs::read_to_string(format!("{name}/.marker")).expect("Could not read dir marker");
    let metadata_json = fs::read_to_string(format!("{name}/metadata.json"))
        .expect("Could not read metadata json file for request");
    let body_json = fs::read_to_string(format!("{name}/body.json")).ok();
    let mut request = RequestBuilder::build_from_string(&metadata_json, &request_type);
    request.set_body(body_json);
    request
}

pub fn delete_request(name: &String) -> EmptyResult {
    fs::remove_dir_all(name).map_err(|err| err.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::types::GemonMethodType, request::rest_request::GemonRestRequestBuilder};
    use reqwest::header::AUTHORIZATION;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn saves_rest_request() {
        let body = json!({
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        });
        let mut headers = HashMap::new();
        headers.insert(AUTHORIZATION.to_string(), "Bearer something".to_string());
        let request = Box::new(
            GemonRestRequestBuilder::new()
                .set_gemon_method_type(GemonMethodType::Post)
                .set_url("http://localhost:8080/post".to_string())
                .set_headers(&headers)
                .set_body(Some(body.to_string()))
                .build(),
        );
        let name = String::from("post_smth");
        let saved_request = save_request(request.clone(), &name);
        assert_eq!(saved_request.json_metadata(), request.json_metadata())
    }

    #[test]
    fn get_request_by_name() {
        let name = String::from("post_smth");
        let request = get_request(&name);
        assert!(!request.json_metadata().is_empty())
    }
}
