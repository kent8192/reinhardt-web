//! Migration source abstraction
//!
//! This module defines the `MigrationSource` trait, which abstracts where migrations come from.
//! Multiple sources can be combined using the Composite pattern.

pub mod composite;
pub mod filesystem;
pub mod registry;

use crate::{Migration, MigrationError, Result};
use async_trait::async_trait;

/// Trait for loading migrations from various sources
///
/// Implementations:
/// - `RegistrySource`: Loads from compile-time registered migrations (linkme)
/// - `FilesystemSource`: Loads from .rs files on disk
/// - `CompositeSource`: Combines multiple sources
/// - `TestMigrationSource`: In-memory source for testing
#[async_trait]
pub trait MigrationSource: Send + Sync {
	/// Returns all migrations from this source
	async fn all_migrations(&self) -> Result<Vec<Migration>>;

	/// Returns migrations for a specific app
	async fn migrations_for_app(&self, app_label: &str) -> Result<Vec<Migration>> {
		let all = self.all_migrations().await?;
		Ok(all
			.into_iter()
			.filter(|m| m.app_label == app_label)
			.collect())
	}

	/// Returns a specific migration by app and name
	async fn get_migration(&self, app_label: &str, name: &str) -> Result<Migration> {
		let migrations = self.migrations_for_app(app_label).await?;
		migrations
			.into_iter()
			.find(|m| m.name == name)
			.ok_or_else(|| MigrationError::NotFound(format!("{}.{}", app_label, name)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Test helper to create a migration
	fn create_test_migration(app_label: &'static str, name: &'static str) -> Migration {
		Migration {
			app_label,
			name,
			operations: vec![],
			dependencies: vec![],
			atomic: true,
			initial: None,
			replaces: vec![],
			state_only: false,
			database_only: false,
		}
	}

	/// Test MigrationSource implementation for unit tests
	struct TestSource {
		migrations: Vec<Migration>,
	}

	#[async_trait]
	impl MigrationSource for TestSource {
		async fn all_migrations(&self) -> Result<Vec<Migration>> {
			Ok(self.migrations.clone())
		}
	}

	#[tokio::test]
	async fn test_all_migrations() {
		let source = TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				create_test_migration("polls", "0002_add_field"),
			],
		};

		let all = source.all_migrations().await.unwrap();
		assert_eq!(all.len(), 2);
		assert_eq!(all[0].app_label, "polls");
		assert_eq!(all[0].name, "0001_initial");
	}

	#[tokio::test]
	async fn test_migrations_for_app() {
		let source = TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				create_test_migration("users", "0001_initial"),
				create_test_migration("polls", "0002_add_field"),
			],
		};

		let polls_migrations = source.migrations_for_app("polls").await.unwrap();
		assert_eq!(polls_migrations.len(), 2);
		assert!(polls_migrations.iter().all(|m| m.app_label == "polls"));

		let users_migrations = source.migrations_for_app("users").await.unwrap();
		assert_eq!(users_migrations.len(), 1);
		assert_eq!(users_migrations[0].name, "0001_initial");
	}

	#[tokio::test]
	async fn test_get_migration() {
		let source = TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				create_test_migration("polls", "0002_add_field"),
			],
		};

		let migration = source.get_migration("polls", "0001_initial").await.unwrap();
		assert_eq!(migration.app_label, "polls");
		assert_eq!(migration.name, "0001_initial");
	}

	#[tokio::test]
	async fn test_get_migration_not_found() {
		let source = TestSource {
			migrations: vec![create_test_migration("polls", "0001_initial")],
		};

		let result = source.get_migration("polls", "0002_nonexistent").await;
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), MigrationError::NotFound(_)));
	}
}
