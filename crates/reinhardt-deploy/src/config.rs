use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{DeployError, DeployResult};

/// Top-level deployment configuration parsed from `deploy.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeployConfig {
	pub project: ProjectConfig,
	pub provider: ProviderConfig,
	#[serde(default)]
	pub app: AppConfig,
	pub database: Option<DatabaseConfig>,
	pub nosql: Option<NoSqlConfig>,
	pub cache: Option<CacheConfig>,
	pub frontend: Option<FrontendConfig>,
	#[serde(rename = "static")]
	pub static_files: Option<StaticConfig>,
	pub media: Option<MediaConfig>,
	pub tasks: Option<TasksConfig>,
	pub websockets: Option<WebSocketsConfig>,
	pub mail: Option<MailConfig>,
	pub secrets: Option<SecretsConfig>,
	#[serde(default)]
	pub network: NetworkConfig,
	#[serde(default)]
	pub terraform: TerraformConfig,
	pub preview: Option<PreviewConfig>,
	pub git: Option<GitConfig>,
	#[serde(default)]
	pub environments: HashMap<String, EnvironmentOverride>,
}

impl DeployConfig {
	/// Load configuration from a file path.
	pub fn from_file(path: &Path) -> DeployResult<Self> {
		if !path.exists() {
			return Err(DeployError::ConfigNotFound {
				path: path.to_path_buf(),
			});
		}
		let content = std::fs::read_to_string(path)?;
		let config: DeployConfig = toml::from_str(&content)?;
		Ok(config)
	}

	/// Load configuration from `deploy.toml` in the project root, or return defaults.
	pub fn load_or_default(project_root: &Path) -> DeployResult<Self> {
		let config_path = project_root.join("deploy.toml");
		if config_path.exists() {
			Self::from_file(&config_path)
		} else {
			Ok(Self::default())
		}
	}
}

/// Project identification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
	#[serde(default)]
	pub name: String,
	pub region: Option<String>,
}

/// Cloud provider selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
	#[serde(rename = "type", default)]
	pub provider_type: ProviderType,
}

/// Supported cloud providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
	#[default]
	Docker,
	#[serde(rename = "fly")]
	FlyIo,
	Aws,
	Gcp,
}

/// Application runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
	#[serde(default = "default_port")]
	pub port: u16,
	#[serde(default = "default_health_check")]
	pub health_check: String,
	#[serde(default = "default_instances")]
	pub instances: u32,
	#[serde(default = "default_cpu")]
	pub cpu: u32,
	#[serde(default = "default_memory")]
	pub memory: u32,
	pub env_file: Option<String>,
}

impl Default for AppConfig {
	fn default() -> Self {
		Self {
			port: default_port(),
			health_check: default_health_check(),
			instances: default_instances(),
			cpu: default_cpu(),
			memory: default_memory(),
			env_file: None,
		}
	}
}

fn default_port() -> u16 {
	8000
}

fn default_health_check() -> String {
	"/health/".to_string()
}

fn default_instances() -> u32 {
	1
}

fn default_cpu() -> u32 {
	256
}

fn default_memory() -> u32 {
	512
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
	#[serde(default)]
	pub engine: DatabaseEngine,
	pub version: Option<String>,
	#[serde(default)]
	pub instance_size: InstanceSize,
	#[serde(default = "default_storage_gb")]
	pub storage_gb: u32,
	#[serde(default = "default_backup_retention_days")]
	pub backup_retention_days: u32,
	#[serde(default)]
	pub high_availability: bool,
}

/// Supported relational database engines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseEngine {
	#[default]
	#[serde(rename = "postgresql")]
	PostgreSql,
	#[serde(rename = "mysql")]
	MySql,
}

/// Instance size tiers for managed services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstanceSize {
	Micro,
	#[default]
	Small,
	Medium,
	Large,
	Xlarge,
}

fn default_storage_gb() -> u32 {
	10
}

fn default_backup_retention_days() -> u32 {
	7
}

