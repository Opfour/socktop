//! WebSocket connector library for socktop agents.
//!
//! This library provides a high-level interface for connecting to socktop agents
//! over WebSocket connections with support for TLS and certificate pinning.
//!
//! # Quick Start
//!
//! ```no_run
//! use socktop_connector::{connect_to_socktop_agent, AgentRequest, AgentResponse};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
//!     
//!     // Get comprehensive system metrics
//!     if let Ok(AgentResponse::Metrics(metrics)) = connector.request(AgentRequest::Metrics).await {
//!         println!("Hostname: {}", metrics.hostname);
//!         println!("CPU Usage: {:.1}%", metrics.cpu_total);
//!         
//!         // CPU temperature if available
//!         if let Some(temp) = metrics.cpu_temp_c {
//!             println!("CPU Temperature: {:.1}°C", temp);
//!         }
//!         
//!         // Memory usage
//!         println!("Memory: {:.1} GB / {:.1} GB",
//!                  metrics.mem_used as f64 / 1_000_000_000.0,
//!                  metrics.mem_total as f64 / 1_000_000_000.0);
//!         
//!         // Per-core CPU usage
//!         for (i, usage) in metrics.cpu_per_core.iter().enumerate() {
//!             println!("Core {}: {:.1}%", i, usage);
//!         }
//!         
//!         // GPU information
//!         if let Some(gpus) = &metrics.gpus {
//!             for gpu in gpus {
//!                 if let Some(name) = &gpu.name {
//!                     println!("GPU {}: {:.1}% usage", name, gpu.utilization.unwrap_or(0.0));
//!                     if let Some(temp) = gpu.temp {
//!                         println!("  Temperature: {:.1}°C", temp);
//!                     }
//!                 }
//!             }
//!         }
//!     }
//!     
//!     // Get process information
//!     if let Ok(AgentResponse::Processes(processes)) = connector.request(AgentRequest::Processes).await {
//!         println!("Running processes: {}", processes.process_count);
//!         for proc in &processes.top_processes {
//!             println!("  PID {}: {} ({:.1}% CPU, {:.1} MB RAM)",
//!                      proc.pid, proc.name, proc.cpu_usage, proc.mem_bytes as f64 / 1_000_000.0);
//!         }
//!     }
//!     
//!     // Get disk information  
//!     if let Ok(AgentResponse::Disks(disks)) = connector.request(AgentRequest::Disks).await {
//!         for disk in disks {
//!             let used_gb = (disk.total - disk.available) as f64 / 1_000_000_000.0;
//!             let total_gb = disk.total as f64 / 1_000_000_000.0;
//!             println!("Disk {}: {:.1} GB / {:.1} GB", disk.name, used_gb, total_gb);
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! # TLS Support
//!
//! ```no_run
//! use socktop_connector::connect_to_socktop_agent_with_tls;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let connector = connect_to_socktop_agent_with_tls(
//!     "wss://secure-host:3000/ws",
//!     "/path/to/ca.pem",
//!     false  // Enable hostname verification
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Continuous Monitoring
//!
//! For real-time system monitoring, you can make requests in a loop. The agent
//! implements intelligent caching to avoid overwhelming the system:
//!
//! ```no_run
//! use socktop_connector::{connect_to_socktop_agent, AgentRequest, AgentResponse};
//! use tokio::time::{sleep, Duration};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut connector = connect_to_socktop_agent("ws://localhost:3000/ws").await?;
//!     
//!     // Monitor system metrics every 2 seconds
//!     loop {
//!         match connector.request(AgentRequest::Metrics).await {
//!             Ok(AgentResponse::Metrics(metrics)) => {
//!                 // Calculate total network activity across all interfaces
//!                 let total_rx: u64 = metrics.networks.iter().map(|n| n.received).sum();
//!                 let total_tx: u64 = metrics.networks.iter().map(|n| n.transmitted).sum();
//!                 
//!                 println!("CPU: {:.1}%, Memory: {:.1}%, Network: ↓{} ↑{}",
//!                     metrics.cpu_total,
//!                     (metrics.mem_used as f64 / metrics.mem_total as f64) * 100.0,
//!                     format_bytes(total_rx),
//!                     format_bytes(total_tx)
//!                 );
//!             }
//!             Err(e) => {
//!                 eprintln!("Connection error: {}", e);
//!                 break;
//!             }
//!             _ => unreachable!(),
//!         }
//!         
//!         sleep(Duration::from_secs(2)).await;
//!     }
//!     
//!     Ok(())
//! }
//!
//! fn format_bytes(bytes: u64) -> String {
//!     const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
//!     let mut size = bytes as f64;
//!     let mut unit_index = 0;
//!     
//!     while size >= 1024.0 && unit_index < UNITS.len() - 1 {
//!         size /= 1024.0;
//!         unit_index += 1;
//!     }
//!     
//!     format!("{:.1}{}", size, UNITS[unit_index])
//! }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod connector;
pub mod error;
pub mod types;

pub use connector::{
    ConnectorConfig, SocktopConnector, WsStream, connect_to_socktop_agent,
    connect_to_socktop_agent_with_config,
};

#[cfg(feature = "tls")]
pub use connector::connect_to_socktop_agent_with_tls;
pub use error::{ConnectorError, Result};
pub use types::{
    AgentRequest, AgentResponse, DiskInfo, GpuInfo, Metrics, NetworkInfo, ProcessInfo,
    ProcessesPayload,
};
