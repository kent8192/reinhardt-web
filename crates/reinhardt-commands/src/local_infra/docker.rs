//! Docker API adapter for local infrastructure.

use async_trait::async_trait;
use bollard::Docker;
use bollard::errors::Error as BollardError;
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::query_parameters::{
	CreateContainerOptionsBuilder, ListContainersOptionsBuilder, RemoveContainerOptionsBuilder,
	StartContainerOptions,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Docker API operation error.
#[derive(Debug, thiserror::Error)]
pub enum DockerError {
	/// Docker backend returned an error.
	#[error("{0}")]
	Backend(String),
}

impl From<BollardError> for DockerError {
	fn from(err: BollardError) -> Self {
		Self::Backend(err.to_string())
	}
}

/// Result type for Docker API operations.
pub(crate) type DockerResult<T> = Result<T, DockerError>;

/// Runtime request for a detached local infrastructure container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerRunSpec {
	/// Container name.
	pub name: String,
	/// Image reference.
	pub image: String,
	/// Host port to bind.
	pub host_port: u16,
	/// Container port to expose.
	pub container_port: u16,
	/// Container environment variables.
	pub env: Vec<(String, String)>,
}

/// One Docker API operation captured by a fake engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerCall {
	/// Container existence lookup.
	ContainerExists {
		/// Container name.
		name: String,
	},
	/// Forced container removal.
	RemoveContainer {
		/// Container name.
		name: String,
	},
	/// Detached container creation and start.
	RunDetached {
		/// Detached container run request.
		spec: DockerRunSpec,
	},
}

/// Docker backend used by local infrastructure commands.
#[async_trait]
pub trait DockerEngine: Clone + Send + Sync + 'static {
	/// Return whether a container with this exact name exists.
	async fn container_exists(&self, name: &str) -> DockerResult<bool>;

	/// Remove a container by name, ignoring missing containers.
	async fn remove_container(&self, name: &str) -> DockerResult<()>;

	/// Create and start a detached container.
	async fn run_detached(&self, spec: DockerRunSpec) -> DockerResult<()>;
}

/// Bollard-backed Docker Engine API client.
#[derive(Clone)]
pub struct BollardDockerEngine {
	docker: Docker,
}

impl BollardDockerEngine {
	/// Connect to the local Docker backend using bollard's default discovery.
	pub fn local() -> DockerResult<Self> {
		let docker = Docker::connect_with_local_defaults()?;
		Ok(Self { docker })
	}
}

#[async_trait]
impl DockerEngine for BollardDockerEngine {
	async fn container_exists(&self, name: &str) -> DockerResult<bool> {
		let mut filters = HashMap::new();
		filters.insert("name", vec![name]);
		let containers = self
			.docker
			.list_containers(Some(
				ListContainersOptionsBuilder::default()
					.all(true)
					.filters(&filters)
					.build(),
			))
			.await
			.map_err(DockerError::from)?;

		Ok(containers.into_iter().any(|container| {
			container
				.names
				.unwrap_or_default()
				.iter()
				.any(|container_name| container_name == &format!("/{name}"))
		}))
	}

	async fn remove_container(&self, name: &str) -> DockerResult<()> {
		if !self.container_exists(name).await? {
			return Ok(());
		}
		self.docker
			.remove_container(
				name,
				Some(
					RemoveContainerOptionsBuilder::default()
						.force(true)
						.v(true)
						.build(),
				),
			)
			.await
			.or_else(|err| match err {
				BollardError::DockerResponseServerError {
					status_code: 404, ..
				} => Ok(()),
				err => Err(DockerError::from(err)),
			})
	}

	async fn run_detached(&self, spec: DockerRunSpec) -> DockerResult<()> {
		let exposed_port = format!("{}/tcp", spec.container_port);
		let mut port_bindings = HashMap::new();
		port_bindings.insert(
			exposed_port.clone(),
			Some(vec![PortBinding {
				host_ip: Some("127.0.0.1".to_string()),
				host_port: Some(spec.host_port.to_string()),
			}]),
		);

		let body = ContainerCreateBody {
			image: Some(spec.image.clone()),
			env: Some(
				spec.env
					.iter()
					.map(|(key, value)| format!("{key}={value}"))
					.collect(),
			),
			exposed_ports: Some(vec![exposed_port]),
			host_config: Some(HostConfig {
				auto_remove: Some(true),
				port_bindings: Some(port_bindings),
				..Default::default()
			}),
			..Default::default()
		};

		let options = CreateContainerOptionsBuilder::default()
			.name(&spec.name)
			.build();
		let container = self
			.docker
			.create_container(Some(options), body)
			.await
			.map_err(DockerError::from)?;
		self.docker
			.start_container(&container.id, None::<StartContainerOptions>)
			.await
			.map_err(DockerError::from)
	}
}

/// Fake Docker engine for tests.
#[derive(Debug, Clone)]
pub struct FakeDockerEngine {
	calls: Arc<Mutex<Vec<DockerCall>>>,
	exists: Arc<Mutex<Vec<bool>>>,
}

impl FakeDockerEngine {
	/// Create a fake engine that returns existence checks in order.
	pub fn new(exists: Vec<bool>) -> Self {
		Self {
			calls: Arc::new(Mutex::new(Vec::new())),
			exists: Arc::new(Mutex::new(exists)),
		}
	}

	/// Return captured Docker API operations.
	pub fn calls(&self) -> Vec<DockerCall> {
		self.calls.lock().expect("calls lock").clone()
	}
}

#[async_trait]
impl DockerEngine for FakeDockerEngine {
	async fn container_exists(&self, name: &str) -> DockerResult<bool> {
		self.calls
			.lock()
			.expect("calls lock")
			.push(DockerCall::ContainerExists {
				name: name.to_string(),
			});
		let mut exists = self.exists.lock().expect("exists lock");
		Ok(if exists.is_empty() {
			false
		} else {
			exists.remove(0)
		})
	}

	async fn remove_container(&self, name: &str) -> DockerResult<()> {
		self.calls
			.lock()
			.expect("calls lock")
			.push(DockerCall::RemoveContainer {
				name: name.to_string(),
			});
		Ok(())
	}

	async fn run_detached(&self, spec: DockerRunSpec) -> DockerResult<()> {
		self.calls
			.lock()
			.expect("calls lock")
			.push(DockerCall::RunDetached { spec });
		Ok(())
	}
}
