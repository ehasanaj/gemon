use super::{Environment, Project, ProjectError};
use crate::{
    config::{effector::Effector, types::GemonMethodType},
    constants::{NO_ENV, PROJECT_ROOT_FILE},
    request::{
        request_builder::{GemonRequest, RequestBuilder},
        rest_request::GemonRestRequest,
    },
    EmptyResult,
};
use serde_json::Value;
use std::{error::Error, fs, path::Path};

const REQUEST_FILES: [&str; 3] = [".marker", "metadata.json", "body.json"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedRequestInfo {
    pub name: String,
    pub request_type: String,
    pub method: Option<GemonMethodType>,
}

/// What currently occupies the project path a request with a given name would be saved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestSlot {
    Free,
    SavedRequest,
    Taken,
}

fn validate_prject() {
    get_project().expect("Valid Gemon project not found!");
}

/// The project, or an error (never a panic) when it is missing or unreadable.
fn require_project() -> Result<Project, Box<dyn Error>> {
    match try_get_project() {
        Ok(Some(project)) => Ok(project),
        Ok(None) => Err(ProjectError::from("Project not found!")),
        Err(message) => Err(ProjectError::from(&message)),
    }
}

/// Reads the project file without panicking, so interactive callers can report problems.
pub fn try_get_project() -> Result<Option<Project>, String> {
    let project_str = match fs::read_to_string(PROJECT_ROOT_FILE) {
        Ok(ps) => ps,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("Error reading {PROJECT_ROOT_FILE}: {err}")),
    };

    serde_json::from_str(&project_str)
        .map(Some)
        .map_err(|err| format!("Error parsing {PROJECT_ROOT_FILE}: {err}"))
}

pub fn get_project() -> Option<Project> {
    try_get_project().unwrap_or_else(|err| panic!("{err}"))
}

/// Request names become directory names inside the project, so they must stay inside it.
pub fn validate_request_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(String::from("Name is required"));
    }
    if name != name.trim() {
        return Err(String::from("Name cannot start or end with spaces"));
    }
    if name.starts_with('.') {
        return Err(String::from("Name cannot start with '.'"));
    }
    if name.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            )
    }) {
        return Err(String::from(
            "Name cannot contain / \\ : * ? \" < > | or control characters",
        ));
    }
    Ok(())
}

/// Environment names are stored as keys next to the reserved default authorization key.
pub fn validate_env_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(String::from("Environment name is required"));
    }
    if name != name.trim() {
        return Err(String::from("Name cannot start or end with spaces"));
    }
    if name == NO_ENV {
        return Err(format!("'{NO_ENV}' is reserved"));
    }
    Ok(())
}

pub fn request_slot(name: &str) -> RequestSlot {
    let path = Path::new(name);
    if !path.exists() {
        RequestSlot::Free
    } else if path.is_dir() && path.join(".marker").exists() {
        RequestSlot::SavedRequest
    } else {
        RequestSlot::Taken
    }
}

fn ensure_request_name_available(name: &str) -> EmptyResult {
    validate_request_name(name).map_err(|message| ProjectError::from(&message))?;
    match request_slot(name) {
        RequestSlot::Free => Ok(()),
        RequestSlot::SavedRequest => Err(ProjectError::from(&format!(
            "A request named '{name}' already exists"
        ))),
        RequestSlot::Taken => Err(ProjectError::from(&format!(
            "'{name}' already exists in the project folder"
        ))),
    }
}

fn saved_request_method(metadata_path: &Path) -> Option<GemonMethodType> {
    let metadata = fs::read_to_string(metadata_path).ok()?;
    let value: Value = serde_json::from_str(&metadata).ok()?;
    serde_json::from_value(value.get("gemon_method_type")?.clone()).ok()
}

pub fn create_project(name: &str) -> EmptyResult {
    Project::init_named(name)
}

pub fn list_saved_requests() -> Result<Vec<SavedRequestInfo>, Box<dyn Error>> {
    require_project()?;

    // Unreadable folders are skipped so one bad entry never hides every other request.
    let mut requests = Vec::new();
    for entry in fs::read_dir(".")? {
        let Ok(entry) = entry else {
            continue;
        };
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

        let Ok(request_type) = fs::read_to_string(marker_path) else {
            continue;
        };
        let request_type = request_type.trim().to_string();
        requests.push(SavedRequestInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            request_type,
            method: saved_request_method(&metadata_path),
        });
    }

    requests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(requests)
}

