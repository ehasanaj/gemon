use std::{env, error::Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    gemon::run(env::args().collect()).await?;
    Ok(())
}
