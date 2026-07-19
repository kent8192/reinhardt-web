//! WebSocket-based HMR notification server.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};

use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};

use super::change_kind::ChangeKind;
use super::config::HmrConfig;
use super::message::HmrMessage;
use super::protocol::{
	BuildDiagnostic, BuildTarget, ClientHello, PatchGeneration, TemplatePatch, TemplatePatchBatch,
};
use super::watcher::FileWatcher;

const MAX_REPLAYED_PATCHES: usize = 64;

struct ReplayState {
	patches: BTreeMap<PatchGeneration, HmrMessage>,
	diagnostics: Option<HmrMessage>,
}

/// The HMR server that watches files and broadcasts changes to connected browsers.
#[derive(Clone)]
pub struct HmrServer {
	config: Arc<HmrConfig>,
	/// Broadcast sender for HMR messages to all connected clients.
	tx: broadcast::Sender<String>,
	/// Tracks connected client count for diagnostics.
	client_count: Arc<Mutex<usize>>,
	replay: Arc<StdMutex<ReplayState>>,
	client_events: broadcast::Sender<String>,
}

impl HmrServer {
	/// Creates a new HMR server with the given configuration.
	pub fn new(config: HmrConfig) -> Self {
		let (tx, _) = broadcast::channel(64);
		let (client_events, _) = broadcast::channel(64);
		Self {
			config: Arc::new(config),
			tx,
			client_count: Arc::new(Mutex::new(0)),
			replay: Arc::new(StdMutex::new(ReplayState {
				patches: BTreeMap::new(),
				diagnostics: None,
			})),
			client_events,
		}
	}

	/// Returns the broadcast sender for sending messages to clients.
	///
	/// This can be used to manually send HMR messages without file watching.
	pub fn sender(&self) -> broadcast::Sender<String> {
		self.tx.clone()
	}

	/// Returns the number of currently connected clients.
	pub async fn client_count(&self) -> usize {
		*self.client_count.lock().await
	}

	/// Returns a receiver for client patch acknowledgements and rejections.
	pub fn client_events(&self) -> broadcast::Receiver<String> {
		self.client_events.subscribe()
	}

	/// Broadcasts and retains a compatible template patch batch for replay.
	pub fn notify_template_patch(&self, batch: TemplatePatchBatch) {
		let generation = batch.generation;
		let message = HmrMessage::TemplatePatchBatch {
			build_id: batch.build_id,
			manifest_digest: batch.manifest_digest,
			generation: batch.generation,
			patches: batch.patches,
		};
		let mut state = self.replay.lock().expect("HMR replay mutex poisoned");
		state.patches.insert(generation, message.clone());
		while state.patches.len() > MAX_REPLAYED_PATCHES {
			let Some(first) = state.patches.keys().next().copied() else {
				break;
			};
			state.patches.remove(&first);
		}
		drop(state);
		self.broadcast(&message);
	}

	/// Broadcasts a build-start notification.
	pub fn notify_build_started(&self, generation: PatchGeneration, targets: Vec<BuildTarget>) {
		self.broadcast(&HmrMessage::BuildStarted {
			generation,
			targets,
		});
	}

	/// Broadcasts and retains normalized build diagnostics for replay.
	pub fn notify_build_diagnostics(&self, diagnostics: Vec<BuildDiagnostic>) {
		let generation = diagnostics
			.iter()
			.map(|diagnostic| diagnostic.generation)
			.max()
			.unwrap_or(PatchGeneration(0));
		let message = HmrMessage::BuildDiagnostics {
			generation,
			diagnostics,
		};
		self.replay
			.lock()
			.expect("HMR replay mutex poisoned")
			.diagnostics = Some(message.clone());
		self.broadcast(&message);
	}

	/// Broadcasts a recovered-build notification.
	pub fn notify_build_recovered(&self, generation: PatchGeneration) {
		self.replay
			.lock()
			.expect("HMR replay mutex poisoned")
			.diagnostics = None;
		self.broadcast(&HmrMessage::BuildRecovered { generation });
	}

	/// Requests a full browser reload without replacing the stored patch replay
	/// state. This is used only after a successful fallback build is ready.
	pub fn notify_full_reload(&self, reason: impl Into<String>) {
		self.broadcast(&HmrMessage::FullReload {
			reason: reason.into(),
		});
	}

	fn broadcast(&self, message: &HmrMessage) {
		if let Ok(json) = message.to_json() {
			let _ = self.tx.send(json);
		}
	}

