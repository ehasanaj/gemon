use serde_derive::{Deserialize, Serialize};
use std::fmt;

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
    RemoveAuthorization,
    AddAuthorization(String),
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

impl GemonMethodType {
    pub const ALL: [GemonMethodType; 5] = [
        GemonMethodType::Get,
        GemonMethodType::Post,
        GemonMethodType::Delete,
        GemonMethodType::Put,
        GemonMethodType::Patch,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            GemonMethodType::Get => "GET",
            GemonMethodType::Post => "POST",
            GemonMethodType::Delete => "DELETE",
            GemonMethodType::Put => "PUT",
            GemonMethodType::Patch => "PATCH",
        }
    }

    pub fn next(self) -> GemonMethodType {
        let index = Self::ALL
            .iter()
            .position(|method| *method == self)
            .unwrap_or_default();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> GemonMethodType {
        let index = Self::ALL
            .iter()
            .position(|method| *method == self)
            .unwrap_or_default();
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

impl fmt::Display for GemonMethodType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonPrinter {
    Terminal,
    File,
}
