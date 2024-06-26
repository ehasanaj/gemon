use self::project_handler::{
    add_env_value, delete_request, get_request, get_selected_env, print_all_env,
    print_selected_env, remove_env, remove_env_value, save_request, set_selected_env,
};
use crate::{
    command::GemonCommand,
    config::{types::GemonProjectScenario, GemonConfig},
    constants::{NO_ENV, PROJECT_ROOT_FILE},
    printer::terminal_printer::TerminalPrinter,
    project::project_handler::get_project,
    request::{request_builder::RequestBuilder, Request},
    EmptyResult,
};
use project_handler::{add_authorization, remove_authorization};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fmt, fs, io::stdin};

pub mod project_handler;

#[derive(Serialize, Deserialize, Clone)]
pub struct Environment {
    values: HashMap<String, String>,
}

impl Environment {
    pub fn from(touple: (String, String)) -> Environment {
        let mut values = HashMap::new();
        values.insert(touple.0, touple.1);
        Environment { values }
    }

    fn add_value(&mut self, env: (String, String)) {
        self.values
            .entry(env.0)
            .and_modify(|entry| *entry = env.1.to_string())
            .or_insert(env.1.to_string());
    }

    fn remove_value(&mut self, key: &str) {
        self.values.remove_entry(key);
    }

    pub fn values(&self) -> HashMap<String, String> {
        self.values.clone()
    }
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

impl ProjectError {
    pub fn from(message: &str) -> Box<Self> {
        Box::new(ProjectError {
            message: String::from(message),
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    name: String,
    selected_environment: Option<String>,
    environments: HashMap<String, Environment>,
    authorization: HashMap<String, String>,
    last_called_request_path: Option<String>,
}

impl Project {
    fn set_last_called_request_path(&mut self, path: Option<String>) {
        self.last_called_request_path = path;
    }

    fn get_last_called_request_path(&self) -> Option<String> {
        self.last_called_request_path.to_owned()
    }

    fn add_env_value(&mut self, name: &String, env_value: (String, String)) {
        self.environments
            .entry(name.into())
            .and_modify(|env| env.add_value(env_value.clone()))
            .or_insert(Environment::from(env_value));
    }

    fn remove_env_value(&mut self, name: &String, key: &str) {
        self.environments
            .entry(name.into())
            .and_modify(|env| env.remove_value(key));
    }

    fn remove_env(&mut self, env: &String) {
        self.environments.remove_entry(env);
    }

    fn set_selected_env(&mut self, env: &String) -> EmptyResult {
        if !self.environments.contains_key(env) {
            return Err(ProjectError::from("Environment does not exist!"));
        }
        self.selected_environment = Some(env.to_owned());
        Ok(())
    }

    fn get_selected_env(&self) -> Option<Environment> {
        match self.selected_environment.as_ref() {
            Some(env) => self.environments.get(env).map(|e| e.clone()),
            None => None,
        }
    }

    fn save(&self) -> EmptyResult {
        let project_str = serde_json::to_string_pretty(&self)?;
        fs::write(PROJECT_ROOT_FILE, project_str).map_err(|err| err.into())
    }

    pub fn env() -> Option<Environment> {
        get_selected_env()
    }

    pub fn authorization(&self) -> Option<&String> {
        let env = match &self.selected_environment {
            Some(e) => e.to_owned(),
            None => String::from(NO_ENV),
        };
        self.authorization.get(&env)
    }

    pub fn set_authorization(&mut self, authorization: &String) -> EmptyResult {
        let env = match &self.selected_environment {
            Some(e) => e.to_owned(),
            None => String::from(NO_ENV),
        };
        self.authorization.insert(env, authorization.into());
        Ok(())
    }

    pub fn remove_authorization(&mut self) -> EmptyResult {
        let env = match &self.selected_environment {
            Some(e) => e.to_owned(),
            None => String::from(NO_ENV),
        };
        self.authorization.remove_entry(&env);
        Ok(())
    }

    pub async fn execute(config: &GemonConfig, scenario: &GemonProjectScenario) -> EmptyResult {
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
            GemonProjectScenario::AddEnv(e, k, v) => add_env_value(e, (k.to_owned(), v.to_owned())),
            GemonProjectScenario::RemoveEnvValue(e, k) => remove_env_value(e, k),
            GemonProjectScenario::SelectEnv(e) => set_selected_env(e),
            GemonProjectScenario::PrintEnvAll => print_all_env(),
            GemonProjectScenario::PrintEnv => print_selected_env(),
            GemonProjectScenario::RemoveEnv(e) => remove_env(e),
            GemonProjectScenario::Help => GemonCommand::print_all(),
            GemonProjectScenario::RemoveAuthorization => remove_authorization(),
            GemonProjectScenario::AddAuthorization(authorization) => {
                add_authorization(authorization)
            }
        }
    }

    fn init() -> EmptyResult {
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
            authorization: HashMap::new(),
        };
        project.save()
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
