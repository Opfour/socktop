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
                // Try to get real metrics from socktop agent
                match self.try_get_metrics(connector) {
                    Ok(metrics_text) => {
                        STATE.metrics_data = Some(metrics_text);
                        STATE.connection_status = "Active".to_string();
                        STATE.error_message = None;
                    }
                    Err(error) => {
                        STATE.error_message = Some(error);
                        STATE.connection_status = "Error".to_string();
                    }
                }
            } else {
                STATE.error_message = Some("No connector available".to_string());
                STATE.connection_status = "Disconnected".to_string();
            }
        }
    }

    fn try_get_metrics(&mut self, connector: &mut SocktopConnector) -> Result<String, String> {
        // Note: This is synchronous for simplicity. In a real plugin you might need
        // to handle async operations differently depending on Zellij's threading model.
        
        // For now, we'll use a blocking approach or return a placeholder
        // that indicates we're trying to connect
        
        // Attempt connection if not connected
        if let Err(e) = futures::executor::block_on(connector.connect()) {
            return Err(format!("Connection failed: {}", e));
        }
        
        // Request metrics
        match futures::executor::block_on(connector.request(AgentRequest::Metrics)) {
            Ok(AgentResponse::Metrics(metrics)) => {
                Ok(format!(
                    "CPU: {:.1}% | Mem: {:.1}% | Host: {} | Load: {:.2}",
                    metrics.cpu_total,
                    (metrics.mem_used as f64 / metrics.mem_total as f64) * 100.0,
                    metrics.hostname,
                    metrics.load_avg_1m
                ))
            }
            Ok(_) => Err("Unexpected response type".to_string()),
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }
}
