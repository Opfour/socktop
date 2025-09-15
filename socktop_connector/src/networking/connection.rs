//! WebSocket connection handling for native (non-WASM) environments.

use crate::config::ConnectorConfig;
use crate::error::{ConnectorError, Result};

use std::io::BufReader;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use url::Url;

#[cfg(feature = "tls")]
use {
    rustls::{self, ClientConfig},
    rustls::{
        DigitallySignedStruct, RootCertStore, SignatureScheme,
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        crypto::ring,
        pki_types::{CertificateDer, ServerName, UnixTime},
    },
    rustls_pemfile::Item,
    std::fs::File,
    tokio_tungstenite::Connector,
};

pub type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Connect to the agent and return the WS stream
pub async fn connect_to_agent(config: &ConnectorConfig) -> Result<WsStream> {
    #[cfg(feature = "tls")]
    ensure_crypto_provider();

    let mut u = Url::parse(&config.url)?;
    if let Some(ca_path) = &config.tls_ca_path {
        if u.scheme() == "ws" {
            let _ = u.set_scheme("wss");
        }
        return connect_with_ca_and_config(u.as_str(), ca_path, config).await;
    }
    // No TLS - hostname verification is not applicable
    connect_without_ca_and_config(u.as_str(), config).await
}

async fn connect_without_ca_and_config(url: &str, config: &ConnectorConfig) -> Result<WsStream> {
    let mut req = url.into_client_request()?;

    // Apply WebSocket protocol configuration
    if let Some(version) = &config.ws_version {
        req.headers_mut().insert(
            "Sec-WebSocket-Version",
            version
                .parse()
                .map_err(|_| ConnectorError::protocol_error("Invalid WebSocket version"))?,
        );
    }

    if let Some(protocols) = &config.ws_protocols {
        let protocols_str = protocols.join(", ");
        req.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            protocols_str
                .parse()
                .map_err(|_| ConnectorError::protocol_error("Invalid WebSocket protocols"))?,
        );
    }

    let (ws, _) = connect_async(req).await?;
    Ok(ws)
}

#[cfg(feature = "tls")]
async fn connect_with_ca_and_config(
    url: &str,
    ca_path: &str,
    config: &ConnectorConfig,
) -> Result<WsStream> {
    // Initialize the crypto provider for rustls
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut root = RootCertStore::empty();
    let mut reader = BufReader::new(File::open(ca_path)?);
    let mut der_certs = Vec::new();
    while let Ok(Some(item)) = rustls_pemfile::read_one(&mut reader) {
        if let Item::X509Certificate(der) = item {
            der_certs.push(der);
        }
    }
    root.add_parsable_certificates(der_certs);

    let mut cfg = ClientConfig::builder()
        .with_root_certificates(root)
        .with_no_client_auth();

    let mut req = url.into_client_request()?;

    // Apply WebSocket protocol configuration
    if let Some(version) = &config.ws_version {
        req.headers_mut().insert(
            "Sec-WebSocket-Version",
            version
                .parse()
                .map_err(|_| ConnectorError::protocol_error("Invalid WebSocket version"))?,
        );
    }

    if let Some(protocols) = &config.ws_protocols {
        let protocols_str = protocols.join(", ");
        req.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            protocols_str
                .parse()
                .map_err(|_| ConnectorError::protocol_error("Invalid WebSocket protocols"))?,
        );
    }

    if !config.verify_hostname {
        #[derive(Debug)]
        struct NoVerify;
        impl ServerCertVerifier for NoVerify {
            fn verify_server_cert(
                &self,
                _end_entity: &CertificateDer<'_>,
                _intermediates: &[CertificateDer<'_>],
                _server_name: &ServerName,
                _ocsp_response: &[u8],
                _now: UnixTime,
            ) -> std::result::Result<ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }
            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &DigitallySignedStruct,
            ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }
            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &DigitallySignedStruct,
            ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }
            fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
                vec![
                    SignatureScheme::ECDSA_NISTP256_SHA256,
                    SignatureScheme::ED25519,
                    SignatureScheme::RSA_PSS_SHA256,
                ]
            }
        }
        cfg.dangerous().set_certificate_verifier(Arc::new(NoVerify));
        // Note: hostname verification disabled (default). Set SOCKTOP_VERIFY_NAME=1 to enable strict SAN checking.
    }
    let cfg = Arc::new(cfg);
    let (ws, _) = tokio_tungstenite::connect_async_tls_with_config(
        req,
        None,
        config.verify_hostname,
        Some(Connector::Rustls(cfg)),
    )
    .await?;
    Ok(ws)
}

#[cfg(not(feature = "tls"))]
async fn connect_with_ca_and_config(
    _url: &str,
    _ca_path: &str,
    _config: &ConnectorConfig,
) -> Result<WsStream> {
    Err(ConnectorError::tls_error(
        "TLS support not compiled in",
        std::io::Error::new(std::io::ErrorKind::Unsupported, "TLS not available"),
    ))
}

#[cfg(feature = "tls")]
fn ensure_crypto_provider() {
    let _ = ring::default_provider().install_default();
}
