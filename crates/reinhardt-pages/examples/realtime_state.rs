//! Query invalidation recipe for typed realtime deployment events.
//!
//! The event callback invalidates the exact typed key and lets the existing
//! QueryClient fetch authoritative status. A family invalidation is available
//! for broad changes, while logout removes the family at the authentication
//! boundary. Synchronization status is kept separate from age-based freshness.

#[path = "realtime_state/status.rs"]
mod status;

use reinhardt_pages::reactive::query::{QueryClient, QueryDefaults};

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
}
