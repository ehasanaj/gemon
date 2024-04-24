use crate::config::GemonConfig;
use bytes::Bytes;

use self::terminal_printer::TerminalPrinter;

pub mod terminal_printer;

pub trait Printer {
    fn print(&self, bytes: &Bytes) -> Result<(), std::io::Error>;
}

pub struct PrinterBuilder {}

impl PrinterBuilder {
    pub fn build(config: &GemonConfig) -> Box<impl Printer> {
        match config.gemon_printer() {
            crate::config::types::GemonPrinter::Terminal => Box::new(TerminalPrinter::new()),
            crate::config::types::GemonPrinter::File => todo!(),
        }
    }
}
