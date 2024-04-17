use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    gemon::run(env::args().collect()).await?;
    Ok(())
}
