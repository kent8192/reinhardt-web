//! Protocol-level WebSocket configuration

use tungstenite::protocol::WebSocketConfig as TungsteniteConfig;

/// Create default WebSocketConfig
///
/// Returns a WebSocketConfig with default settings.
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::protocol::default_websocket_config;
///
/// let config = default_websocket_config();
/// ```
pub fn default_websocket_config() -> TungsteniteConfig {
	TungsteniteConfig::default()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	/// Test: Default WebSocketConfig
	#[rstest]
	fn test_default_websocket_config() {
		let _config = default_websocket_config();
		// Default config should be created successfully
	}
}
