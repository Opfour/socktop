# socktop_connector

A WebSocket connector library for communicating with socktop agents.

## Overview

`socktop_connector` provides a high-level, type-safe interface for connecting to socktop agents over WebSocket connections. It handles connection management, TLS certificate pinning, compression, and protocol buffer decoding automatically.

## Features

- **WebSocket Communication**: Support for both `ws://` and `wss://` connections
- **TLS Security**: Certificate pinning for secure connections with self-signed certificates
- **Hostname Verification**: Configurable hostname verification for TLS connections
- **Type Safety**: Strongly typed requests and responses
- **Automatic Compression**: Handles gzip compression/decompression transparently
- **Protocol Buffer Support**: Decodes binary process data automatically
- **Error Handling**: Comprehensive error handling with detailed error messages

## Connection Types

### Non-TLS Connections (`ws://`)
Use `connect_to_socktop_agent()` for unencrypted WebSocket connections. 

### TLS Connections (`wss://`) 
Use `connect_to_socktop_agent_with_tls()` for encrypted connections with certificate pinning. You can control hostname verification with the `verify_hostname` parameter.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
socktop_connector = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use socktop_connector::{connect_to_socktop_agent, AgentRequest, AgentResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a socktop agent (non-TLS connections are always unverified)
    let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
    
    // Request metrics
    match connector.request(AgentRequest::Metrics).await? {
        AgentResponse::Metrics(metrics) => {
            println!("CPU: {}%, Memory: {}/{}MB", 
                metrics.cpu_total,
                metrics.mem_used / 1024 / 1024,
                metrics.mem_total / 1024 / 1024
            );
        }
        _ => unreachable!(),
    }
    
    // Request process list
    match connector.request(AgentRequest::Processes).await? {
        AgentResponse::Processes(processes) => {
            println!("Total processes: {}", processes.process_count);
            for process in processes.top_processes.iter().take(5) {
                println!("  {} (PID: {}) - CPU: {}%", 
                    process.name, process.pid, process.cpu_usage);
            }
        }
        _ => unreachable!(),
    }
    
    Ok(())
}
```

### TLS with Certificate Pinning

```rust
use socktop_connector::{connect_to_socktop_agent_with_tls, AgentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect with TLS certificate pinning and hostname verification
    let mut connector = connect_to_socktop_agent_with_tls(
        "wss://remote-host:8443/ws",
        "/path/to/cert.pem",
        false  // Enable hostname verification
    ).await?;
    
    let response = connector.request(AgentRequest::Disks).await?;
    println!("Got disk info: {:?}", response);
    
    Ok(())
}
```

### Advanced Configuration

```rust
use socktop_connector::{ConnectorConfig, SocktopConnector, AgentRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a custom configuration
    let config = ConnectorConfig::new("wss://remote-host:8443/ws")
        .with_tls_ca("/path/to/cert.pem")
        .with_hostname_verification(false);
    
    // Create and connect
    let mut connector = SocktopConnector::new(config);
    connector.connect().await?;
    
    // Make requests
    let response = connector.request(AgentRequest::Metrics).await?;
    
    // Clean disconnect
    connector.disconnect().await?;
    
    Ok(())
}
```

## Request Types

The library supports three types of requests:

- `AgentRequest::Metrics` - Get current system metrics (CPU, memory, network, etc.)
- `AgentRequest::Disks` - Get disk usage information
- `AgentRequest::Processes` - Get running process information

## Response Types

Responses are automatically parsed into strongly-typed structures:

- `AgentResponse::Metrics(Metrics)` - System metrics with CPU, memory, network data
- `AgentResponse::Disks(Vec<DiskInfo>)` - List of disk usage information
- `AgentResponse::Processes(ProcessesPayload)` - Process list with CPU and memory usage

## Configuration Options

The library provides flexible configuration through the `ConnectorConfig` builder:

- `with_tls_ca(path)` - Enable TLS with certificate pinning
- `with_hostname_verification(bool)` - Control hostname verification for TLS connections
  - `true` (recommended): Verify the server hostname matches the certificate  
  - `false`: Skip hostname verification (useful for localhost or IP-based connections)

**Note**: Hostname verification only applies to TLS connections (`wss://`). Non-TLS connections (`ws://`) don't use certificates, so hostname verification is not applicable.

## Security Considerations

- **Production TLS**: Always enable hostname verification (`verify_hostname: true`) for production
- **Development/Testing**: You may disable hostname verification for localhost or IP addresses
- **Certificate Pinning**: Use `with_tls_ca()` for self-signed certificates
- **Non-TLS**: Use only for development or trusted networks

## Environment Variables

Currently no environment variables are used. All configuration is done through the API.

## Error Handling

The library uses `anyhow::Error` for error handling, providing detailed error messages for common failure scenarios:

- Connection failures
- TLS certificate validation errors  
- Protocol errors
- Parsing errors

## License

MIT License - see the LICENSE file for details.
