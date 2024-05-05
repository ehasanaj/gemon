use crate::config::{types::GemonPrinter, GemonConfig};
use bytes::Bytes;

use self::{file_printer::FilePrinter, terminal_printer::TerminalPrinter};

pub mod file_printer;
pub mod terminal_printer;

pub trait Printer {
    fn print(&self, bytes: &Bytes) -> Result<(), std::io::Error>;
}

pub struct PrinterBuilder {}

impl PrinterBuilder {
    pub fn build(config: &GemonConfig) -> Box<dyn Printer> {
        match config.gemon_printer() {
            GemonPrinter::Terminal => Box::new(TerminalPrinter::new()),
            GemonPrinter::File => {
                let mut file_printer = FilePrinter::new(config.gemon_also_print_to_terminal());
                file_printer.set_file_path(config.gemon_response_file_path());
                Box::new(file_printer)
            }
        }
    }
}