/// NoSQL database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoSqlConfig {
	#[serde(default)]
	pub enabled: bool,
	#[serde(default)]
	pub engine: NoSqlEngine,
	pub version: Option<String>,
	#[serde(default)]
	pub instance_size: InstanceSize,
	#[serde(default = "default_storage_gb")]
	pub storage_gb: u32,
	#[serde(default = "default_backup_retention_days")]
	pub backup_retention_days: u32,
}

/// Supported NoSQL engines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NoSqlEngine {
	#[default]
	#[serde(rename = "mongodb")]
	MongoDb,
	#[serde(rename = "dynamodb")]
	DynamoDb,
	Firestore,
}

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
	#[serde(default = "default_cache_engine")]
	pub engine: String,
	pub version: Option<String>,
	#[serde(default)]
	pub instance_size: InstanceSize,
}

fn default_cache_engine() -> String {
	"redis".to_string()
}

/// Frontend build and deployment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendConfig {
	#[serde(default)]
	pub enabled: bool,
	#[serde(rename = "type")]
	pub frontend_type: Option<String>,
	pub build_tool: Option<String>,
	pub dist_dir: Option<String>,
	pub fallback: Option<String>,
	#[serde(default)]
	pub cdn: bool,
}

/// Static file hosting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticConfig {
	pub storage: Option<String>,
	#[serde(default)]
	pub cdn: bool,
	pub path: Option<String>,
}

/// Media file storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
	pub storage: Option<String>,
	pub bucket_prefix: Option<String>,
	#[serde(default)]
	pub cdn: bool,
}

/// Background task configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
	#[serde(default)]
	pub enabled: bool,
	pub backend: Option<String>,
	#[serde(default = "default_workers")]
	pub workers: u32,
	#[serde(default)]
	pub separate_container: bool,
}

fn default_workers() -> u32 {
	1
}

/// WebSocket configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketsConfig {
	#[serde(default)]
	pub enabled: bool,
	pub channel_backend: Option<String>,
	#[serde(default = "default_ws_path")]
	pub path: String,
}

fn default_ws_path() -> String {
	"/ws/".to_string()
}

/// Mail backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
	pub backend: Option<String>,
}

/// Secrets management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
	pub backend: Option<String>,
	#[serde(default)]
	pub auto_provision: bool,
}

/// Network and domain configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
	pub domain: Option<String>,
	#[serde(default = "default_true")]
	pub tls: bool,
	#[serde(default = "default_true")]
	pub force_https: bool,
	#[serde(default)]
	pub websocket: bool,
	#[serde(default)]
	pub grpc: bool,
}

impl Default for NetworkConfig {
	fn default() -> Self {
		Self {
			domain: None,
			tls: true,
			force_https: true,
			websocket: false,
			grpc: false,
		}
	}
}

fn default_true() -> bool {
	true
}

/// Terraform backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformConfig {
	#[serde(default = "default_terraform_version")]
	pub version: String,
	#[serde(default = "default_terraform_backend")]
	pub backend: String,
	#[serde(default = "default_true")]
	pub lock: bool,
}

impl Default for TerraformConfig {
	fn default() -> Self {
		Self {
			version: default_terraform_version(),
			backend: default_terraform_backend(),
			lock: true,
		}
	}
}

fn default_terraform_version() -> String {
	"1.11".to_string()
}

fn default_terraform_backend() -> String {
	"local".to_string()
}

/// Preview environment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
	#[serde(default)]
	pub enabled: bool,
	#[serde(default)]
	pub auto_deploy: bool,
	#[serde(default)]
	pub branch_subdomains: bool,
	#[serde(default = "default_ttl_hours")]
	pub ttl_hours: u32,
	#[serde(default)]
	pub shared_database: bool,
	#[serde(default)]
	pub seed_data: bool,
}

fn default_ttl_hours() -> u32 {
	72
}

/// Git integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
	#[serde(default)]
	pub auto_deploy: bool,
	#[serde(default = "default_production_branch")]
	pub production_branch: String,
	#[serde(default)]
	pub preview_branches: Vec<String>,
	#[serde(default)]
	pub ignored_branches: Vec<String>,
}

fn default_production_branch() -> String {
	"main".to_string()
}

