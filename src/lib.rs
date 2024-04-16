use std::io;

use crate::config::args::Arguments;

pub mod config;

pub fn run(args: Vec<String>) -> Result<(), io::Error> {
    Arguments::parse(args)
}
