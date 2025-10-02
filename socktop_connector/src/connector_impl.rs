//! Modular SocktopConnector implementation using networking and WASM modules.

use crate::config::ConnectorConfig;
use crate::error::{ConnectorError, Result};
use crate::{AgentRequest, AgentResponse};

#[cfg(feature = "networking")]
use crate::networking::{
    WsStream, connect_to_agent, request_disks, request_journal_entries, request_metrics,
    request_process_metrics, request_processes,
};

#[cfg(all(feature = "wasm", not(feature = "networking")))]
use crate::wasm::{connect_to_agent, send_request_and_wait};

#[cfg(all(feature = "wasm", not(feature = "networking")))]
use crate::{DiskInfo, Metrics, ProcessesPayload};

#[cfg(all(feature = "wasm", not(feature = "networking")))]
use web_sys::WebSocket;

/// Main connector for communicating with socktop agents
pub struct SocktopConnector {
    pub config: ConnectorConfig,
    #[cfg(feature = "networking")]
    stream: Option<WsStream>,
    #[cfg(all(feature = "wasm", not(feature = "networking")))]
    websocket: Option<WebSocket>,
}

impl SocktopConnector {
    /// Create a new connector with the given configuration
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "networking")]
            stream: None,
            #[cfg(all(feature = "wasm", not(feature = "networking")))]
            websocket: None,
        }
    }
}

#[cfg(feature = "networking")]
impl SocktopConnector {
    /// Connect to the agent
    pub async fn connect(&mut self) -> Result<()> {
        let stream = connect_to_agent(&self.config).await?;
        self.stream = Some(stream);
        Ok(())
    }

    /// Send a request to the agent and get the response
    pub async fn request(&mut self, request: AgentRequest) -> Result<AgentResponse> {
        let stream = self.stream.as_mut().ok_or(ConnectorError::NotConnected)?;

        match request {
            AgentRequest::Metrics => {
                let metrics = request_metrics(stream)
                    .await
                    .ok_or_else(|| ConnectorError::invalid_response("Failed to get metrics"))?;
                Ok(AgentResponse::Metrics(metrics))
            }
            AgentRequest::Disks => {
                let disks = request_disks(stream)
                    .await
                    .ok_or_else(|| ConnectorError::invalid_response("Failed to get disks"))?;
                Ok(AgentResponse::Disks(disks))
            }
            AgentRequest::Processes => {
                let processes = request_processes(stream)
                    .await
                    .ok_or_else(|| ConnectorError::invalid_response("Failed to get processes"))?;
                Ok(AgentResponse::Processes(processes))
            }
            AgentRequest::ProcessMetrics { pid } => {
                let process_metrics =
                    request_process_metrics(stream, pid).await.ok_or_else(|| {
                        ConnectorError::invalid_response("Failed to get process metrics")
                    })?;
                Ok(AgentResponse::ProcessMetrics(process_metrics))
            }
            AgentRequest::JournalEntries { pid } => {
                let journal_entries =
                    request_journal_entries(stream, pid).await.ok_or_else(|| {
                        ConnectorError::invalid_response("Failed to get journal entries")
                    })?;
                Ok(AgentResponse::JournalEntries(journal_entries))
            }
        }
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Disconnect from the agent
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.close(None).await;
        }
        Ok(())
    }
}

// WASM WebSocket implementation
#[cfg(all(feature = "wasm", not(feature = "networking")))]
impl SocktopConnector {
    /// Connect to the agent using WASM WebSocket
    pub async fn connect(&mut self) -> Result<()> {
        let websocket = connect_to_agent(&self.config).await?;
        self.websocket = Some(websocket);
        Ok(())
    }

    /// Send a request to the agent and get the response
    pub async fn request(&mut self, request: AgentRequest) -> Result<AgentResponse> {
        let ws = self
            .websocket
            .as_ref()
            .ok_or(ConnectorError::NotConnected)?;

        send_request_and_wait(ws, request).await
    }

    /// Check if the connector is connected
    pub fn is_connected(&self) -> bool {
        use crate::utils::WEBSOCKET_OPEN;
        self.websocket
            .as_ref()
            .is_some_and(|ws| ws.ready_state() == WEBSOCKET_OPEN)
    }

