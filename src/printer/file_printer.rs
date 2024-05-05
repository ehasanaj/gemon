use std::fs;

use serde_json::Value;

use super::Printer;

pub struct FilePrinter {
    file_path: String,
}

impl FilePrinter {
    pub fn new() -> FilePrinter {
        FilePrinter {
            file_path: String::new(),
        }
    }

    pub fn set_file_path(&mut self, file_path: &str) {
        self.file_path = String::from(file_path);
    }
}

impl Printer for FilePrinter {
    fn print(&self, bytes: &bytes::Bytes) -> Result<(), std::io::Error> {
        let response_value: Value = serde_json::from_slice(bytes)?;
        let pretty_response = serde_json::to_string_pretty(&response_value)?;
        fs::write(&self.file_path, pretty_response)
    }
}
