use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub enum GemonScenario {
    Request,
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
    AddEnv(String, String, String),
    RemoveEnv(String, String),
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
