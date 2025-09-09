use zellij_tile::prelude::*;
use serde::{Deserialize, Serialize};
use socktop_connector::{ConnectorConfig, AgentRequest, SocktopConnector, AgentResponse};
use std::collections::HashMap;

#[derive(Default)]
struct State {
    connector: Option<SocktopConnector>,
    metrics_data: Option<String>,
    connection_status: String,
    error_message: Option<String>,
    update_counter: u32,
}

static mut STATE: State = State {
    connector: None,
    metrics_data: None,
    connection_status: String::new(),
    error_message: None,
    update_counter: 0,
};

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        // Get server URL from plugin config or use default
        let server_url = configuration
            .get("server_url")
            .cloned()
            .unwrap_or_else(|| "ws://localhost:3000/ws".to_string());

        // Initialize connector configuration
        let config = ConnectorConfig::new(&server_url);
        let connector = SocktopConnector::new(config);
        
        unsafe {
            STATE.connector = Some(connector);
            STATE.connection_status = "Connecting...".to_string();
        }

        // Set up periodic updates
        set_timeout(1.0); // Update every second
        
        // Start initial connection
        self.connect_to_socktop();
        
        request_permission(&[
            PermissionType::ReadApplicationState,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                unsafe {
                    STATE.update_counter += 1;
                }
                
                // Request metrics every update cycle
                self.fetch_metrics();
                
                // Set next timer
                set_timeout(2.0); // Update every 2 seconds
                true
            }
            Event::Key(key) => {
                match key {
                    Key::Char('r') => {
                        // Reconnect on 'r' key press
                        self.connect_to_socktop();
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        unsafe {
            let mut output = Vec::new();
            
            // Header
            output.push("╭─ Socktop Metrics Plugin ─╮".to_string());
            output.push(format!("│ Status: {:<18} │", STATE.connection_status));
            output.push("├──────────────────────────╯".to_string());
            
            // Metrics display
            if let Some(ref metrics) = STATE.metrics_data {
                output.push("│ System Metrics:".to_string());
                output.push(format!("│   {}", metrics));
            } else if let Some(ref error) = STATE.error_message {
                output.push("│ Error:".to_string());
                output.push(format!("│   {}", error));
            } else {
                output.push("│ Waiting for data...".to_string());
            }
            
            // Footer
            output.push("│".to_string());
            output.push(format!("│ Updates: {} │ Press 'r' to reconnect", STATE.update_counter));
            output.push("╰──────────────────────────╯".to_string());

            // Print lines within terminal bounds
            for (i, line) in output.iter().enumerate() {
                if i < rows {
                    println!("{}", line);
                }
            }
        }
    }
}

impl State {
    fn connect_to_socktop(&mut self) {
        unsafe {
            if let Some(ref mut connector) = STATE.connector {
                STATE.connection_status = "Connecting...".to_string();
                STATE.error_message = None;
                
                // In a real implementation, you'd use async/await here
                // For this scaffold, we'll simulate the connection
                // Note: Zellij plugins have limitations with async operations
                STATE.connection_status = "Connected".to_string();
            }
        }
    }
    
    fn fetch_metrics(&mut self) {
        unsafe {
            if let Some(ref mut connector) = STATE.connector {
                // In a real implementation, you would:
                // 1. Make an async call to connector.request(AgentRequest::Metrics)
                // 2. Handle the response and update STATE.metrics_data
                // 3. Handle errors and update STATE.error_message
                
                // For this scaffold, we'll simulate a response
                match STATE.update_counter % 4 {
                    0 => {
                        STATE.metrics_data = Some("CPU: 45.2%, Memory: 67.8%".to_string());
                        STATE.connection_status = "Active".to_string();
                    }
                    1 => {
                        STATE.metrics_data = Some("CPU: 32.1%, Memory: 71.3%".to_string());
                    }
                    2 => {
                        STATE.metrics_data = Some("CPU: 58.7%, Memory: 69.1%".to_string());
                    }
                    _ => {
                        STATE.metrics_data = Some("CPU: 41.9%, Memory: 72.4%".to_string());
                    }
                }
            } else {
                STATE.error_message = Some("No connector available".to_string());
                STATE.connection_status = "Disconnected".to_string();
            }
        }
    }
}

// Async helper for real WebSocket operations (commented out for scaffold)
/*
async fn connect_and_fetch(connector: &mut SocktopConnector) -> Result<String, String> {
    // Connect to socktop agent
    connector.connect().await
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    // Request metrics
    let response = connector.request(AgentRequest::Metrics).await
        .map_err(|e| format!("Metrics request failed: {}", e))?;
    
    // Format response
    match response {
        AgentResponse::Metrics(metrics) => {
            Ok(format!("CPU: {:.1}%, Mem: {:.1}%, Host: {}", 
                metrics.cpu_total,
                (metrics.mem_used as f64 / metrics.mem_total as f64) * 100.0,
                metrics.hostname
            ))
        }
        _ => Err("Unexpected response type".to_string())
    }
}
*/
