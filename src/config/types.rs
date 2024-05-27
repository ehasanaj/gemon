use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub enum GemonScenario {
    Request,
    Misc(MiscScenario),
    Project(GemonProjectScenario),
}

#[derive(Debug, Clone)]
pub enum GemonProjectScenario {
    Init,
    Call(String),
    Save(String),
    SaveAndCall(String),
    Delete(String),
    PrintLastCall,
    PrintEnvAll,
    PrintEnv,
    AddEnv(String, String, String),
    RemoveEnvValue(String, String),
    RemoveEnv(String),
    SelectEnv(String),
    Help,
}

#[derive(Debug, Clone)]
pub enum MiscScenario {
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonType {
    Rest,
    Websocket,
    Proto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GemonMethodType {
    Get,
    Post,
    Delete,
    Put,
    Patch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonPrinter {
    Terminal,
    File,
}
