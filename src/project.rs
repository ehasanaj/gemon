use crate::{constants::PROJECT_ROOT_FILE, request_builder::rest_request::GemonRestRequest};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt, fs, io::stdin};

#[derive(Serialize, Deserialize)]
struct Environment {
    values: HashMap<String, String>,
}

#[derive(Debug)]
pub struct ProjectExistsError {
    pub message: String,
}

impl fmt::Display for ProjectExistsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl Error for ProjectExistsError {}

#[derive(Serialize, Deserialize)]
pub struct Project {
    name: String,
    selected_environment: Option<String>,
    environments: HashMap<String, Environment>,
    requests: HashMap<String, GemonRestRequest>,
}

impl Project {
    pub fn init() -> Result<(), Box<dyn Error>> {
        if let Some(_) = Project::get_project() {
            return Err(Box::new(ProjectExistsError {
                message: "Project already exists".to_string(),
            }));
        }

        println!("Provide the name of the project:");
        let mut name = String::new();
        stdin()
            .read_line(&mut name)
            .expect("Failed to read project name");

        let project = Project {
            name: name.trim().to_string(),
            selected_environment: None,
            environments: HashMap::new(),
            requests: HashMap::new(),
        };

        let project_str = serde_json::to_string_pretty(&project)?;
        fs::write(PROJECT_ROOT_FILE, project_str)?;
        Ok(())
    }

    fn get_project() -> Option<Project> {
        let project_str = match fs::read_to_string(PROJECT_ROOT_FILE) {
            Ok(ps) => ps,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => return None,
                _ => panic!("Error reading project file"),
            },
        };

        let project: Project =
            serde_json::from_str(&project_str).expect("Error parsing project file");
        Some(project)
    }
}
