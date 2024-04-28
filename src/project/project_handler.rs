use super::Project;
use crate::{
    constants::PROJECT_ROOT_FILE,
    request::{request_builder::GemonRequest, rest_request::GemonRestRequestBuilder},
    EmptyResult,
};
use std::fs;

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
    let request_json = request.to_string_pretty();
    println!("{}", request_json);
    get_request(name)
}

pub fn get_request(_name: &String) -> Box<impl GemonRequest> {
    Box::new(GemonRestRequestBuilder::new().build()) //TODO build as it should with all the required info
}

pub fn delete_request(name: &String) -> EmptyResult {
    fs::remove_dir_all(name).map_err(|err| err.into())
}
