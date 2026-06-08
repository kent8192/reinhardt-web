use reinhardt_commands::local_infra::{
	DatabaseInfraInput, DockerCall, DockerEngine, FakeDockerEngine, InfraCommand, LocalInfraConfig,
	LocalInfraSettingsSource, LocalInfraState, LocalServiceState, PortAllocator, RedisInfraInput,
	ServiceRuntimeStatus, StateStore,
};
use reinhardt_conf::settings::sources::ConfigSource;
use tempfile::TempDir;

#[test]
fn state_store_round_trips_local_infra_state() {
	let temp = TempDir::new().unwrap();
	let store = StateStore::new(temp.path());
	let state = LocalInfraState {
		project_id: "project123".to_string(),
		profile: "local".to_string(),
		services: vec![LocalServiceState {
			name: "postgres".to_string(),
			container_name: "reinhardt-project123-local-postgres".to_string(),
			image: "postgres:17-alpine".to_string(),
			host: "127.0.0.1".to_string(),
			host_port: 55432,
			container_port: 5432,
			status: ServiceRuntimeStatus::Running,
			metadata: serde_json::json!({"database": "app", "user": "postgres"}),
		}],
	};

	store.save(&state).unwrap();
	let loaded = store.load().unwrap().expect("state should exist");

	assert_eq!(loaded.project_id, "project123");
	assert_eq!(loaded.profile, "local");
	assert_eq!(loaded.services.len(), 1);
	assert_eq!(loaded.services[0].host_port, 55432);
}

#[test]
fn state_store_missing_file_returns_none() {
	let temp = TempDir::new().unwrap();
	let store = StateStore::new(temp.path());

	let loaded = store.load().unwrap();

	assert!(loaded.is_none());
}

#[test]
fn settings_overlay_maps_postgres_and_redis_state_to_settings_keys() {
	let state = LocalInfraState {
		project_id: "project123".to_string(),
		profile: "local".to_string(),
		services: vec![
			LocalServiceState {
				name: "postgres".to_string(),
				container_name: "pg".to_string(),
				image: "postgres:17-alpine".to_string(),
				host: "127.0.0.1".to_string(),
				host_port: 55432,
				container_port: 5432,
				status: ServiceRuntimeStatus::Running,
				metadata: serde_json::json!({
					"database": "app",
					"user": "postgres",
					"password": "postgres"
				}),
			},
			LocalServiceState {
				name: "redis".to_string(),
				container_name: "redis".to_string(),
				image: "redis:7-alpine".to_string(),
				host: "127.0.0.1".to_string(),
				host_port: 56379,
				container_port: 6379,
				status: ServiceRuntimeStatus::Running,
				metadata: serde_json::json!({"database": 1}),
			},
		],
	};

	let loaded = LocalInfraSettingsSource::from_state(state).load().unwrap();
	let core = loaded.get("core").unwrap();
	let cache = loaded.get("cache").unwrap();

	assert_eq!(
		core["databases"]["default"]["host"],
		serde_json::json!("127.0.0.1")
	);
	assert_eq!(
		core["databases"]["default"]["port"],
		serde_json::json!(55432)
	);
	assert_eq!(
		cache["location"],
		serde_json::json!("redis://127.0.0.1:56379/1")
	);
	assert_eq!(
		loaded.get("redis_url").unwrap(),
		&serde_json::json!("redis://127.0.0.1:56379/1")
	);
}

#[test]
fn port_allocator_uses_fallback_when_requested_port_is_occupied() {
	let allocator = PortAllocator;
	let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
	let occupied = listener.local_addr().unwrap().port();

	let selected = allocator.select_port(occupied).unwrap();

	assert_ne!(selected, occupied);
}

