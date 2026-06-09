//! State persistence for local infrastructure containers.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Persisted state for a project's local infrastructure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalInfraState {
	/// Stable project identifier used for local infrastructure names.
	pub project_id: String,
	/// Local infrastructure profile name.
	pub profile: String,
	/// Services tracked for this project and profile.
	pub services: Vec<LocalServiceState>,
}

/// Persisted runtime state for one local service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalServiceState {
	/// Logical service name.
	pub name: String,
	/// Runtime container name.
	pub container_name: String,
	/// Container image reference.
	pub image: String,
	/// Host address used to reach the service.
	pub host: String,
	/// Host port exposed for the service.
	pub host_port: u16,
	/// Port exposed inside the container.
	pub container_port: u16,
	/// Last observed runtime status.
	pub status: ServiceRuntimeStatus,
	/// Service-specific persisted metadata.
	pub metadata: serde_json::Value,
}

/// Runtime status recorded for a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRuntimeStatus {
	/// The service is running.
	Running,
	/// The service is stopped.
	Stopped,
	/// The service container is missing.
	Missing,
	/// The persisted state no longer matches runtime state.
	Stale,
}

/// Project-local state store.
#[derive(Debug, Clone)]
pub struct StateStore {
	path: PathBuf,
}

impl StateStore {
	/// Create a state store rooted at a project directory.
	pub fn new(project_root: impl AsRef<Path>) -> Self {
		Self {
			path: project_root
				.as_ref()
				.join(".reinhardt")
				.join("local-infra.json"),
		}
	}

	/// Return the state file path.
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Load persisted state, returning `None` when the state file does not exist.
	pub fn load(&self) -> io::Result<Option<LocalInfraState>> {
		if !self.path.exists() {
			return Ok(None);
		}
		let bytes = fs::read(&self.path)?;
		let state = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
		Ok(Some(state))
	}

	/// Save state atomically through a temporary file in the state directory.
	pub fn save(&self, state: &LocalInfraState) -> io::Result<()> {
		if let Some(parent) = self.path.parent() {
			fs::create_dir_all(parent)?;
		}
		let tmp = self.path.with_extension("json.tmp");
		let bytes = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
		fs::write(&tmp, bytes)?;
		fs::rename(tmp, &self.path)?;
		Ok(())
	}

	/// Remove the state file if it exists.
	pub fn remove(&self) -> io::Result<()> {
		match fs::remove_file(&self.path) {
			Ok(()) => Ok(()),
			Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
			Err(err) => Err(err),
		}
	}
}
