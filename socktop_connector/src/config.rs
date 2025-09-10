//! Configuration for socktop WebSocket connections.

/// Configuration for connecting to a socktop agent.
#[derive(Debug, Clone)]
pub struct ConnectorConfig {
    pub url: String,
    pub tls_ca_path: Option<String>,
    pub verify_hostname: bool,
    pub ws_protocols: Option<Vec<String>>,
    pub ws_version: Option<String>,
}

impl ConnectorConfig {
    /// Create a new connector configuration with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            tls_ca_path: None,
            verify_hostname: false,
            ws_protocols: None,
            ws_version: None,
        }
    }

    /// Set the path to a custom TLS CA certificate file.
    pub fn with_tls_ca(mut self, ca_path: impl Into<String>) -> Self {
        self.tls_ca_path = Some(ca_path.into());
        self
    }

    /// Enable or disable hostname verification for TLS connections.
    pub fn with_hostname_verification(mut self, verify: bool) -> Self {
        self.verify_hostname = verify;
        self
    }

    /// Set WebSocket sub-protocols to negotiate.
    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.ws_protocols = Some(protocols);
        self
    }

    /// Set WebSocket protocol version (default is "13").
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.ws_version = Some(version.into());
        self
    }
}
