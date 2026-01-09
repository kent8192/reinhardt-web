//! DM chat components
//!
//! Provides DM chat interface with real-time message delivery.
//!
//! TODO: Implement WebSocket integration with use_websocket hook

use reinhardt::pages::Signal;
use reinhardt::pages::component::View;
use reinhardt::pages::page;

/// DM chat component
///
/// Provides real-time chat interface using WebSocket connection.
/// Messages are sent and received through WebSocket, with state managed via hooks.
///
/// TODO: Implement WebSocket integration
#[cfg(target_arch = "wasm32")]
pub fn dm_chat(room_id: String) -> View {
	let room_id_signal = Signal::new(room_id.clone());

	page!(|room_id_signal: Signal<String>| {
		div {
			class: "dm-chat-container container mt-4",
			div {
				class: "card mb-3",
				div {
					class: "card-header bg-primary text-white",
					h5 {
						class: "mb-0",
						{ format!("DM Chat - Room: {}", room_id_signal.get()) }
					}
					span {
						class: "badge bg-warning ms-2",
						"WebSocket: Pending Implementation"
					}
				}
			}
			div {
				class: "card mb-3",
				style: "height: 400px; overflow-y: auto;",
				div {
					class: "card-body",
					p {
						class: "text-muted",
						"WebSocket integration is under development..."
					}
					p {
						class: "text-info",
						{ format!("WebSocket URL: ws://localhost:8001/ws/dm?room_id={}", room_id_signal.get()) }
					}
				}
			}
			div {
				class: "card",
				div {
					class: "card-body",
					div {
						class: "input-group",
						input {
							r#type: "text",
							class: "form-control",
							placeholder: "WebSocket integration coming soon...",
						}
						button {
							class: "btn btn-secondary",
							r#type: "button",
							"Send (Disabled)"
						}
					}
				}
			}
		}
	})(room_id_signal)
}

/// DM chat component (server-side placeholder)
#[cfg(not(target_arch = "wasm32"))]
pub fn dm_chat(_room_id: String) -> View {
	page!(|| {
		div {
			class: "dm-chat-container container mt-4",
			p {
				"DM chat is only available in WASM builds"
			}
		}
	})()
}
