pub mod parsers;

use anyhow::anyhow;
use backoff::{future::retry, ExponentialBackoff};
use hex::FromHex;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::{
    fs,
    time::{Duration, Instant},
};
use tempfile::tempdir;

use casper_client::{
    get_deploy, get_state_root_hash, put_deploy, query_global_state, Error, JsonRpcId, Verbosity,
};
use casper_types::{
    account::AccountHash,
    contracts::ContractHash,
    execution::{execution_result_v1::ExecutionResultV1, ExecutionResult},
    runtime_args, DeployBuilder, ExecutableDeployItem, Key, PublicKey, RuntimeArgs, SecretKey,
    StoredValue, TimeDiff, Timestamp,
};

use parsers::RawNodeType;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NodeState {
    Running,
    Stopped,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CasperSidecarPorts {
    pub node_client_port: u16,
    pub rpc_port: u16,
    pub speculative_exec_port: u16,
}

pub struct CasperSidecar {
    pub id: u8,
    pub validator_group_id: u8,
    pub state: NodeState,
    pub port: CasperSidecarPorts,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CasperNodePorts {
    pub protocol_port: u16,
    pub binary_port: u16,
    pub rest_port: u16,
    pub sse_port: u16,
}

pub struct CasperNode {
    pub id: u8,
    pub validator_group_id: u8,
    pub state: NodeState,
    pub port: CasperNodePorts,
}

pub struct CCTLNetwork {
    pub working_dir: PathBuf,
    pub casper_nodes: Vec<CasperNode>,
    pub casper_sidecars: Vec<CasperSidecar>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeployableContract {
    /// This is the named key under which the contract hash is located
    pub hash_name: String,
    pub runtime_args: Option<RuntimeArgs>,
    pub path: PathBuf,
}

impl FromStr for DeployableContract {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

/// Configures the casper-client verbosity level depending on the tracing log level
pub fn casper_client_verbosity() -> Verbosity {
    if tracing::enabled!(tracing::Level::TRACE) {
        Verbosity::High
    } else if tracing::enabled!(tracing::Level::DEBUG) {
        Verbosity::Medium
    } else {
        Verbosity::Low
    }
}

// max amount allowed to be used on gas fees
pub const MAX_GAS_FEE_PAYMENT_AMOUNT: u64 = 10_000_000_000_000;

impl CCTLNetwork {
    /// Spins up a CCTL network, and deploys a contract if provided
    ///
    /// If a chain spec and config path are not provided, the environment variables `CCTL_CHAINSPEC` and `CCTL_CONFIG` are used.
    ///
    /// WARNING: do not use this function in unit tests, only sequentially executed integration tests.
    /// Ensure that two instances of this function are not running at the same time even in different processes.
    pub async fn run(
        working_dir: Option<PathBuf>,
        contracts_to_deploy: Option<Vec<DeployableContract>>,
        chainspec_path: Option<PathBuf>,
        config_path: Option<PathBuf>,
    ) -> anyhow::Result<CCTLNetwork> {
        let chainspec_path: Option<PathBuf> =
            chainspec_path.or_else(|| env::var("CCTL_CASPER_CHAINSPEC").ok().map(PathBuf::from));
        let config_path =
            config_path.or_else(|| env::var("CCTL_CASPER_NODE_CONFIG").ok().map(PathBuf::from));

        let working_dir = working_dir
            .map(|dir| {
                std::fs::create_dir_all(&dir)
                    .expect("Failed to create the provided working directory");
                dir
            })
            .unwrap_or(tempdir()?.into_path());
        let assets_dir = working_dir.join("assets");
        tracing::info!("Working directory: {:?}", working_dir);

        let mut setup_command = Command::new("cctl-infra-net-setup");
        setup_command.env("CCTL_ASSETS", &assets_dir);

        if let Some(chainspec_path) = chainspec_path {
            setup_command.arg(format!("chainspec={}", chainspec_path.to_str().unwrap()));
        };

        if let Some(config_path) = config_path {
            setup_command.arg(format!("config={}", config_path.to_str().unwrap()));
        };

        tracing::info!("Setting up network configuration");
        let output = setup_command
            .output()
            .expect("Failed to setup network configuration");
        let output = std::str::from_utf8(output.stdout.as_slice()).unwrap();
        tracing::info!("{}", output);

        let output = Command::new("cctl-infra-net-start")
            .env("CCTL_ASSETS", &assets_dir)
            .output()
            .expect("Failed to start network");
        let output = std::str::from_utf8(output.stdout.as_slice()).unwrap();
        tracing::info!("{}", output);
        let (_, nodes) = parsers::parse_cctl_infra_net_start_lines(output).unwrap();

        tracing::info!("Fetching the networks node ports");
        let output = Command::new("cctl-infra-node-view-ports")
            .env("CCTL_ASSETS", &assets_dir)
            .output()
            .expect("Failed to get the networks node ports");
        let output = std::str::from_utf8(output.stdout.as_slice()).unwrap();
        tracing::info!("{}", output);
        let (_, node_ports) = parsers::parse_cctl_infra_node_view_port_lines(output).unwrap();

        tracing::info!("Fetching the networks sidecar ports");
        let output = Command::new("cctl-infra-sidecar-view-ports")
            .env("CCTL_ASSETS", &assets_dir)
            .output()
            .expect("Failed to get the networks node ports");
        let output = std::str::from_utf8(output.stdout.as_slice()).unwrap();
        tracing::info!("{}", output);
        let (_, sidecar_ports) = parsers::parse_cctl_infra_sidecar_view_port_lines(output).unwrap();

        // Match the started nodes and sidecars with their respective ports

        let (casper_nodes, casper_sidecars): (Vec<CasperNode>, Vec<CasperSidecar>) =
            nodes.iter().partition_map(|node_type| match node_type {
                RawNodeType::CasperNode(validator_group_id, node_id, state) => {
                    if let Some(&(_, port)) = node_ports
                        .iter()
                        .find(|(node_id_ports, _)| node_id_ports == node_id)
                    {
                        Either::Left(CasperNode {
                            validator_group_id: *validator_group_id,
                            state: *state,
                            id: *node_id,
                            port,
                        })
                    } else {
                        panic!("Can't find ports for node with id {}", node_id)
                    }
                }
                RawNodeType::CasperSidecar(validator_group_id, node_id, state) => {
                    if let Some(&(_, port)) = sidecar_ports
                        .iter()
                        .find(|(node_id_ports, _)| node_id_ports == node_id)
                    {
                        Either::Right(CasperSidecar {
                            validator_group_id: *validator_group_id,
                            state: *state,
                            id: *node_id,
                            port: CasperSidecarPorts {
                                node_client_port: port.node_client_port,
                                rpc_port: port.rpc_port,
                                speculative_exec_port: port.speculative_exec_port,
                            },
                        })
                    } else {
                        panic!("Can't find ports for sidecar with id {}", node_id)
                    }
                }
            });

        tracing::info!("Waiting for block 1");
        let output = Command::new("cctl-chain-await-until-block-n")
            .env("CCTL_ASSETS", &assets_dir)
            .arg("height=1")
            .output()
            .expect("Waiting for network to start processing blocks failed");
        let output = std::str::from_utf8(output.stdout.as_slice()).unwrap();
        tracing::info!("{}", output);

        if let Some(contract_to_deploy) = contract_to_deploy {
            let rpc_port = casper_sidecars.first().unwrap().port.rpc_port;
            let casper_sidecar_rpc_url = format!("http://0.0.0.0:{rpc_port}/rpc");
            let deployer_skey =
                SecretKey::from_file(working_dir.join("assets/users/user-1/secret_key.pem"))?;
            let deployer_pkey =
                PublicKey::from_file(working_dir.join("assets/users/user-1/public_key.pem"))?;

            let contracts_dir = working_dir.join("contracts");
            fs::create_dir_all(&contracts_dir)?;

            for contract_to_deploy in contracts_to_deploy {
                let (hash_name, contract_hash) = deploy_contract(
                    &casper_sidecar_rpc_url,
                    &deployer_skey,
                    &deployer_pkey.to_account_hash(),
                    &contract_to_deploy,
                )
                .await?;
                fs::write(
                    contracts_dir.join(hash_name),
                    // For a ContractHash contract- will always be the prefix
                    contract_hash
                        .to_formatted_string()
                        .strip_prefix("contract-")
                        .unwrap(),
                )?
            }
        }
        Ok(CCTLNetwork {
            working_dir,
            casper_nodes,
            casper_sidecars,
        })
    }
    /// Get the deployed contract hash for a hash_name that was passed to new_contract
    /// https://docs.rs/casper-contract/latest/casper_contract/contract_api/storage/fn.new_contract.html
    pub fn get_contract_hash_for(&self, hash_name: &str) -> ContractHash {
        let contract_hash_path = self.working_dir.join("contracts").join(hash_name);
        let contract_hash_string = fs::read_to_string(contract_hash_path).unwrap();
        let contract_hash_bytes = <[u8; 32]>::from_hex(contract_hash_string).unwrap();
        ContractHash::new(contract_hash_bytes)
    }
}

impl Drop for CCTLNetwork {
    fn drop(&mut self) {
        let output = Command::new("cctl-infra-net-stop")
            .env("CCTL_ASSETS", self.working_dir.join("assets"))
            .output()
            .expect("Failed to stop the network");
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();
    }
}

/// Deploys a contract as the given user for the contract's defined hash name located at the path.
/// The hash name should be equal to the hash name passed to https://docs.rs/casper-contract/latest/casper_contract/contract_api/storage/fn.new_locked_contract.html
async fn deploy_contract(
    casper_node_rpc_url: &str,
    contract_deployer_skey: &SecretKey,
    contract_deployer_addr: &AccountHash,
    DeployableContract {
        hash_name,
        runtime_args,
        path,
    }: &DeployableContract,
) -> anyhow::Result<(String, ContractHash)> {
    tracing::info!(
        "Deploying contract '{}': {}",
        &hash_name,
        path.to_str().unwrap()
    );

    let casper_client_verbosity = casper_client_verbosity();

    let contract_bytes = fs::read(path)?;
    let runtime_args = runtime_args.clone().unwrap_or(runtime_args! {});
    let contract = ExecutableDeployItem::new_module_bytes(contract_bytes.into(), runtime_args);
    let deploy = DeployBuilder::new(
        // TODO ideally make the chain-name configurable
        "cspr-dev-cctl",
        contract,
    )
    .with_secret_key(contract_deployer_skey)
    .with_standard_payment(MAX_GAS_FEE_PAYMENT_AMOUNT) // max amount allowed to be used on gas fees
    .with_timestamp(Timestamp::now())
    .with_ttl(TimeDiff::from_millis(60_000)) // 1 min
    .build()?;

    tracing::info!("Submitting contract deploy");
    let deploy_hash = put_deploy(
        JsonRpcId::Number(1),
        casper_node_rpc_url,
        casper_client_verbosity,
        deploy,
    )
    .await
    .map_err(Into::<anyhow::Error>::into)
    .map(|response| response.result.deploy_hash)?;

    const MAX_CONTRACT_INIT_WAIT_TIME: Duration = Duration::from_secs(60);
    tracing::info!(
        "Waiting {MAX_CONTRACT_INIT_WAIT_TIME:?} for successful contract initialization"
    );
    let start = Instant::now();
    retry(ExponentialBackoff::default(), || async {
        let timed_out = start.elapsed() > MAX_CONTRACT_INIT_WAIT_TIME;

        let response = get_deploy(
            JsonRpcId::Number(1),
            casper_node_rpc_url,
            casper_client_verbosity,
            deploy_hash,
            false,
        )
        .await
        .map_err(|err| {
            let elapsed = start.elapsed().as_secs();
            tracing::info!("Waited {elapsed}s for successful contract initialization, the last reported error was: {err:?}");
            err
        })
        .map_err(|err| match &err {
            err if timed_out => backoff::Error::permanent(anyhow!("Timeout on error: {err:?}")),
            Error::ResponseIsHttpError { .. } | Error::FailedToGetResponse { .. } => {
                backoff::Error::transient(anyhow!(err))
            }
            _ => backoff::Error::permanent(anyhow!(err)),
        })?;

        match response.result.execution_info {
            Some(execution_info) => match execution_info.execution_result {
                Some(execution_result) => match &execution_result {
                    ExecutionResult::V1(execution_result_v1) => match execution_result_v1 {
                        ExecutionResultV1::Failure { error_message, .. } => {
                            Err(backoff::Error::permanent(anyhow!(error_message.clone())))
                        }
                        ExecutionResultV1::Success { .. } => Ok(()),
                    }
                    ExecutionResult::V2(execution_result_v2) => match &execution_result_v2.error_message {
                        None => Ok(()),
                        Some(error_message) => Err(backoff::Error::permanent(anyhow!(error_message.clone())))
                    }
                }
                None if timed_out => Err(backoff::Error::permanent(anyhow!(
                    "Timeout on error: No execution result"
                ))),
                None => Err(backoff::Error::transient(anyhow!(
                    "No execution results there yet"
                ))),
            },
            None if timed_out => Err(backoff::Error::permanent(anyhow!(
                "Timeout on error: No execution info"
            ))),
            None => Err(backoff::Error::transient(anyhow!(
                "No execution results there yet"
            ))),
        }
    })
    .await?;
    tracing::info!("Contract was deployed successfully");

    tracing::info!("Fetching deployed contract hash");
    // Query global state
    let state_root_hash = get_state_root_hash(
        JsonRpcId::Number(1),
        casper_node_rpc_url,
        casper_client_verbosity,
        Option::None,
    )
    .await
    .map_err(Into::<anyhow::Error>::into)
    .and_then(|response| {
        response
            .result
            .state_root_hash
            .ok_or(anyhow!("No state root hash present in response"))
    })?;

    tracing::info!("Querying global state");
    let contract_hash: ContractHash = query_global_state(
        JsonRpcId::Number(1),
        casper_node_rpc_url,
        casper_client_verbosity,
        casper_client::rpcs::GlobalStateIdentifier::StateRootHash(state_root_hash), // fetches recent blocks state root hash
        Key::AddressableEntity(casper_types::EntityAddr::Account(contract_deployer_addr.0)),
        vec![hash_name.clone()],
    )
    .await
    .map_err(Into::<anyhow::Error>::into)
    .and_then(|response| match response.result.stored_value {
        StoredValue::Package(package) => Ok(ContractHash::from(
            package
                .versions()
                .contract_hashes()
                .next()
                .expect("")
                .value(),
        )),
        StoredValue::ContractPackage(contract_package) => Ok(*contract_package
            .versions()
            .values()
            .next()
            .expect("Expected at least one contract version")),
        other => Err(anyhow!(
            "Unexpected result type, type is not a CLValue: {:?}",
            other
        )),
    })?;
    tracing::info!(
        "Successfully fetched the contract hash for {}: {}",
        &hash_name,
        &contract_hash
    );
    Ok::<(String, ContractHash), anyhow::Error>((hash_name.clone(), contract_hash))
}
