use super::Printer;
use serde_json::Value;
use std::fs;

pub struct FilePrinter {
    file_path: Option<String>,
    also_print_to_terminal: bool,
}

impl FilePrinter {
    pub fn new(also_print_to_terminal: bool) -> FilePrinter {
        FilePrinter {
            file_path: None,
            also_print_to_terminal,
        }
    }

    pub fn set_file_path(&mut self, file_path: Option<String>) {
        self.file_path = file_path;
    }
}

impl Printer for FilePrinter {
    fn print(&self, bytes: &bytes::Bytes) -> Result<(), std::io::Error> {
        let response_value: Value = serde_json::from_slice(bytes)?;
        let pretty_response = serde_json::to_string_pretty(&response_value)?;
        if self.also_print_to_terminal {
            println!("{}", pretty_response);
        }
        let path = self.file_path.as_ref().expect("File path missing!");
        fs::write(path, pretty_response)
    }
}
