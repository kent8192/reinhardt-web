//! Service specifications for local infrastructure.

use serde_json::json;

use super::{LocalServiceState, ServiceRuntimeStatus};

/// Runtime service specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceSpec {
	/// PostgreSQL service.
	Postgres(PostgresService),
	/// Redis service.
	Redis(RedisService),
}

impl ServiceSpec {
	/// Return the logical service name.
	pub fn name(&self) -> &'static str {
		match self {
			Self::Postgres(_) => "postgres",
			Self::Redis(_) => "redis",
		}
	}

	/// Return the preferred host port.
	pub fn requested_port(&self) -> u16 {
		match self {
			Self::Postgres(service) => service.port,
			Self::Redis(service) => service.port,
		}
	}

	/// Return the container image.
	pub fn image(&self) -> &'static str {
		match self {
			Self::Postgres(_) => "postgres:17-alpine",
			Self::Redis(_) => "redis:7-alpine",
		}
	}

	/// Return the internal container port.
	pub fn container_port(&self) -> u16 {
		match self {
			Self::Postgres(_) => 5432,
			Self::Redis(_) => 6379,
		}
	}

	/// Convert this service into persisted runtime state.
	pub fn to_state(&self, container_name: String, host_port: u16) -> LocalServiceState {
		match self {
			Self::Postgres(service) => LocalServiceState {
				name: "postgres".to_string(),
				container_name,
				image: self.image().to_string(),
				host: "127.0.0.1".to_string(),
				host_port,
				container_port: self.container_port(),
				status: ServiceRuntimeStatus::Running,
				metadata: json!({
					"database": service.database.clone(),
					"user": service.user.clone(),
				}),
			},
			Self::Redis(service) => LocalServiceState {
				name: "redis".to_string(),
				container_name,
				image: self.image().to_string(),
				host: "127.0.0.1".to_string(),
				host_port,
				container_port: self.container_port(),
				status: ServiceRuntimeStatus::Running,
				metadata: json!({"database": service.database}),
			},
		}
	}
}

/// PostgreSQL local service settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresService {
	/// Preferred host port.
	pub port: u16,
	/// Database name.
	pub database: String,
	/// Database user.
	pub user: String,
	/// Optional database password.
	pub password: Option<String>,
}

/// Redis local service settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisService {
	/// Preferred host port.
	pub port: u16,
	/// Redis database number.
	pub database: u16,
}
