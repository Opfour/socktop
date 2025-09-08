# socktop_connector

A WebSocket connector library for communicating with socktop agents.

## Overview

`socktop_connector` provides a high-level, type-safe interface for connecting to socktop agents over WebSocket connections. It handles connection management, TLS certificate pinning, compression, and protocol buffer decoding automatically.

The library is designed for professional use with structured error handling that allows you to pattern match on specific error types, making it easy to implement robust error recovery and monitoring strategies.

## Features

- **WebSocket Communication**: Support for both `ws://` and `wss://` connections
- **TLS Security**: Certificate pinning for secure connections with self-signed certificates
- **Hostname Verification**: Configurable hostname verification for TLS connections
- **Type Safety**: Strongly typed requests and responses
- **Automatic Compression**: Handles gzip compression/decompression transparently
- **Protocol Buffer Support**: Decodes binary process data automatically
- **Error Handling**: Comprehensive error handling with structured error types for pattern matching

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
tokio = { version = "1", features = ["rt", "rt-multi-thread", "net", "time", "macros"] }
```

**WASM Compatibility:** For WASM environments, use minimal features (single-threaded runtime):
```toml
[dependencies]
socktop_connector = "0.1"
tokio = { version = "1", features = ["rt", "time", "macros"] }
```
Note: TLS features (`wss://` connections) are not available in WASM environments.

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

### Error Handling with Pattern Matching

Take advantage of structured error types for robust error handling:

