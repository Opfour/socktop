//! Example of using socktop_connector in a WASM environment.
//!
//! This example demonstrates how to use the connector without TLS dependencies
//! for WebAssembly builds.

use socktop_connector::{connect_to_socktop_agent, ConnectorConfig, AgentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("WASM-compatible socktop connector example");
    
    // For WASM builds, use ws:// (not wss://) to avoid TLS dependencies
    let url = "ws://localhost:3000/ws";
    
    // Method 1: Simple connection (recommended for most use cases)
    let mut connector = connect_to_socktop_agent(url).await?;
    
    // Method 2: With custom WebSocket configuration
    let config = ConnectorConfig::new(url)
        .with_protocols(vec!["socktop".to_string()])
        .with_version("13".to_string());
    
    let mut connector_custom = socktop_connector::SocktopConnector::new(config);
    connector_custom.connect().await?;
    
    // Make a request to get metrics
    match connector.request(AgentRequest::Metrics).await {
        Ok(response) => {
            println!("Successfully received response: {:?}", response);
        }
        Err(e) => {
            println!("Request failed: {}", e);
        }
    }
    
    println!("WASM example completed successfully!");
    Ok(())
}
