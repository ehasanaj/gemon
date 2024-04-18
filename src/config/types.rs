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
    PATCH,
}