    /// Disconnect from the agent
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ws) = self.websocket.take() {
            let _ = ws.close();
        }
        Ok(())
    }

    /// Request metrics from the agent
    pub async fn get_metrics(&mut self) -> Result<Metrics> {
        match self.request(AgentRequest::Metrics).await? {
            AgentResponse::Metrics(metrics) => Ok(metrics),
            _ => Err(ConnectorError::protocol_error(
                "Unexpected response type for metrics",
            )),
        }
    }

    /// Request disk information from the agent
    pub async fn get_disks(&mut self) -> Result<Vec<DiskInfo>> {
        match self.request(AgentRequest::Disks).await? {
            AgentResponse::Disks(disks) => Ok(disks),
            _ => Err(ConnectorError::protocol_error(
                "Unexpected response type for disks",
            )),
        }
    }

    /// Request process information from the agent
    pub async fn get_processes(&mut self) -> Result<ProcessesPayload> {
        match self.request(AgentRequest::Processes).await? {
            AgentResponse::Processes(processes) => Ok(processes),
            _ => Err(ConnectorError::protocol_error(
                "Unexpected response type for processes",
            )),
        }
    }
}

// Stub implementations when neither networking nor wasm is enabled
#[cfg(not(any(feature = "networking", feature = "wasm")))]
impl SocktopConnector {
    /// Connect to the socktop agent endpoint.
    ///
    /// Note: Networking functionality is disabled. Enable the "networking" feature to use this function.
    pub async fn connect(&mut self) -> Result<()> {
        Err(ConnectorError::protocol_error(
            "Networking functionality disabled. Enable the 'networking' feature to connect to agents.",
        ))
    }

    /// Send a request to the agent and await a response.
    ///
    /// Note: Networking functionality is disabled. Enable the "networking" feature to use this function.
    pub async fn request(&mut self, _request: AgentRequest) -> Result<AgentResponse> {
        Err(ConnectorError::protocol_error(
            "Networking functionality disabled. Enable the 'networking' feature to send requests.",
        ))
    }

    /// Close the connection to the agent.
    ///
    /// Note: Networking functionality is disabled. This is a no-op when networking is disabled.
    pub async fn disconnect(&mut self) -> Result<()> {
        Ok(()) // No-op when networking is disabled
    }
}

/// Convenience function to create a connector and connect in one step.
///
/// This function is for non-TLS WebSocket connections (`ws://`). Since there's no
/// certificate involved, hostname verification is not applicable.
///
/// For TLS connections with certificate pinning, use `connect_to_socktop_agent_with_tls()`.
#[cfg(feature = "networking")]
pub async fn connect_to_socktop_agent(url: impl Into<String>) -> Result<SocktopConnector> {
    let config = ConnectorConfig::new(url);
    let mut connector = SocktopConnector::new(config);
    connector.connect().await?;
    Ok(connector)
}

/// Convenience function to create a connector with TLS and connect in one step.
///
/// This function enables TLS with certificate pinning using the provided CA certificate.
/// The `verify_hostname` parameter controls whether the server's hostname is verified
/// against the certificate (recommended for production, can be disabled for testing).
#[cfg(feature = "tls")]
#[cfg(feature = "networking")]
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub async fn connect_to_socktop_agent_with_tls(
    url: impl Into<String>,
    ca_path: impl Into<String>,
    verify_hostname: bool,
) -> Result<SocktopConnector> {
    let config = ConnectorConfig::new(url)
        .with_tls_ca(ca_path)
        .with_hostname_verification(verify_hostname);
    let mut connector = SocktopConnector::new(config);
    connector.connect().await?;
    Ok(connector)
}

/// Convenience function to create a connector with custom WebSocket protocol configuration.
///
/// This function allows you to specify WebSocket protocol version and sub-protocols.
/// Most users should use the simpler `connect_to_socktop_agent()` function instead.
///
/// # Example
/// ```no_run
/// use socktop_connector::connect_to_socktop_agent_with_config;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let connector = connect_to_socktop_agent_with_config(
///     "ws://localhost:3000/ws",
///     Some(vec!["socktop".to_string()]), // WebSocket sub-protocols
///     Some("13".to_string()), // WebSocket version (13 is standard)
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "networking")]
pub async fn connect_to_socktop_agent_with_config(
    url: impl Into<String>,
    protocols: Option<Vec<String>>,
    version: Option<String>,
) -> Result<SocktopConnector> {
    let mut config = ConnectorConfig::new(url);

    if let Some(protocols) = protocols {
        config = config.with_protocols(protocols);
    }

    if let Some(version) = version {
        config = config.with_version(version);
    }

    let mut connector = SocktopConnector::new(config);
    connector.connect().await?;
    Ok(connector)
}
