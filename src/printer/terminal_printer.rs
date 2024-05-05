use bytes::Bytes;
use serde_json::Value;

use super::Printer;

pub struct TerminalPrinter {}

impl TerminalPrinter {
    pub fn new() -> TerminalPrinter {
        TerminalPrinter {}
    }

    pub fn print_string(&self, path: &str) -> Result<(), std::io::Error> {
        let value: Value = serde_json::from_str(path)?;
        let pretty_response = serde_json::to_string_pretty(&value)?;
        println!("{}", pretty_response);
        Ok(())
    }
}

impl Printer for TerminalPrinter {
    fn print(&self, bytes: &Bytes) -> Result<(), std::io::Error> {
        let response_value: Value = serde_json::from_slice(bytes)?;
        let pretty_response = serde_json::to_string_pretty(&response_value)?;
        println!("{}", pretty_response);
        Ok(())
    }
}
