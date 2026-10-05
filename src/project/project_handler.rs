use super::{Environment, Project, ProjectError};
use crate::{
    config::effector::Effector,
    constants::PROJECT_ROOT_FILE,
    request::{
        request_builder::{GemonRequest, RequestBuilder},
        rest_request::GemonRestRequest,
    },
    EmptyResult,
};
use std::{error::Error, fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedRequestInfo {
    pub name: String,
    pub request_type: String,
}

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

pub fn create_project(name: &str) -> EmptyResult {
    Project::init_named(name)
}

pub fn list_saved_requests() -> Result<Vec<SavedRequestInfo>, Box<dyn Error>> {
    get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;

    let mut requests = Vec::new();
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let marker_path = path.join(".marker");
        let metadata_path = path.join("metadata.json");
        let body_path = path.join("body.json");
        if !(marker_path.exists() && metadata_path.exists() && body_path.exists()) {
            continue;
        }

        let request_type = fs::read_to_string(marker_path)?.trim().to_string();
        requests.push(SavedRequestInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            request_type,
        });
    }

    requests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(requests)
}

pub fn read_saved_rest_request(name: &str) -> Result<GemonRestRequest, Box<dyn Error>> {
    get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;

    read_saved_rest_request_from(Path::new("."), name)
}

pub(crate) fn read_saved_rest_request_from(
    root: &Path,
    name: &str,
) -> Result<GemonRestRequest, Box<dyn Error>> {
    let request_path = root.join(name);
    let request_type = fs::read_to_string(request_path.join(".marker"))?;
    if request_type.trim() != "REST" {
        return Err(Box::new(ProjectError {
            message: format!("Saved request '{name}' is not a REST request"),
        }));
    }

    let metadata_json = fs::read_to_string(request_path.join("metadata.json"))?;
    let body = fs::read_to_string(request_path.join("body.json")).ok();
    let mut request: GemonRestRequest = serde_json::from_str(&metadata_json)?;
    request.set_body(body);
    Ok(request)
}

pub fn save_request(request: Box<impl GemonRequest>, name: &str) -> Box<impl GemonRequest> {
    validate_prject();
    save_request_to_project(Path::new("."), request.as_ref(), name)
        .expect("Could not save request into project");
    request
}

pub(crate) fn save_request_to_project(
    root: &Path,
    request: &impl GemonRequest,
    name: &str,
) -> EmptyResult {
    let request_dir = root.join(name);
    fs::create_dir_all(&request_dir)?;
    fs::write(request_dir.join("metadata.json"), request.json_metadata())?;
    fs::write(request_dir.join("body.json"), request.json_body())?;
    fs::write(request_dir.join(".marker"), request.request_type())?;
    Ok(())
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
        .map(Effector::apply_env_to_string)
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

pub fn add_authorization(authorization: &String) -> EmptyResult {
    let mut project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    project.set_authorization(authorization)?;
    project.save()
}

pub fn remove_authorization() -> EmptyResult {
    let mut project = get_project().ok_or(ProjectError {
        message: String::from("Project not found!"),
    })?;
    project.remove_authorization()?;
    project.save()
}

pub fn authorization() -> Option<String> {
    let project = get_project()
        .ok_or(ProjectError {
            message: String::from("Project not found!"),
        })
        .ok();
    match project {
        Some(p) => p.authorization().cloned(),
        None => None,
    }
}
