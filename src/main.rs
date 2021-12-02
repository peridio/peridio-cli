mod cli;
mod node;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = cli::Opt::from_args();
    opts.handle_command().await;
    Ok(())
}
