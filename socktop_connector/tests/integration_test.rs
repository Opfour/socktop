use socktop_connector::{
    AgentRequest, AgentResponse, connect_to_socktop_agent, connect_to_socktop_agent_with_tls,
};

// Integration probe: only runs when SOCKTOP_WS is set to an agent WebSocket URL.
// Example: SOCKTOP_WS=ws://127.0.0.1:3000/ws cargo test -p socktop_connector --test integration_test -- --nocapture
#[tokio::test]
async fn probe_ws_endpoints() {
    // Gate the test to avoid CI failures when no agent is running.
    let url = match std::env::var("SOCKTOP_WS") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!(
                "skipping ws_probe: set SOCKTOP_WS=ws://host:port/ws to run this integration test"
            );
            return;
        }
    };

    // Optional pinned CA for WSS/self-signed setups
    let tls_ca = std::env::var("SOCKTOP_TLS_CA").ok();

    let mut connector = if let Some(ca_path) = tls_ca {
        connect_to_socktop_agent_with_tls(&url, ca_path, true)
            .await
            .expect("connect ws with TLS")
    } else {
        connect_to_socktop_agent(&url).await.expect("connect ws")
    };

    // Should get fast metrics quickly
    let response = connector.request(AgentRequest::Metrics).await;
    assert!(response.is_ok(), "expected Metrics payload within timeout");
    if let Ok(AgentResponse::Metrics(_)) = response {
        // Success
    } else {
        panic!("expected Metrics response");
    }

    // Processes may be gzipped and a bit slower, but should arrive
    let response = connector.request(AgentRequest::Processes).await;
    assert!(
        response.is_ok(),
        "expected Processes payload within timeout"
    );
    if let Ok(AgentResponse::Processes(_)) = response {
        // Success
    } else {
        panic!("expected Processes response");
    }
}
