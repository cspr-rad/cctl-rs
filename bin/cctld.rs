use casper_types::runtime_args;
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
    pub deploy_contract: Option<String>,
    #[arg(short, long)]
    pub chainspec_path: Option<PathBuf>,
    #[arg(short, long)]
    pub config_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let deploy_contract = cli.deploy_contract.map(|deploy_contracts_arg| {
        match deploy_contracts_arg.split_once(':') {
            Some((hash_name, path)) => cctl::DeployableContract {
                hash_name: hash_name.to_string(),
                // FIXME at some point we want to make this parametrizable
                runtime_args: runtime_args! {},
                path: PathBuf::from(&path),
            },
            None => panic!("Error parsing the provided deploy contracts argument."),
        }
    });
    let _network = cctl::CCTLNetwork::run(
        cli.working_dir,
        deploy_contract,
        cli.chainspec_path,
        cli.config_path,
    )
    .await
    .expect("An error occured while starting the CCTL network");

    let _ = sd_notify::notify(true, &[NotifyState::Ready]);
    signal::ctrl_c().await?;
    Ok(())
}
