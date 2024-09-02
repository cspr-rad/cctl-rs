use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use casper_client::{get_node_status, JsonRpcId, Verbosity};
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
            let node_status = get_node_status(
                JsonRpcId::Number(1),
                &format!("http://0.0.0.0:{}", node.port.rpc_port),
                Verbosity::High,
            )
            .await
            .unwrap();
            assert_eq!(node_status.result.reactor_state, "Validate");
        }
    }
}
