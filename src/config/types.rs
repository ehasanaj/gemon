#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonType {
    REST,
    WEBSOCKET,
    PROTO,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemonMethodType {
    GET,
    POST,
    DELETE,
    PUT,
}

impl GemonMethodType {
    pub fn value(&self) -> String {
        match self {
            GemonMethodType::GET => String::from("GET"),
            GemonMethodType::POST => String::from("POST"),
            GemonMethodType::DELETE => String::from("DELETE"),
            GemonMethodType::PUT => String::from("PUT"),
        }
    }
}
