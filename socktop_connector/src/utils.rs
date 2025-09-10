//! Shared utilities for both networking and WASM implementations.

#[cfg(any(feature = "networking", feature = "wasm"))]
use flate2::read::GzDecoder;
#[cfg(any(feature = "networking", feature = "wasm"))]
use std::io::Read;

use crate::error::{ConnectorError, Result};

// WebSocket state constants
#[cfg(feature = "wasm")]
#[allow(dead_code)]
pub const WEBSOCKET_CONNECTING: u16 = 0;
#[cfg(feature = "wasm")]
#[allow(dead_code)]
pub const WEBSOCKET_OPEN: u16 = 1;
#[cfg(feature = "wasm")]
#[allow(dead_code)]
pub const WEBSOCKET_CLOSING: u16 = 2;
#[cfg(feature = "wasm")]
#[allow(dead_code)]
pub const WEBSOCKET_CLOSED: u16 = 3;

// Gzip magic header constants
pub const GZIP_MAGIC_1: u8 = 0x1f;
pub const GZIP_MAGIC_2: u8 = 0x8b;

/// Unified gzip decompression to string for both networking and WASM
#[cfg(any(feature = "networking", feature = "wasm"))]
pub fn gunzip_to_string(bytes: &[u8]) -> Result<String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .map_err(|e| ConnectorError::protocol_error(format!("Gzip decompression failed: {e}")))?;
    Ok(decompressed)
}

/// Unified gzip decompression to bytes for both networking and WASM
#[cfg(any(feature = "networking", feature = "wasm"))]
pub fn gunzip_to_vec(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| ConnectorError::protocol_error(format!("Gzip decompression failed: {e}")))?;
    Ok(decompressed)
}

/// Unified gzip detection for both networking and WASM
#[cfg(any(feature = "networking", feature = "wasm"))]
pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == GZIP_MAGIC_1 && bytes[1] == GZIP_MAGIC_2
}

/// Unified debug logging for both networking and WASM modes
#[cfg(any(feature = "networking", feature = "wasm"))]
#[allow(dead_code)]
pub fn log_debug(message: &str) {
    #[cfg(feature = "networking")]
    if std::env::var("SOCKTOP_DEBUG").ok().as_deref() == Some("1") {
        eprintln!("{message}");
    }

    #[cfg(all(feature = "wasm", not(feature = "networking")))]
    eprintln!("{message}");
}
