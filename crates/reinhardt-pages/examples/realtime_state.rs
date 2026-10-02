//! Query invalidation recipe for typed realtime deployment events.
//!
//! The event callback invalidates the exact typed key and lets the existing
//! QueryClient fetch authoritative status. A family invalidation is available
//! for broad changes, while logout removes the family at the authentication
//! boundary. Synchronization status is kept separate from age-based freshness.

#[path = "realtime_state/logs.rs"]
mod logs;
#[path = "realtime_state/status.rs"]
mod status;
#[cfg(test)]
#[path = "realtime_state/tests.rs"]
mod tests;

use reinhardt_pages::reactive::query::{QueryClient, QueryDefaults};
use reinhardt_pages::reactive::{
	ReactiveScope,
	hooks::{WebSocketSubscriptionOptions, use_websocket},
};

use logs::{LogGap, LogLimits, LogRow, LogSnapshot, LogState, ReconcileToken};

fn main() {
	ReactiveScope::run(|| {
		let client = QueryClient::new(QueryDefaults::default());
		let _status_classifier = status::status_sync::<status::DeploymentStatus, String>;

		status::invalidate_deployment(
			&client,
			status::DeploymentEvent { deployment_id: 42 },
			|| async { Ok(status::DeploymentStatus::Running) },
		);
		status::invalidate_all_deployments(&client);
		status::remove_deployments_on_logout(&client);
		let socket = use_websocket("wss://example.invalid/events", Default::default());
		let callback_client = client.clone();
		let _subscription = socket.subscribe_json(
			WebSocketSubscriptionOptions::new(std::num::NonZeroUsize::new(16 * 1024).unwrap()),
			move |event: status::DeploymentEvent| {
				status::invalidate_deployment(&callback_client, event, || async {
					Ok(status::DeploymentStatus::Running)
				})
			},
			|error| eprintln!("Realtime error: {error:?}"),
		);
		demonstrate_log_model();
	});
}

fn demonstrate_log_model() {
	let token = ReconcileToken {
		selection: 42,
		connection: 1,
		attempt: 1,
	};
	let mut logs = LogState::new(LogLimits::default());
	logs.begin(token);
	let _ = logs.push(
		token,
		LogRow {
			id: Some(1),
			cursor: Some(1),
			text: "started".to_owned(),
		},
	);
	let _ = logs.finish(
		token,
		LogSnapshot {
			rows: vec![LogRow {
				id: Some(1),
				cursor: Some(1),
				text: "started".to_owned(),
			}],
			watermark: Some(1),
		},
	);
	let _ = (logs.rows(), logs.sync(), logs.retention_removed());
	logs.fail(token, LogGap::FetchFailed);
	let retry = ReconcileToken {
		connection: 2,
		attempt: 2,
		..token
	};
	logs.begin(retry);
	logs.fail(retry, LogGap::CursorExpired);
	logs.stop();
}
