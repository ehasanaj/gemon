use self::project_handler::{delete_request, get_request, save_request};
use crate::{
    config::{types::GemonProjectScenario, GemonConfig},
    constants::PROJECT_ROOT_FILE,
    printer::terminal_printer::TerminalPrinter,
    project::project_handler::get_project,
    request::{request_builder::RequestBuilder, Request},
};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt, fs, io::stdin};

mod project_handler;

#[derive(Serialize, Deserialize)]
struct Environment {
    values: HashMap<String, String>,
}

#[derive(Debug)]
pub struct ProjectError {
    pub message: String,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl Error for ProjectError {}

#[derive(Serialize, Deserialize)]
pub struct Project {
    name: String,
    selected_environment: Option<String>,
    environments: HashMap<String, Environment>,
    last_called_request_path: Option<String>,
}

impl Project {
    fn set_last_called_request_path(&mut self, path: Option<String>) {
        self.last_called_request_path = path;
    }

    fn get_last_called_request_path(&self) -> Option<String> {
        self.last_called_request_path.to_owned()
    }

    pub async fn execute(
        config: &GemonConfig,
        scenario: &GemonProjectScenario,
    ) -> Result<(), Box<dyn Error>> {
        match scenario {
            GemonProjectScenario::Init => Project::init(),
            GemonProjectScenario::Call(name) => {
                Request::call(get_request(name), config).await?;
                Project::update_last_request_path(config.gemon_response_file_path())?;
                Ok(())
            }
            GemonProjectScenario::Save(name) => {
                let request = RequestBuilder::build(config);
                save_request(request, name);
                Ok(())
            }
            GemonProjectScenario::SaveAndCall(name) => {
                let request = RequestBuilder::build(config);
                Request::call(save_request(request, name), config).await
            }
            GemonProjectScenario::Delete(name) => delete_request(name),
            GemonProjectScenario::PrintLastCall => Project::print_last_called_request(),
        }
    }

    fn init() -> Result<(), Box<dyn Error>> {
        if get_project().is_some() {
            return Err(Box::new(ProjectError {
                message: String::from("Project already exists"),
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
            last_called_request_path: None,
        };

        let project_str = serde_json::to_string_pretty(&project)?;
        fs::write(PROJECT_ROOT_FILE, project_str).map_err(|err| err.into())
    }

    fn update_last_request_path(path: Option<String>) -> Result<(), Box<dyn Error>> {
        let mut project = get_project().ok_or_else(|| {
            Box::new(ProjectError {
                message: String::from("Project could not be found!"),
            })
        })?;
        if path.is_none() {
            return Ok(());
        }
        project.set_last_called_request_path(path);
        let project_str = serde_json::to_string_pretty(&project)?;
        fs::write(PROJECT_ROOT_FILE, project_str).map_err(|err| err.into())
    }

    fn print_last_called_request() -> Result<(), Box<dyn Error>> {
        let project = get_project().ok_or_else(|| {
            Box::new(ProjectError {
                message: String::from("Project could not be found!"),
            })
        })?;
        let path = project.get_last_called_request_path().ok_or_else(|| {
            Box::new(ProjectError {
                message: String::from("No available response to print!"),
            })
        })?;
        let value = fs::read_to_string(path)?;
        TerminalPrinter::new()
            .print_string(&value)
            .map_err(|err| err.into())
    }
}