/// Per-environment configuration overrides.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentOverride {
	pub domain: Option<String>,
	pub app_instances: Option<u32>,
	pub app_cpu: Option<u32>,
	pub app_memory: Option<u32>,
	pub db_instance_size: Option<InstanceSize>,
	pub db_storage_gb: Option<u32>,
	pub db_high_availability: Option<bool>,
	pub cache_instance_size: Option<InstanceSize>,
	pub env_file: Option<String>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn parse_minimal_config() {
		// Arrange
		let toml_str = r#"
[project]
name = "myapp"

[provider]
type = "docker"
"#;

		// Act
		let config: DeployConfig = toml::from_str(toml_str).unwrap();

		// Assert
		assert_eq!(config.project.name, "myapp");
		assert_eq!(config.provider.provider_type, ProviderType::Docker);
	}

	#[rstest]
	fn parse_full_config() {
		// Arrange
		let toml_str = r#"
[project]
name = "myapp"
region = "ap-northeast-1"

[provider]
type = "aws"

[app]
port = 8000
health_check = "/health/"
instances = 2

[database]
engine = "postgresql"
version = "16"
instance_size = "small"
storage_gb = 20

[nosql]
enabled = true
engine = "mongodb"
version = "7"
instance_size = "small"

[cache]
engine = "redis"
version = "7"
instance_size = "small"

[frontend]
enabled = true
type = "spa"
build_tool = "trunk"

[network]
domain = "myapp.example.com"
tls = true
force_https = true
"#;

		// Act
		let config: DeployConfig = toml::from_str(toml_str).unwrap();

		// Assert
		assert_eq!(config.provider.provider_type, ProviderType::Aws);
		assert_eq!(config.app.port, 8000);
		assert_eq!(config.app.instances, 2);
		let db = config.database.unwrap();
		assert_eq!(db.engine, DatabaseEngine::PostgreSql);
		assert_eq!(db.instance_size, InstanceSize::Small);
		let nosql = config.nosql.unwrap();
		assert!(nosql.enabled);
		assert_eq!(nosql.engine, NoSqlEngine::MongoDb);
		let frontend = config.frontend.unwrap();
		assert!(frontend.enabled);
		assert!(config.network.tls);
	}

	#[rstest]
	fn default_config_values() {
		// Arrange & Act
		let config = DeployConfig::default();

		// Assert
		assert_eq!(config.provider.provider_type, ProviderType::Docker);
		assert_eq!(config.app.port, 8000);
		assert_eq!(config.app.instances, 1);
		assert_eq!(config.app.cpu, 256);
		assert_eq!(config.app.memory, 512);
		assert!(config.network.tls);
		assert!(config.network.force_https);
	}

	#[rstest]
	fn load_config_from_file() {
		// Arrange
		let dir = tempfile::tempdir().unwrap();
		let config_path = dir.path().join("deploy.toml");
		std::fs::write(
			&config_path,
			r#"
[project]
name = "fileapp"

[provider]
type = "gcp"
"#,
		)
		.unwrap();

		// Act
		let config = DeployConfig::from_file(&config_path).unwrap();

		// Assert
		assert_eq!(config.project.name, "fileapp");
		assert_eq!(config.provider.provider_type, ProviderType::Gcp);
	}

	#[rstest]
	fn load_config_missing_file() {
		// Arrange
		let path = std::path::PathBuf::from("/nonexistent/deploy.toml");

		// Act
		let result = DeployConfig::from_file(&path);

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	fn parse_all_provider_types() {
		// Arrange & Act & Assert
		let docker: ProviderConfig = toml::from_str(r#"type = "docker""#).unwrap();
		assert_eq!(docker.provider_type, ProviderType::Docker);

		let aws: ProviderConfig = toml::from_str(r#"type = "aws""#).unwrap();
		assert_eq!(aws.provider_type, ProviderType::Aws);

		let gcp: ProviderConfig = toml::from_str(r#"type = "gcp""#).unwrap();
		assert_eq!(gcp.provider_type, ProviderType::Gcp);

		let fly: ProviderConfig = toml::from_str(r#"type = "fly""#).unwrap();
		assert_eq!(fly.provider_type, ProviderType::FlyIo);
	}
}
