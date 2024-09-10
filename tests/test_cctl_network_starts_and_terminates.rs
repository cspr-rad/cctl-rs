use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use casper_client::{get_peers, JsonRpcId, Verbosity};
use cctl::{CCTLNetwork, NodeState};

fn tracing_init() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

#[tokio::test]
async fn test_cctl_network_starts_and_terminates() {
    tracing_init();

    let network = CCTLNetwork::run(None, None, None, None).await.unwrap();

    for node in &network.casper_sidecars {
        if node.state == NodeState::Running {
            // FIXME: getting the status is currently broken beteen sidecar <-> node
            // let response = get_node_status(
            //   JsonRpcId::Number(1),
            //   &format!("http://0.0.0.0:{}/rpc", node.port.rpc_port),
            //  )
            //  .await
            //  .unwrap();
            //  assert_eq!(response.result.reactor_state, ReactorState::Validate);
            let response = get_peers(
                JsonRpcId::Number(1),
                &format!("http://0.0.0.0:{}/rpc", node.port.rpc_port),
                Verbosity::High,
            )
            .await
            .unwrap();
            assert!(!response.result.peers.is_empty());
        }
    }
}
