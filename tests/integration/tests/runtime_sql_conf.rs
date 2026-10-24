//! Cross-crate regression coverage for generated native arguments in settings stores.
#![cfg(feature = "dynamic-database")]

use chrono::Duration;
use reinhardt_conf::settings::audit::backends::DatabaseAuditBackend;
use reinhardt_conf::settings::audit::{
	AuditBackend, AuditEvent, ChangeRecord, EventFilter, EventType,
};
use reinhardt_conf::settings::backends::DatabaseBackend;
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;
use testcontainers::runners::AsyncRunner;

async fn exercise_stores(url: &str) {
	// Arrange: the settings schema and audit schema use their public initialization paths.
	sqlx::any::install_default_drivers();
	let settings = DatabaseBackend::new(url).await.unwrap();
	settings.create_table().await.unwrap();
	let audit = DatabaseAuditBackend::new(url).await.unwrap();
	let key = "quoted' ? $1 config";
	let value = json!({"message": "quoted' \"text\" ? $2", "nullable": null});

	// Act / Assert: quoted input and omitted NULL binds survive every settings operation.
	for ttl in [None, Some(3600)] {
		settings.set(key, &value, ttl).await.unwrap();
		assert_eq!(settings.get(key).await.unwrap(), Some(value.clone()));
		assert!(settings.exists(key).await.unwrap());
		assert_eq!(settings.keys().await.unwrap(), vec![key]);
	}
	settings
		.set("expired' ? $3", &value, Some(0))
		.await
		.unwrap();
	assert_eq!(settings.cleanup_expired().await.unwrap(), 1);
	assert_eq!(settings.keys().await.unwrap(), vec![key]);
	settings.delete(key).await.unwrap();
	assert!(!settings.exists(key).await.unwrap());
	assert_eq!(settings.get(key).await.unwrap(), None);

	// Act: audit filtering binds the event type, user, and temporal bounds in AST order.
	let user = "admin' ? $4";
	let changes = HashMap::from([(
		key.to_owned(),
		ChangeRecord {
			old_value: None,
			new_value: Some(value.clone()),
		},
	)]);
	let event = AuditEvent::new(EventType::ConfigUpdate, Some(user.to_owned()), changes);
	let timestamp = event.timestamp;
	audit.log_event(event).await.unwrap();
	let events = audit
		.get_events(Some(EventFilter {
			event_type: Some(EventType::ConfigUpdate),
			user: Some(user.to_owned()),
			start_time: Some(timestamp - Duration::seconds(1)),
			end_time: Some(timestamp + Duration::seconds(1)),
		}))
		.await
		.unwrap();
	// Assert
	assert_eq!(events.len(), 1);
	assert_eq!(events[0].timestamp, timestamp);
	assert_eq!(events[0].user.as_deref(), Some(user));
	assert_eq!(events[0].changes[key].old_value, None);
	assert_eq!(events[0].changes[key].new_value, Some(value));
}

#[rstest]
#[tokio::test]
async fn postgres_settings_and_audit_keep_generated_bindings() {
	// Each test owns its container; early returns and panics retain RAII cleanup.
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	exercise_stores(&url).await;
}

#[rstest]
#[tokio::test]
async fn mysql_settings_and_audit_keep_generated_bindings() {
	let container = testcontainers_modules::mysql::Mysql::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"mysql://root@{}:{}/test",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(3306).await.unwrap()
	);
	exercise_stores(&url).await;
}
