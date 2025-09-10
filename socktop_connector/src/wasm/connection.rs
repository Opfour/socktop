//! WebSocket connection handling for WASM environments.

use crate::config::ConnectorConfig;
use crate::error::{ConnectorError, Result};
use crate::utils::{WEBSOCKET_CLOSED, WEBSOCKET_CLOSING, WEBSOCKET_OPEN};

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

/// Connect to the agent using WASM WebSocket
pub async fn connect_to_agent(config: &ConnectorConfig) -> Result<WebSocket> {
    let websocket = WebSocket::new(&config.url).map_err(|e| {
        ConnectorError::protocol_error(format!("Failed to create WebSocket: {e:?}"))
    })?;

    // Set binary type for proper message handling
    websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // Wait for connection to be ready with proper async delays
    let start_time = js_sys::Date::now();
    let timeout_ms = 10000.0; // 10 second timeout (increased from 5)

    // Poll connection status until ready or timeout
    loop {
        let ready_state = websocket.ready_state();

        if ready_state == WEBSOCKET_OPEN {
            // OPEN - connection is ready
            break;
        } else if ready_state == WEBSOCKET_CLOSED {
            // CLOSED
            return Err(ConnectorError::protocol_error(
                "WebSocket connection closed",
            ));
        } else if ready_state == WEBSOCKET_CLOSING {
            // CLOSING
            return Err(ConnectorError::protocol_error("WebSocket is closing"));
        }

        // Check timeout
        let now = js_sys::Date::now();
        if now - start_time > timeout_ms {
            return Err(ConnectorError::protocol_error(
                "WebSocket connection timeout",
            ));
        }

        // Proper async delay using setTimeout Promise
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let closure = Closure::once(move || resolve.call0(&JsValue::UNDEFINED));
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    100, // 100ms delay between polls
                )
                .unwrap();
            closure.forget();
        });

        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    Ok(websocket)
}
