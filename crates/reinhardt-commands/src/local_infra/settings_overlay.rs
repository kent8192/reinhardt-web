//! Settings overlay generated from resolved local infrastructure state.

use super::LocalInfraState;
use indexmap::IndexMap;
use reinhardt_conf::settings::sources::{ConfigSource, SourceError};
use serde_json::{Value, json};

/// Config source that overlays resolved local infrastructure endpoints.
#[derive(Debug, Clone)]
pub struct LocalInfraSettingsSource {
	state: LocalInfraState,
}

impl LocalInfraSettingsSource {
	/// Create an overlay source from persisted local infrastructure state.
	pub fn from_state(state: LocalInfraState) -> Self {
		Self { state }
	}
}

impl ConfigSource for LocalInfraSettingsSource {
	fn load(&self) -> Result<IndexMap<String, Value>, SourceError> {
		let mut root = IndexMap::new();

		for service in &self.state.services {
			match service.name.as_str() {
				"postgres" => {
					root.insert(
						"core".to_string(),
						json!({
							"databases": {
								"default": {
									"engine": "postgresql",
									"host": service.host.clone(),
									"port": service.host_port,
									"name": service.metadata.get("database").cloned().unwrap_or(json!("postgres")),
									"user": service.metadata.get("user").cloned().unwrap_or(json!("postgres")),
								}
							}
						}),
					);
				}
				"redis" => {
					let database = service
						.metadata
						.get("database")
						.and_then(Value::as_u64)
						.unwrap_or(0);
					let url = format!(
						"redis://{}:{}/{}",
						service.host, service.host_port, database
					);
					root.insert(
						"cache".to_string(),
						json!({"backend": "redis", "location": url}),
					);
					root.insert("redis_url".to_string(), json!(url));
				}
				_ => {}
			}
		}

		Ok(root)
	}

	fn priority(&self) -> u8 {
		55
	}

	fn description(&self) -> String {
		"local infrastructure overlay".to_string()
	}
}
