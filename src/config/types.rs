use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub enum GemonScenario {
    Request,
    Project(GemonProjectScenario),
}

#[derive(Debug, Clone)]
pub enum GemonProjectScenario {
    Init,
    Call,
    Save(String),
    Delete(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonType {
    REST,
    WEBSOCKET,
    PROTO,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GemonMethodType {
    GET,
    POST,
    DELETE,
    PUT,
    PATCH,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonPrinter {
    Terminal,
    File,
}