```rust
use socktop_connector::{connect_to_socktop_agent, ConnectorError, AgentRequest};

#[tokio::main]
async fn main() {
    // Handle connection errors specifically
    let mut connector = match connect_to_socktop_agent("ws://localhost:3000/ws").await {
        Ok(conn) => conn,
        Err(ConnectorError::WebSocketError(e)) => {
            eprintln!("Failed to connect to WebSocket: {}", e);
            return;
        }
        Err(ConnectorError::UrlError(e)) => {
            eprintln!("Invalid URL provided: {}", e);
            return;
        }
        Err(e) => {
            eprintln!("Connection failed: {}", e);
            return;
        }
    };
    
    // Handle request errors specifically  
    match connector.request(AgentRequest::Metrics).await {
        Ok(response) => println!("Success: {:?}", response),
        Err(ConnectorError::JsonError(e)) => {
            eprintln!("Failed to parse server response: {}", e);
        }
        Err(ConnectorError::WebSocketError(e)) => {
            eprintln!("Communication error: {}", e);
        }
        Err(e) => eprintln!("Request failed: {}", e),
    }
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

### WebSocket Protocol Configuration

For version compatibility (if applies), you can configure WebSocket protocol version and sub-protocols:

```rust
use socktop_connector::{ConnectorConfig, SocktopConnector, connect_to_socktop_agent_with_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Method 1: Using the convenience function
    let connector = connect_to_socktop_agent_with_config(
        "ws://localhost:3000/ws",
        Some(vec!["socktop".to_string(), "v1".to_string()]), // Sub-protocols
        Some("13".to_string()), // WebSocket version (13 is standard)
    ).await?;
    
    // Method 2: Using ConnectorConfig builder
    let config = ConnectorConfig::new("ws://localhost:3000/ws")
        .with_protocols(vec!["socktop".to_string()])
        .with_version("13");
    
    let mut connector = SocktopConnector::new(config);
    connector.connect().await?;
    
    Ok(())
}
```

**Note:** WebSocket version 13 is the current standard and is used by default. The sub-protocols feature is useful for protocol negotiation with servers that support multiple protocols.

## Continuous Updates

The socktop agent provides real-time system metrics. Each request returns the current snapshot, but you can implement continuous monitoring by making requests in a loop:

```rust
use socktop_connector::{connect_to_socktop_agent, AgentRequest, AgentResponse, ConnectorError};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
    
    // Monitor system metrics every 2 seconds
    loop {
        match connector.request(AgentRequest::Metrics).await {
            Ok(AgentResponse::Metrics(metrics)) => {
                // Calculate total network activity across all interfaces
                let total_rx: u64 = metrics.networks.iter().map(|n| n.received).sum();
                let total_tx: u64 = metrics.networks.iter().map(|n| n.transmitted).sum();
                
                println!("CPU: {:.1}%, Memory: {:.1}%, Network: ↓{} ↑{}", 
                    metrics.cpu_total,
                    (metrics.mem_used as f64 / metrics.mem_total as f64) * 100.0,
                    format_bytes(total_rx),
                    format_bytes(total_tx)
                );
            }
            Err(e) => {
                eprintln!("Error getting metrics: {}", e);
                
                // You can pattern match on specific error types for different handling
                match e {
                    socktop_connector::ConnectorError::WebSocketError(_) => {
                        eprintln!("Connection lost, attempting to reconnect...");
                        // Implement reconnection logic here
                        break;
                    }
                    socktop_connector::ConnectorError::JsonError(_) => {
                        eprintln!("Data parsing error, continuing...");
                        // Continue with next iteration for transient parsing errors
                    }
                    _ => {
                        eprintln!("Other error, stopping monitoring");
                        break;
                    }
                }
            }
            _ => unreachable!(),
        }
        
        sleep(Duration::from_secs(2)).await;
    }
    
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.1}{}", size, UNITS[unit_index])
}
```

### Understanding Data Freshness

The socktop agent implements intelligent caching to avoid overwhelming the system:

- **Metrics**: Cached for ~250ms by default (cheap / fast-changing data like CPU, memory)
- **Processes**: Cached for ~1500ms by default (exppensive / moderately changing data)  
- **Disks**: Cached for ~1000ms by default (cheap / slowly changing data)

These values have been generally tuned in advance. You should not need to override them. The reason for this cache is for the use case that multiple clients are requesting data. In general a single client should never really hit a cached response since the polling rates are slower that the cache intervals. Cache intervals have been tuned based on how much work the agent has to do in the case of reloading fresh data.


This means:

1. **Multiple rapid requests** for the same data type will return cached results
2. **Different data types** have independent cache timers
3. **Fresh data** is automatically retrieved when cache expires

```rust
use socktop_connector::{connect_to_socktop_agent, AgentRequest, AgentResponse};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
    
    // This demonstrates cache behavior
    println!("Requesting metrics twice quickly...");
    
    // First request - fresh data from system
    let start = std::time::Instant::now();
    connector.request(AgentRequest::Metrics).await?;
    println!("First request took: {:?}", start.elapsed());
    
    // Second request immediately - cached data  
    let start = std::time::Instant::now();
    connector.request(AgentRequest::Metrics).await?;
    println!("Second request took: {:?}", start.elapsed()); // Much faster!
    
    // Wait for cache to expire, then request again
    sleep(Duration::from_millis(300)).await;
    let start = std::time::Instant::now();
    connector.request(AgentRequest::Metrics).await?;
    println!("Third request (after cache expiry): {:?}", start.elapsed());
    
    Ok(())
}
```

The WebSocket connection remains open between requests, providing efficient real-time monitoring without connection overhead.

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
- `with_protocols(Vec<String>)` - Set WebSocket sub-protocols for protocol negotiation
- `with_version(String)` - Set WebSocket protocol version (default is "13", the current standard)

**Note**: Hostname verification only applies to TLS connections (`wss://`). Non-TLS connections (`ws://`) don't use certificates, so hostname verification is not applicable.

## WASM Support

`socktop_connector` supports WebAssembly (WASM) environments with some limitations:

### Supported Features
- Non-TLS WebSocket connections (`ws://`)  
- All core functionality (metrics, processes, disks)
- Continuous monitoring examples

### WASM Configuration
```toml
[dependencies]
socktop_connector = "0.1"
tokio = { version = "1", features = ["rt", "time", "macros"] }
# Note: "net" feature not needed in WASM - WebSocket connections use browser APIs
```