#[tokio::test]
async fn docker_engine_records_container_existence_checks() {
	let docker = FakeDockerEngine::new(vec![true]);

	let exists = docker.container_exists("reinhardt-test").await.unwrap();

	assert!(exists);
	let calls = docker.calls();
	assert_eq!(calls.len(), 1);
	assert_eq!(
		calls[0],
		DockerCall::ContainerExists {
			name: "reinhardt-test".to_string()
		}
	);
}

#[test]
fn local_infra_config_derives_postgres_and_redis_services() {
	let config = LocalInfraConfig::derive(
		"project123",
		"local",
		Some(DatabaseInfraInput {
			engine: "postgresql".to_string(),
			host: "localhost".to_string(),
			port: 5432,
			name: "app".to_string(),
			user: "postgres".to_string(),
			password: Some("postgres".to_string()),
		}),
		Some(RedisInfraInput {
			url: "redis://localhost:6379/1".to_string(),
		}),
	)
	.unwrap();

	assert_eq!(config.project_id, "project123");
	assert_eq!(config.profile, "local");
	assert_eq!(config.services.len(), 2);
	assert_eq!(config.services[0].name(), "postgres");
	assert_eq!(config.services[1].name(), "redis");
}

#[test]
fn local_infra_config_ignores_sqlite_database() {
	let config = LocalInfraConfig::derive(
		"project123",
		"local",
		Some(DatabaseInfraInput {
			engine: "sqlite".to_string(),
			host: "localhost".to_string(),
			port: 0,
			name: "db.sqlite3".to_string(),
			user: String::new(),
			password: None,
		}),
		None,
	)
	.unwrap();

	assert!(config.services.is_empty());
}

#[tokio::test]
async fn infra_down_removes_state_even_when_containers_are_missing() {
	let temp = TempDir::new().unwrap();
	let store = StateStore::new(temp.path());
	store
		.save(&LocalInfraState {
			project_id: "project123".to_string(),
			profile: "local".to_string(),
			services: vec![],
		})
		.unwrap();
	let docker = FakeDockerEngine::new(vec![]);

	InfraCommand::execute_with_runner(
		reinhardt_commands::local_infra::InfraSubcommand::Down { profile: None },
		temp.path(),
		docker,
	)
	.await
	.unwrap();

	assert!(store.load().unwrap().is_none());
}

#[tokio::test]
async fn infra_up_writes_state_for_started_services() {
	let temp = TempDir::new().unwrap();
	let docker = FakeDockerEngine::new(vec![]);

	let config = LocalInfraConfig::derive(
		"project123",
		"local",
		Some(DatabaseInfraInput {
			engine: "postgresql".to_string(),
			host: "localhost".to_string(),
			port: 5432,
			name: "app".to_string(),
			user: "postgres".to_string(),
			password: Some("postgres".to_string()),
		}),
		None,
	)
	.unwrap();

	InfraCommand::up_with_config(temp.path(), config, docker)
		.await
		.unwrap();

	let state = StateStore::new(temp.path()).load().unwrap().unwrap();
	assert_eq!(state.services.len(), 1);
	assert_eq!(state.services[0].name, "postgres");
}

#[test]
fn infra_run_loads_state_as_local_infra_settings_source() {
	let temp = TempDir::new().unwrap();
	let store = StateStore::new(temp.path());
	store
		.save(&LocalInfraState {
			project_id: "project123".to_string(),
			profile: "local".to_string(),
			services: vec![LocalServiceState {
				name: "redis".to_string(),
				container_name: "redis".to_string(),
				image: "redis:7-alpine".to_string(),
				host: "127.0.0.1".to_string(),
				host_port: 56379,
				container_port: 6379,
				status: ServiceRuntimeStatus::Running,
				metadata: serde_json::json!({"database": 0}),
			}],
		})
		.unwrap();

	let source = InfraCommand::settings_source_from_state(temp.path()).unwrap();
	let loaded = source.load().unwrap();

	assert_eq!(
		loaded.get("redis_url").unwrap(),
		&serde_json::json!("redis://127.0.0.1:56379/0")
	);
}