	/// Starts the HMR server, binding the WebSocket listener and file watcher.
	///
	/// This method spawns background tasks for:
	/// 1. Accepting WebSocket connections
	/// 2. Watching files and broadcasting changes
	///
	/// Returns the bound address for testing purposes.
	pub async fn start(&self) -> Result<std::net::SocketAddr, std::io::Error> {
		if !self.config.enabled {
			// Bind to a port but don't start accepting connections
			let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
			return listener.local_addr();
		}

		let addr = format!("127.0.0.1:{}", self.config.ws_port);
		let listener = TcpListener::bind(&addr).await?;
		let bound_addr = listener.local_addr()?;

		// Spawn WebSocket acceptor task
		let tx = self.tx.clone();
		let client_count = self.client_count.clone();
		let replay = self.replay.clone();
		let client_events = self.client_events.clone();
		tokio::spawn(async move {
			Self::accept_connections(listener, tx, client_count, replay, client_events).await;
		});

		// Spawn file watcher task
		let config_for_watcher = (*self.config).clone();
		let tx_for_watcher = self.tx.clone();
		tokio::spawn(async move {
			Self::watch_files(config_for_watcher, tx_for_watcher).await;
		});

		Ok(bound_addr)
	}

	/// Starts only the WebSocket listener.
	///
	/// This mode is used by outer build loops that already own file watching
	/// and want to broadcast only after their rebuild pipeline succeeds.
	pub async fn start_listener_only(&self) -> Result<std::net::SocketAddr, std::io::Error> {
		if !self.config.enabled {
			let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
			return listener.local_addr();
		}

		let addr = format!("127.0.0.1:{}", self.config.ws_port);
		let listener = TcpListener::bind(&addr).await?;
		let bound_addr = listener.local_addr()?;

		let tx = self.tx.clone();
		let client_count = self.client_count.clone();
		let replay = self.replay.clone();
		let client_events = self.client_events.clone();
		tokio::spawn(async move {
			Self::accept_connections(listener, tx, client_count, replay, client_events).await;
		});

		Ok(bound_addr)
	}

