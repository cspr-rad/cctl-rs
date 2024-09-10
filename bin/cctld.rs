use clap::Parser;
use sd_notify::NotifyState;
use std::path::PathBuf;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
pub struct Cli {
    #[arg(short, long)]
    pub working_dir: Option<PathBuf>,
    #[arg(short, long)]
    pub deploy_contracts: Option<Vec<cctl::DeployableContract>>,
    #[arg(short = 's', long)]
    pub chainspec_path: Option<PathBuf>,
    #[arg(short = 'c', long)]
    pub config_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let _network = cctl::CCTLNetwork::run(
        cli.working_dir,
        cli.deploy_contracts,
        cli.chainspec_path,
        cli.config_path,
    )
    .await
    .expect("An error occured while starting the CCTL network");

    let _ = sd_notify::notify(true, &[NotifyState::Ready]);
    signal::ctrl_c().await?;
    Ok(())
}
