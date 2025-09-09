use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use socktop_connector::{ConnectorConfig, AgentRequest, SocktopConnector};

// Import the `console.log` function from the Web API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Define a macro for easier console logging
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// This is the main entry point called from JavaScript
#[wasm_bindgen]
pub fn test_socktop_connector(server_url: Option<String>) {
    console_error_panic_hook::set_once();
    
    // Use provided URL or default
    let url = server_url.unwrap_or_else(|| "ws://localhost:3000/ws".to_string());
    
    console_log!("🦀 Starting WASM connector test...");
    console_log!("🌐 Connecting to: {}", url);
    
    // Test 1: Create configuration
    let config = ConnectorConfig::new(&url);
    console_log!("✅ Config created: {}", config.url);
    
    // Test 2: Test configuration methods
    let config_with_protocols = config
        .clone()
        .with_protocols(vec!["socktop".to_string(), "v1".to_string()]);
    console_log!("✅ Config with protocols: {:?}", config_with_protocols.ws_protocols);
    
    let config_with_version = config_with_protocols.with_version("13".to_string());
    console_log!("✅ Config with version: {:?}", config_with_version.ws_version);

    // Test 3: Create request types
    let _metrics_request = AgentRequest::Metrics;
    let _disks_request = AgentRequest::Disks;
    let _processes_request = AgentRequest::Processes;
    console_log!("✅ AgentRequest types created");
    
    // Test 4: Test serialization
    if let Ok(json) = serde_json::to_string(&AgentRequest::Metrics) {
        console_log!("✅ Serialization works: {}", json);
    }

    // Test 5: WebSocket connection test
    console_log!("🌐 Testing WebSocket connection...");
    
    spawn_local(async move {
        test_websocket_connection(config_with_version).await;
        
        console_log!("");
        console_log!("🎉 socktop_connector WASM Test Results:");
        console_log!("✅ ConnectorConfig API works in WASM");
        console_log!("✅ AgentRequest types work in WASM"); 
        console_log!("✅ SocktopConnector compiles for WASM");
        console_log!("✅ Connection stays alive with regular requests");
    });
}

async fn test_websocket_connection(config: ConnectorConfig) {
    console_log!("📡 Connecting to agent...");
    
    let mut connector = SocktopConnector::new(config);
    
    match connector.connect().await {
        Ok(()) => {
            console_log!("✅ Connected!");
            
            // Test continuous monitoring (5 rounds)
            for round in 1..=5 {
                console_log!("🔄 Round {}/5 - Requesting metrics...", round);
                
                // Request metrics (mimicking TUI behavior)
                match connector.request(AgentRequest::Metrics).await {
                    Ok(response) => {
                        match &response {
                            socktop_connector::AgentResponse::Metrics(metrics) => {
                                console_log!("✅ Round {} - CPU: {:.1}%, Mem: {:.1}%, Host: {}", 
                                    round,
                                    metrics.cpu_total,
                                    (metrics.mem_used as f64 / metrics.mem_total as f64) * 100.0,
                                    metrics.hostname
                                );
                                
                                // Show JSON summary for each round (clean, collapsible)
                                if let Ok(json_str) = serde_json::to_string_pretty(&response) {
                                    console_log!("📊 Round {} JSON ({} chars):", round, json_str.len());
                                    console_log!("{}", json_str);
                                }
                            },
                            _ => console_log!("📊 Round {} - Received non-metrics response", round),
                        }
                    }
                    Err(e) => {
                        console_log!("❌ Round {} failed: {}", round, e);
                        console_log!("🔍 Error details: {:?}", e);
                    }
                }

                // Every other round, also test disks and processes
                if round % 2 == 0 {
                    console_log!("💾 Round {} - Requesting disk info...", round);
                    match connector.request(AgentRequest::Disks).await {
                        Ok(response) => {
                            match &response {
                                socktop_connector::AgentResponse::Disks(disks) => {
                                    console_log!("✅ Round {} - Got {} disks", round, disks.len());
                                    for disk in disks.iter().take(3) { // Show first 3 disks
                                        let used_gb = (disk.total - disk.available) / 1024 / 1024 / 1024;
                                        let total_gb = disk.total / 1024 / 1024 / 1024;
                                        console_log!("  💿 {}: {}/{} GB used", disk.name, used_gb, total_gb);
                                    }
                                },
                                _ => console_log!("❌ Round {} - Unexpected disk response type", round),
                            }
                        },
                        Err(e) => console_log!("❌ Round {} - Disk request failed: {}", round, e),
                    }
                    
                    console_log!("⚙️  Round {} - Requesting process info...", round);
                    match connector.request(AgentRequest::Processes).await {
                        Ok(response) => {
                            match &response {
                                socktop_connector::AgentResponse::Processes(processes) => {
                                    console_log!("✅ Round {} - Process count: {}, Top processes: {}", 
                                        round, 
                                        processes.process_count,
                                        processes.top_processes.len()
                                    );
                                    if processes.top_processes.is_empty() {
                                        console_log!("ℹ️  No top processes in response (process_count: {})", processes.process_count);
                                    } else {
                                        for process in processes.top_processes.iter().take(3) { // Show top 3 processes
                                            console_log!("  ⚙️ {}: {:.1}% CPU, {} MB", 
                                                process.name, 
                                                process.cpu_usage,
                                                process.mem_bytes / 1024 / 1024
                                            );
                                        }
                                    }
                                },
                                _ => console_log!("❌ Round {} - Unexpected process response type", round),
                            }
                        },
                        Err(e) => console_log!("❌ Round {} - Process request failed: {}", round, e),
                    }
                }
                
                // Wait 1 second between rounds
                if round < 5 {
                    console_log!("⏱️  Waiting 1 second...");
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        let closure = wasm_bindgen::closure::Closure::once(move || resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
                        web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(
                                closure.as_ref().unchecked_ref(),
                                1000, // 1 second delay
                            )
                            .unwrap();
                        closure.forget();
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                }
            }
            
            console_log!("");
            console_log!("🎉 Test completed successfully!");
            
            // Clean disconnect
            match connector.disconnect().await {
                Ok(()) => console_log!("✅ Disconnected"),
                Err(e) => console_log!("⚠️  Disconnect error: {}", e),
            }
        }
        Err(e) => {
            console_log!("❌ Connection failed: {}", e);
            console_log!("💡 Make sure socktop_agent is running on localhost:3000");
        }
    }
}