	/// Accepts incoming WebSocket connections and forwards broadcast messages.
	async fn accept_connections(
		listener: TcpListener,
		tx: broadcast::Sender<String>,
		client_count: Arc<Mutex<usize>>,
		replay: Arc<StdMutex<ReplayState>>,
		client_events: broadcast::Sender<String>,
	) {
		loop {
			let (stream, _addr) = match listener.accept().await {
				Ok(conn) => conn,
				Err(err) => {
					tracing::error!(error = %err, "[HMR] Failed to accept connection");
					tokio::time::sleep(std::time::Duration::from_millis(100)).await;
					continue;
				}
			};

			let mut rx = tx.subscribe();
			let client_count = client_count.clone();
			let replay = replay.clone();
			let client_events = client_events.clone();

			tokio::spawn(async move {
				// Increment client count
				{
					let mut count = client_count.lock().await;
					*count += 1;
				}

				// Simple HTTP upgrade and WebSocket frame handling
				// For MVP, we use a minimal text-frame-only WebSocket implementation
				let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await else {
					let mut count = client_count.lock().await;
					*count = count.saturating_sub(1);
					return;
				};

				use futures_util::{SinkExt, StreamExt};
				let (mut write, mut read) = ws_stream.split();

				// Send connected message
				let connected_msg = HmrMessage::Connected.to_json().unwrap_or_default();
				let _ = write
					.send(tokio_tungstenite::tungstenite::Message::Text(
						connected_msg.into(),
					))
					.await;

				// Forward broadcast messages to this client
				loop {
					tokio::select! {
						msg = rx.recv() => {
							match msg {
								Ok(text) => {
									let send_result = write
										.send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
										.await;
									if send_result.is_err() {
										break;
									}
								}
								Err(broadcast::error::RecvError::Lagged(_)) => continue,
								Err(broadcast::error::RecvError::Closed) => break,
							}
						}
						ws_msg = read.next() => {
							match ws_msg {
								Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => break,
								Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
									if let Ok(message) = HmrMessage::from_json(&text) {
										if let Some(hello) = message.client_hello() {
											for replay_message in Self::replay_for_client(&replay, &hello).await {
												let Ok(json) = replay_message.to_json() else { continue };
												if write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await.is_err() {
												break;
												}
											}
										} else if matches!(message, HmrMessage::PatchRejected { .. } | HmrMessage::PatchApplied { .. })
											&& let Ok(json) = message.to_json() {
											let _ = client_events.send(json);
										}
									}
								}
								Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(payload))) => {
									if write.send(tokio_tungstenite::tungstenite::Message::Pong(payload)).await.is_err() {
										break;
									}
								}
								Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(_))) => {}
								Some(Ok(_)) => {
									// Ignore other client messages; HMR is server-push only
								}
								_ => break, // Connection closed or error
							}
						}
					}
				}

				// Decrement client count
				{
					let mut count = client_count.lock().await;
					*count = count.saturating_sub(1);
				}
			});
		}
	}

	async fn replay_for_client(
		replay: &Arc<StdMutex<ReplayState>>,
		hello: &ClientHello,
	) -> Vec<HmrMessage> {
		let state = replay.lock().expect("HMR replay mutex poisoned");
		if state.patches.values().any(|message| {
			matches!(message, HmrMessage::TemplatePatchBatch { patches, .. } if Self::client_has_incompatible_abi(hello, patches))
		}) {
			return vec![HmrMessage::FullReload {
				reason: "HMR client template ABI is stale".to_owned(),
			}];
		}
		let mut messages = state.patches.values().cloned().collect::<Vec<_>>();
		if let Some(diagnostics) = &state.diagnostics {
			messages.push(diagnostics.clone());
		}
		messages
	}

	/// Returns true only when the browser already knows a patched key with a
	/// different ABI. Absent keys are valid: templates register lazily when a
	/// route, branch, or keyed iteration first mounts, and the WASM bridge
	/// retains those patches until it can validate the descriptor.
	fn client_has_incompatible_abi(hello: &ClientHello, patches: &[TemplatePatch]) -> bool {
		patches.iter().any(|patch| {
			hello
				.abi_hashes
				.iter()
				.find(|(key, _)| key == &patch.key)
				.is_some_and(|(_, abi_hash)| abi_hash != &patch.abi_hash)
		})
	}

	/// Watches files and broadcasts change notifications.
	async fn watch_files(config: HmrConfig, tx: broadcast::Sender<String>) {
		let mut watcher = match FileWatcher::new(config) {
			Ok(w) => w,
			Err(e) => {
				tracing::error!(error = %e, "[HMR] Failed to start file watcher");
				return;
			}
		};

		// Track recently sent paths to deduplicate within a short window.
		// Use config.debounce_ms so the clearing interval matches the configured debounce.
		let mut recent_paths: HashSet<String> = HashSet::new();
		let mut debounce_interval = tokio::time::interval(std::time::Duration::from_millis(
			watcher.config().debounce_ms,
		));

		loop {
			tokio::select! {
				Some(event) = watcher.rx.recv() => {
					// Compute a relative URL path by stripping the watch root prefix and
					// normalizing path separators to `/`. This prevents leaking absolute
					// filesystem paths (including Windows backslashes) to the browser.
					let watch_root = watcher.config().watch_paths.first()
						.map(|p| p.as_path());
					let relative_path = watch_root
						.and_then(|root| event.path.strip_prefix(root).ok())
						.unwrap_or(&event.path)
						.to_string_lossy()
						.replace('\\', "/");

					// Skip if we recently sent this path
					if recent_paths.contains(&relative_path) {
						continue;
					}
					recent_paths.insert(relative_path.clone());

					let msg = match event.kind {
						ChangeKind::Css => HmrMessage::CssUpdate { path: relative_path },
						ChangeKind::Rust => HmrMessage::FullReload {
							reason: format!("Rust source changed: {}", event.path.display()),
						},
						ChangeKind::Template => HmrMessage::FullReload {
							reason: format!("Template changed: {}", event.path.display()),
						},
						ChangeKind::Asset => HmrMessage::FullReload {
							reason: format!("Asset changed: {}", event.path.display()),
						},
						ChangeKind::Unknown => HmrMessage::FullReload {
							reason: format!("File changed: {}", event.path.display()),
						},
					};

					if let Ok(json) = msg.to_json() {
						let _ = tx.send(json);
					}
				}
				_ = debounce_interval.tick() => {
					// Clear recent paths periodically to allow re-notification
					recent_paths.clear();
				}
			}
		}
	}

	/// Broadcasts a change notification for the given path.
	///
	/// This is useful for programmatic triggering of HMR events.
	pub fn notify_change(&self, path: &str, kind: ChangeKind) {
		let msg = match kind {
			ChangeKind::Css => HmrMessage::CssUpdate {
				path: path.to_string(),
			},
			_ => HmrMessage::FullReload {
				reason: format!("File changed: {}", path),
			},
		};

		self.broadcast(&msg);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_hmr_server_creation() {
		// Arrange
		let config = HmrConfig::default();

		// Act
		let server = HmrServer::new(config);

		// Assert
		assert!(server.tx.receiver_count() == 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_hmr_server_client_count_initially_zero() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);

		// Act & Assert
		assert_eq!(server.client_count().await, 0);
	}

	#[rstest]
	fn test_notify_change_css() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let mut rx = server.sender().subscribe();

		// Act
		server.notify_change("styles/main.css", ChangeKind::Css);

		// Assert
		let msg = rx.try_recv().unwrap();
		let parsed: HmrMessage = serde_json::from_str(&msg).unwrap();
		assert_eq!(
			parsed,
			HmrMessage::CssUpdate {
				path: "styles/main.css".to_string()
			}
		);
	}

	#[rstest]
	fn test_notify_change_rust() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let mut rx = server.sender().subscribe();

		// Act
		server.notify_change("src/main.rs", ChangeKind::Rust);

		// Assert
		let msg = rx.try_recv().unwrap();
		let parsed: HmrMessage = serde_json::from_str(&msg).unwrap();
		assert!(matches!(parsed, HmrMessage::FullReload { .. }));
	}

	#[rstest]
	#[tokio::test]
	async fn test_hmr_server_start_disabled() {
		// Arrange
		let config = HmrConfig::builder().enabled(false).build();
		let server = HmrServer::new(config);

		// Act - should bind without error even when disabled
		let result = server.start().await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn test_hmr_server_start_binds_port() {
		// Arrange
		let config = HmrConfig::builder().ws_port(0).build(); // port 0 = OS-assigned
		let server = HmrServer::new(config);

		// Act
		let addr = server.start().await.unwrap();

		// Assert
		assert_ne!(addr.port(), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_hmr_server_start_listener_only_binds_port() {
		// Arrange
		let config = HmrConfig::builder().ws_port(0).build();
		let server = HmrServer::new(config);

		// Act
		let addr = server.start_listener_only().await.unwrap();

		// Assert
		assert_ne!(addr.port(), 0);
		assert_eq!(server.client_count().await, 0);
	}

	// --- Additional notify_change variant coverage ---

	#[rstest]
	fn test_notify_change_template() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let mut rx = server.sender().subscribe();

		// Act
		server.notify_change("templates/index.html", ChangeKind::Template);

		// Assert
		let msg = rx.try_recv().unwrap();
		let parsed: HmrMessage = serde_json::from_str(&msg).unwrap();
		assert!(matches!(parsed, HmrMessage::FullReload { .. }));
	}

	#[rstest]
	fn test_notify_change_asset() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let mut rx = server.sender().subscribe();

		// Act
		server.notify_change("static/logo.png", ChangeKind::Asset);

		// Assert
		let msg = rx.try_recv().unwrap();
		let parsed: HmrMessage = serde_json::from_str(&msg).unwrap();
		assert!(matches!(parsed, HmrMessage::FullReload { .. }));
	}

	#[rstest]
	fn test_notify_change_unknown() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let mut rx = server.sender().subscribe();

		// Act
		server.notify_change("Makefile", ChangeKind::Unknown);

		// Assert
		let msg = rx.try_recv().unwrap();
		let parsed: HmrMessage = serde_json::from_str(&msg).unwrap();
		assert!(matches!(parsed, HmrMessage::FullReload { .. }));
	}

	#[rstest]
	fn test_notify_change_no_receivers_does_not_panic() {
		// Arrange — no subscriber; send should silently succeed (channel ignores missing receivers)
		let config = HmrConfig::default();
		let server = HmrServer::new(config);

		// Act & Assert — must not panic
		server.notify_change("src/main.rs", ChangeKind::Rust);
	}

	#[rstest]
	fn test_sender_multiple_receivers_get_same_message() {
		// Arrange
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let mut rx1 = server.sender().subscribe();
		let mut rx2 = server.sender().subscribe();
		let mut rx3 = server.sender().subscribe();

		// Act
		server.notify_change("styles/app.css", ChangeKind::Css);

		// Assert — all receivers get the same message
		let msg1 = rx1.try_recv().unwrap();
		let msg2 = rx2.try_recv().unwrap();
		let msg3 = rx3.try_recv().unwrap();
		assert_eq!(msg1, msg2);
		assert_eq!(msg2, msg3);
	}

	#[rstest]
	fn test_sender_cloned_independently() {
		// Cloned senders must share the same broadcast channel
		let config = HmrConfig::default();
		let server = HmrServer::new(config);
		let sender_clone = server.sender();
		let mut rx = server.sender().subscribe();

		// Send via cloned sender
		let msg = HmrMessage::Connected.to_json().unwrap();
		let _ = sender_clone.send(msg.clone());

		// Assert — receiver on original channel gets the message
		assert_eq!(rx.try_recv().unwrap(), msg);
	}
}