### WASM Limitations
- **No TLS support**: `wss://` connections are not available
- **No certificate pinning**: TLS-related features are disabled
- **Browser WebSocket API**: Uses browser's native WebSocket implementation

### WASM Example
```rust
use socktop_connector::{connect_to_socktop_agent, AgentRequest, AgentResponse};

// Use current_thread runtime for WASM compatibility
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
    
    match connector.request(AgentRequest::Metrics).await? {
        AgentResponse::Metrics(metrics) => {
            // In WASM, you might log to browser console instead of println!
            web_sys::console::log_1(&format!("CPU: {}%", metrics.cpu_total).into());
        }
        _ => unreachable!(),
    }
    
    Ok(())
}
```

## Security Considerations

- **Production TLS**: You can hostname verification (`verify_hostname: true`) for production systems, This will add an additional level of production of verifying the hostname against the certificate. Generally this is to stop a man in the middle attack, but since it will be the client who is fooled and not the server, the risk and likelyhood of this use case is rather low. Which is why this is disabled by default. 
- **Certificate Pinning**: Use `with_tls_ca()` for self-signed certificates, the socktop agent will generate certificates on start. see main readme for more details. 
- **Non-TLS**: Use only for development or trusted networks

## Environment Variables

Currently no environment variables are used. All configuration is done through the API.

## Error Handling

The library uses structured error types via `thiserror` for comprehensive error handling. You can pattern match on specific error types:

```rust
use socktop_connector::{connect_to_socktop_agent, ConnectorError, AgentRequest};

#[tokio::main]
async fn main() {
    match connect_to_socktop_agent("invalid://url").await {
        Ok(mut connector) => {
            // Handle successful connection
            match connector.request(AgentRequest::Metrics).await {
                Ok(response) => println!("Got response: {:?}", response),
                Err(ConnectorError::WebSocketError(e)) => {
                    eprintln!("WebSocket communication failed: {}", e);
                }
                Err(ConnectorError::JsonError(e)) => {
                    eprintln!("Failed to parse response: {}", e);
                }
                Err(e) => eprintln!("Other error: {}", e),
            }
        }
        Err(ConnectorError::UrlError(e)) => {
            eprintln!("Invalid URL: {}", e);
        }
        Err(ConnectorError::WebSocketError(e)) => {
            eprintln!("Failed to connect: {}", e);
        }
        Err(ConnectorError::TlsError(msg)) => {
            eprintln!("TLS error: {}", msg);
        }
        Err(e) => {
            eprintln!("Connection failed: {}", e);
        }
    }
}
```

### Error Types

The `ConnectorError` enum provides specific variants for different error conditions:

- `ConnectorError::WebSocketError` - WebSocket connection or communication errors
- `ConnectorError::TlsError` - TLS-related errors (certificate validation, etc.)
- `ConnectorError::UrlError` - URL parsing errors
- `ConnectorError::JsonError` - JSON serialization/deserialization errors
- `ConnectorError::ProtocolError` - Protocol-level errors
- `ConnectorError::CompressionError` - Gzip compression/decompression errors
- `ConnectorError::IoError` - I/O errors
- `ConnectorError::Other` - Other errors with descriptive messages

All errors implement `std::error::Error` so they work seamlessly with `Box<dyn std::error::Error>`, `anyhow`, and other error handling crates.

### Migration from Generic Errors

If you were previously using the library with generic error handling, your existing code will continue to work:

```rust
// This continues to work as before
async fn my_function() -> Result<(), Box<dyn std::error::Error>> {
    let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
    let response = connector.request(AgentRequest::Metrics).await?;
    Ok(())
}

// But now you can also use structured error handling for better control
async fn improved_function() -> Result<(), ConnectorError> {
    let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
    let response = connector.request(AgentRequest::Metrics).await?;
    Ok(())
}
```

## License

MIT License - see the LICENSE file for details.
