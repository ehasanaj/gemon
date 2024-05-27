use crate::{config::types::MiscScenario, EmptyResult};

pub struct Misc;

impl Misc {
    fn version() {
        println!("Gemon version: {}", env!("CARGO_PKG_VERSION"));
    }

    pub fn execute(scenario: &MiscScenario) -> EmptyResult {
        match scenario {
            MiscScenario::Version => Ok(Self::version()),
        }
    }
}
