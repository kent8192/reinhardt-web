//! Local infrastructure configuration derived from resolved settings.

use super::{PostgresService, RedisService, ServiceSpec};

/// Database settings normalized for local infrastructure derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseInfraInput {
	/// Database engine.
	pub engine: String,
	/// Configured database host.
	pub host: String,
	/// Preferred host port.
	pub port: u16,
	/// Database name.
	pub name: String,
	/// Database user.
	pub user: String,
	/// Optional database password.
	pub password: Option<String>,
}

/// Redis settings normalized for local infrastructure derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisInfraInput {
	/// Redis URL.
	pub url: String,
}

/// Derived local infrastructure plan.
#[derive(Debug, Clone)]
pub struct LocalInfraConfig {
	/// Stable project identifier.
	pub project_id: String,
	/// Settings profile.
	pub profile: String,
	/// Services to provision.
	pub services: Vec<ServiceSpec>,
}

impl LocalInfraConfig {
	/// Derive local infrastructure services from normalized inputs.
	pub fn derive(
		project_id: impl Into<String>,
		profile: impl Into<String>,
		database: Option<DatabaseInfraInput>,
		redis: Option<RedisInfraInput>,
	) -> Result<Self, String> {
		let mut services = Vec::new();

		if let Some(database) = database
			&& database.engine.contains("postgres")
		{
			services.push(ServiceSpec::Postgres(PostgresService {
				port: database.port,
				database: database.name,
				user: database.user,
				password: database.password,
			}));
		}

		if let Some(redis) = redis {
			let url = url::Url::parse(&redis.url).map_err(|err| err.to_string())?;
			if url.scheme() == "redis" {
				services.push(ServiceSpec::Redis(RedisService {
					port: url.port().unwrap_or(6379),
					database: url.path().trim_start_matches('/').parse().unwrap_or(0),
				}));
			}
		}

		Ok(Self {
			project_id: project_id.into(),
			profile: profile.into(),
			services,
		})
	}
}
