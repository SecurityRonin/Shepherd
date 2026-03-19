use anyhow::Result;
use shepherd_core::config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("shepherd=info".parse()?))
        .init();

    let cfg = config::load_config(None)?;
    let (addr, _state, handle) = shepherd_server::startup::start_server(cfg).await?;
    println!("Shepherd server listening on http://{addr}");
    handle.await?;
    Ok(())
}
