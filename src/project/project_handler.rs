use super::{Environment, Project, ProjectError};
use crate::{
    config::effector::Effector,
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
    let metadata_json = Effector::apply_env_to_string(
        fs::read_to_string(format!("{name}/metadata.json"))
            .expect("Could not read metadata json file for request"),
    );
    let body_json = fs::read_to_string(format!("{name}/body.json"))
        .and_then(|bj| Ok(Effector::apply_env_to_string(bj)))
        .ok();
    let mut request = RequestBuilder::build_from_string(&metadata_json, &request_type);
    request.set_body(body_json);
    request
}

pub fn delete_request(name: &String) -> EmptyResult {
    validate_prject();
    fs::remove_dir_all(name).map_err(|err| err.into())
}

pub fn add_env_value(name: &String, env_value: (String, String)) -> EmptyResult {
    let mut project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    project.add_env_value(name, env_value);
    project.save()
}

pub fn remove_env_value(env: &String, key: &str) -> EmptyResult {
    let mut project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    project.remove_env_value(env, key);
    project.save()
}

pub fn remove_env(env: &String) -> EmptyResult {
    let mut project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    project.remove_env(env);
    project.save()
}

pub fn set_selected_env(env: &String) -> EmptyResult {
    let mut project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    project.set_selected_env(env)?;
    project.save()
}

pub fn get_selected_env() -> Option<Environment> {
    get_project().and_then(|p| p.get_selected_env())
}

pub fn print_selected_env() -> EmptyResult {
    let project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    let selected_env = project
        .get_selected_env()
        .ok_or(ProjectError {
            message: String::from("Selected env not set!"),
        })?
        .values();
    let result = serde_json::to_string_pretty(&selected_env)?;
    println!("{}", result);
    Ok(())
}

pub fn print_all_env() -> EmptyResult {
    let project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    let result = serde_json::to_string_pretty(&project.environments)?;
    println!("{}", result);
    Ok(())
}
