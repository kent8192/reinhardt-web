use std::future::Future;

use reinhardt_pages::reactive::query::{QueryClient, QueryFamily, QueryHandle, QueryKey};
use serde::{Deserialize, Serialize};

/// Typed event emitted when a deployment status may have changed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DeploymentEvent {
	pub deployment_id: u64,
}

/// Authoritative status returned by the deployment query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeploymentStatus {
	Queued,
	Running,
	Succeeded,
	Failed,
}

/// Synchronization state rendered alongside a deployment snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusSync {
	Syncing,
	Current,
	Degraded,
	Stopped,
}

/// Stable family used by the example's status queries.
pub(crate) const DEPLOYMENT_STATUS: QueryFamily<u64, DeploymentStatus, String> =
	QueryFamily::new("examples.realtime.deployment-status.v1");

/// Classifies synchronization separately from age-based query freshness.
pub(crate) fn status_sync<T: Clone + 'static, E: Clone + 'static>(
	query: &QueryHandle<T, E>,
	live: bool,
) -> StatusSync {
	if !live {
		return StatusSync::Stopped;
	}
	if query.error().is_some() || query.refetch_error().is_some() {
		return StatusSync::Degraded;
	}
	if query.data().is_none() || query.is_invalidated() || query.is_fetching() {
		return StatusSync::Syncing;
	}
	StatusSync::Current
}

/// Builds the exact key from the same family descriptor used by the observer.
pub(crate) fn deployment_key<F, Fut>(
	deployment_id: u64,
	fetch_status: F,
) -> QueryKey<DeploymentStatus, String>
where
	F: Fn() -> Fut + 'static,
	Fut: Future<Output = Result<DeploymentStatus, String>> + 'static,
{
	DEPLOYMENT_STATUS
		.query(deployment_id, fetch_status)
		.key()
		.clone()
}

/// Invalidates one deployment after a typed realtime event.
pub(crate) fn invalidate_deployment<F, Fut>(
	client: &QueryClient,
	event: DeploymentEvent,
	fetch_status: F,
) where
	F: Fn() -> Fut + 'static,
	Fut: Future<Output = Result<DeploymentStatus, String>> + 'static,
{
	let key = deployment_key(event.deployment_id, fetch_status);
	client.invalidate(&key);
}

/// Invalidates all deployment status observers after a broad server-side change.
pub(crate) fn invalidate_all_deployments(client: &QueryClient) {
	client.invalidate_family(DEPLOYMENT_STATUS);
}

/// Removes all deployment status entries at an authentication boundary.
pub(crate) fn remove_deployments_on_logout(client: &QueryClient) {
	client.remove_family(DEPLOYMENT_STATUS);
}
