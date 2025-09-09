//! Error types for socktop_connector

use thiserror::Error;

/// Errors that can occur when using socktop_connector
#[derive(Error, Debug)]
pub enum ConnectorError {
    /// WebSocket connection failed
    #[cfg(feature = "networking")]
    #[error("WebSocket connection failed: {source}")]
    ConnectionFailed {
        #[from]
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// URL parsing error
    #[cfg(feature = "networking")]
    #[error("Invalid URL: {url}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },

    /// TLS certificate error
    #[error("TLS certificate error: {message}")]
    TlsError {
        message: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Certificate file not found or invalid
    #[error("Certificate file error at '{path}': {message}")]
    CertificateError { path: String, message: String },

    /// Invalid server response format
    #[error("Invalid response from server: {message}")]
    InvalidResponse { message: String },

    /// JSON parsing error
    #[error("JSON parsing error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    /// Request/response protocol error
    #[error("Protocol error: {message}")]
    ProtocolError { message: String },

    /// Connection is not established
    #[error("Not connected to server")]
    NotConnected,

    /// Connection was closed unexpectedly
    #[error("Connection closed: {reason}")]
    ConnectionClosed { reason: String },

    /// IO error (network, file system, etc.)
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// Compression/decompression error
    #[error("Compression error: {message}")]
    CompressionError { message: String },

    /// Protocol Buffer parsing error
    #[error("Protocol buffer error: {source}")]
    ProtobufError {
        #[from]
        source: prost::DecodeError,
    },
}

/// Result type alias for connector operations
pub type Result<T> = std::result::Result<T, ConnectorError>;

impl ConnectorError {
    /// Create a TLS error with context
    pub fn tls_error(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::TlsError {
            message: message.into(),
            source: Box::new(source),
        }
    }

    /// Create a certificate error
    pub fn certificate_error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::CertificateError {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a protocol error
    pub fn protocol_error(message: impl Into<String>) -> Self {
        Self::ProtocolError {
            message: message.into(),
        }
    }

    /// Create an invalid response error
    pub fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            message: message.into(),
        }
    }

    /// Create a connection closed error
    pub fn connection_closed(reason: impl Into<String>) -> Self {
        Self::ConnectionClosed {
            reason: reason.into(),
        }
    }

    /// Create a compression error
    pub fn compression_error(message: impl Into<String>) -> Self {
        Self::CompressionError {
            message: message.into(),
        }
    }

    /// Create a serialization error (wraps JSON error)
    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::ProtocolError {
            message: message.into(),
        }
    }
}

#[cfg(feature = "networking")]
impl From<url::ParseError> for ConnectorError {
    fn from(source: url::ParseError) -> Self {
        Self::InvalidUrl {
            url: "unknown".to_string(), // We don't have the URL in the error context
            source,
        }
    }
}
