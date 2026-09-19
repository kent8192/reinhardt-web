//! Query invalidation recipe for typed realtime deployment events.
//!
//! The event callback invalidates the exact typed key and lets the existing
//! QueryClient fetch authoritative status. A family invalidation is available
//! for broad changes, while logout removes the family at the authentication
//! boundary. Synchronization status is kept separate from age-based freshness.

#[path = "realtime_state/status.rs"]
mod status;
#[path = "realtime_state/logs.rs"]
mod logs;
#[cfg(test)]
#[path = "realtime_state/tests.rs"]
mod tests;

use reinhardt_pages::reactive::query::{QueryClient, QueryDefaults};

use logs::{LogGap, LogLimits, LogRow, LogSnapshot, LogState, ReconcileToken};

fn main() {
	let client = QueryClient::new(QueryDefaults::default());
	let _status_classifier = status::status_sync::<status::DeploymentStatus, String>;

	status::invalidate_deployment(
		&client,
		status::DeploymentEvent { deployment_id: 42 },
		|| async { Ok(status::DeploymentStatus::Running) },
	);
	status::invalidate_all_deployments(&client);
	status::remove_deployments_on_logout(&client);
	demonstrate_log_model();
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
	logs.fail(
		retry,
		LogGap::CursorExpired,
	);
	logs.stop();
}