pub fn read_saved_rest_request(name: &str) -> Result<GemonRestRequest, Box<dyn Error>> {
    require_project()?;

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
    if let Err(message) = validate_request_name(name) {
        panic!("Invalid request name '{name}': {message}");
    }
    if request_slot(name) == RequestSlot::Taken {
        panic!("'{name}' already exists in the project folder and is not a saved request");
    }
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

/// Saves a REST request without panicking; overwrites an existing saved request of that name.
pub fn save_rest_request(name: &str, request: &GemonRestRequest) -> EmptyResult {
    require_project()?;
    validate_request_name(name).map_err(|message| ProjectError::from(&message))?;
    if request_slot(name) == RequestSlot::Taken {
        return Err(ProjectError::from(&format!(
            "'{name}' already exists in the project folder"
        )));
    }
    save_request_to_project(Path::new("."), request, name)
}

pub fn rename_request(old: &str, new: &str) -> EmptyResult {
    require_project()?;
    if old == new {
        return Ok(());
    }
    if request_slot(old) != RequestSlot::SavedRequest {
        return Err(ProjectError::from(&format!("Saved request '{old}' not found")));
    }
    // On case-insensitive file systems `new` may resolve to `old` itself.
    let case_change = existing_entry_name(new).as_deref() == Some(old);
    if case_change {
        validate_request_name(new).map_err(|message| ProjectError::from(&message))?;
    } else {
        ensure_request_name_available(new)?;
    }
    fs::rename(old, new).map_err(|err| err.into())
}

/// The on-disk spelling of the project entry `name` refers to: itself when it exists exactly,
/// or a differently cased entry on case-insensitive file systems.
pub fn existing_entry_name(name: &str) -> Option<String> {
    let entries = fs::read_dir(".")
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if entries.iter().any(|entry| entry == name) {
        return Some(name.to_string());
    }
    if !Path::new(name).exists() {
        return None;
    }
    let lower = name.to_lowercase();
    entries.into_iter().find(|entry| entry.to_lowercase() == lower)
}

/// Copies the request definition (not stored responses) under a new name.
pub fn duplicate_request(source: &str, new: &str) -> EmptyResult {
    require_project()?;
    if request_slot(source) != RequestSlot::SavedRequest {
        return Err(ProjectError::from(&format!(
            "Saved request '{source}' not found"
        )));
    }
    ensure_request_name_available(new)?;
    fs::create_dir_all(new)?;
    for file in REQUEST_FILES {
        let from = Path::new(source).join(file);
        if from.exists() {
            fs::copy(from, Path::new(new).join(file))?;
        }
    }
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

/// Deletes a saved request folder; refuses anything that is not one, so a name like
/// `../x` or `src` can never remove other files.
pub fn delete_request(name: &String) -> EmptyResult {
    require_project()?;
    validate_request_name(name).map_err(|message| ProjectError::from(&message))?;
    if request_slot(name) != RequestSlot::SavedRequest {
        return Err(ProjectError::from(&format!("'{name}' is not a saved request")));
    }
    fs::remove_dir_all(name).map_err(|err| err.into())
}

pub fn add_env_value(name: &String, env_value: (String, String)) -> EmptyResult {
    let mut project = require_project()?;
    project.add_env_value(name, env_value);
    project.save()
}

pub fn remove_env_value(env: &String, key: &str) -> EmptyResult {
    let mut project = require_project()?;
    project.remove_env_value(env, key);
    project.save()
}

pub fn remove_env(env: &String) -> EmptyResult {
    let mut project = require_project()?;
    project.remove_env(env);
    project.save()
}

pub fn set_selected_env(env: &String) -> EmptyResult {
    let mut project = require_project()?;
    project.set_selected_env(env)?;
    project.save()
}

pub fn get_selected_env() -> Option<Environment> {
    get_project().and_then(|p| p.get_selected_env())
}

pub fn print_selected_env() -> EmptyResult {
    let project = require_project()?;
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
    let project = require_project()?;
    let result = serde_json::to_string_pretty(&project.environments)?;
    println!("{}", result);
    Ok(())
}

pub fn add_env(name: &str) -> EmptyResult {
    validate_env_name(name).map_err(|message| ProjectError::from(&message))?;
    let mut project = require_project()?;
    project.add_env(name)?;
    project.save()
}

pub fn rename_env(old: &str, new: &str) -> EmptyResult {
    validate_env_name(new).map_err(|message| ProjectError::from(&message))?;
    let mut project = require_project()?;
    project.rename_env(old, new)?;
    project.save()
}

pub fn clear_selected_env() -> EmptyResult {
    let mut project = require_project()?;
    project.clear_selected_env();
    project.save()
}

/// Sets or, when `authorization` is empty, removes the authorization of `env`.
/// `NO_ENV` addresses the default authorization used when no environment is selected.
pub fn set_authorization_for(env: &str, authorization: &str) -> EmptyResult {
    let mut project = require_project()?;
    if authorization.trim().is_empty() {
        project.remove_authorization_for(env);
    } else {
        project.set_authorization_for(env, authorization);
    }
    project.save()
}

pub fn set_last_response_path(path: &str) -> EmptyResult {
    let mut project = require_project()?;
    project.set_last_called_request_path(Some(path.to_string()));
    project.save()
}

pub fn add_authorization(authorization: &String) -> EmptyResult {
    let mut project = require_project()?;
    project.set_authorization(authorization)?;
    project.save()
}

pub fn remove_authorization() -> EmptyResult {
    let mut project = require_project()?;
    project.remove_authorization()?;
    project.save()
}

pub fn authorization() -> Option<String> {
    try_get_project()
        .ok()
        .flatten()
        .and_then(|project| project.authorization().cloned())
}
