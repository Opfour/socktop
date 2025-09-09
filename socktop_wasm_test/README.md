# WASM Compatibility Guide for socktop_connector

This directory contains a complete WebAssembly (WASM) compatibility test and implementation guide for the `socktop_connector` library.

## Overview

`socktop_connector` provides **full WebSocket networking support** for WebAssembly environments. The library includes complete connectivity functionality with automatic compression and protobuf decoding, making it easy to connect to socktop agents directly from browser applications.

## What Works in WASM

- ✅ **Full WebSocket connections** (`ws://` connections)
- ✅ **All request types** (`AgentRequest::Metrics`, `AgentRequest::Disks`, `AgentRequest::Processes`)
- ✅ **Automatic data processing**: Gzip decompression for metrics/disks, protobuf decoding for processes
- ✅ Configuration types (`ConnectorConfig`) 
- ✅ Request/Response types (`AgentRequest`, `AgentResponse`)  
- ✅ JSON serialization/deserialization of all types
- ✅ Protocol and version configuration builders
- ✅ All type-safe validation and error handling

## What Doesn't Work in WASM

- ❌ TLS connections (`wss://`) - use `ws://` only
- ❌ TLS certificate handling (use non-TLS endpoints)

## Quick Test

```bash
# Build the WASM package
wasm-pack build --target web --out-dir pkg

# Serve the test page
basic-http-server . --addr 127.0.0.1:8000

# Open http://127.0.0.1:8000 in your browser
# Check the browser console for test results
```

## WASM Dependencies

The test uses the WASM-compatible networking features:

```toml
[dependencies]
socktop_connector = { version = "0.1.5", default-features = false, features = ["wasm"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
wasm-bindgen = "0.2"
console_error_panic_hook = "0.1"

[dependencies.web-sys]
version = "0.3"
features = ["console"]
```

**Key**: Use `features = ["wasm"]` to enable full WebSocket networking support in WASM builds.

## Implementation Strategy

### 1. Use socktop_connector Types for Configuration

```rust
use wasm_bindgen::prelude::*;
use socktop_connector::{ConnectorConfig, AgentRequest, AgentResponse};

#[wasm_bindgen]
pub fn create_config() -> String {
    // Use socktop_connector types for type-safe configuration
    let config = ConnectorConfig::new("ws://localhost:3000/ws")
        .with_protocols(vec!["socktop".to_string(), "v1".to_string()])
        .with_version("13".to_string());
    
    // Return JSON for use with browser WebSocket API  
    serde_json::to_string(&config).unwrap_or_default()
}
```

### 2. Create Type-Safe Requests

```rust
#[wasm_bindgen]
pub fn create_metrics_request() -> String {
    let request = AgentRequest::Metrics;
    serde_json::to_string(&request).unwrap_or_default()
}

#[wasm_bindgen]
pub fn create_processes_request() -> String {
    let request = AgentRequest::Processes;
    serde_json::to_string(&request).unwrap_or_default()
}
```

### 3. Parse Responses with Type Safety

```rust
#[wasm_bindgen]
pub fn parse_metrics_response(json: &str) -> Option<String> {
    match serde_json::from_str::<AgentResponse>(json) {
        Ok(AgentResponse::Metrics(metrics)) => {
            Some(format!("CPU: {}%, Memory: {}MB", 
                metrics.cpu_total,
                metrics.mem_used / 1024 / 1024))
        }
        _ => None
    }
}
```

### 4. Browser Integration

Then in JavaScript:

```javascript
import init, { 
    create_config, 
    create_metrics_request, 
    parse_metrics_response 
} from './pkg/socktop_wasm_test.js';

async function run() {
    await init();
    
    // Use type-safe configuration
    const configJson = create_config();
    const config = JSON.parse(configJson);
    
    // Create WebSocket with proper protocols
    const ws = new WebSocket(config.url, config.ws_protocols);
    
    ws.onopen = () => {
        // Send type-safe requests
        ws.send(create_metrics_request());
    };
    
    ws.onmessage = (event) => {
        // Handle responses with type safety
        const result = parse_metrics_response(event.data);
        if (result) {
            console.log(result);
        }
    };
}

run();
```

## Benefits of This Approach

1. **Type Safety**: All socktop types work identically in WASM
2. **Validation**: Configuration validation happens in Rust
3. **Maintainability**: Share types between native and WASM code
4. **Performance**: Rust types compile to efficient WASM
5. **Future Proof**: Updates to socktop types automatically work in WASM

## Real-World Usage

For production WASM applications:

1. Use this pattern to create a WASM module that exports configuration and serialization functions
2. Handle WebSocket connections in JavaScript using browser APIs
3. Use the exported functions for type-safe message creation and parsing
4. Leverage socktop's structured error handling for robust applications
- **No TLS dependencies**: Completely avoids rustls/TLS
- **No tokio/mio**: Uses only WASM-compatible dependencies

### ❌ WASM Limitations  
- **No native networking**: `tokio-tungstenite` doesn't work in WASM
- **No TLS support**: rustls is not WASM-compatible
- **No file system**: Certificate loading not available

## Architecture for WASM Users

```
WASM Application
├── Use socktop_connector types (✅ this test proves it works)
├── Use browser WebSocket API for networking
└── Handle serialization with socktop message format
```

## Quick Start

1. **Build the WASM package**:
   ```bash
   cd socktop_wasm_test  
   wasm-pack build --target web --out-dir pkg
   ```

2. **Start local server**:
   ```bash
   basic-http-server .
   ```

3. **Open browser** to `http://localhost:8000` and click "Run WASM Test"

## Success Criteria

- ✅ WASM builds without any networking dependencies
- ✅ Core types compile and serialize properly
- ✅ Configuration API works for WebSocket setup
- ✅ No rustls/TLS/tokio/mio dependencies

## Real-World WASM Usage

WASM users should:
1. **Use these types** for message structure compatibility
2. **Use browser WebSocket** for actual connections:
   ```javascript
   const ws = new WebSocket('ws://localhost:3000/ws');
   ws.send(JSON.stringify({ request: 'Metrics' }));
   ```
3. **Handle responses** using the same serialization format

This test proves `socktop_connector`'s **types and patterns** work in WASM, even though the networking must be handled differently.
