//! Migration repository abstraction
//!
//! This module defines the `MigrationRepository` trait, which abstracts how migrations are stored.

pub mod filesystem;

use crate::{Migration, MigrationError, Result};
use async_trait::async_trait;

/// Trait for persisting migrations to various storage backends
///
/// Implementations:
/// - `RustFileRepository`: Writes .rs files with metadata comments
/// - `InMemoryRepository`: In-memory storage for testing
#[async_trait]
pub trait MigrationRepository: Send + Sync {
	/// Saves a migration to the repository
	async fn save(&mut self, migration: &Migration) -> Result<()>;

	/// Retrieves a migration by app and name
	async fn get(&self, app_label: &str, name: &str) -> Result<Migration>;

	/// Lists all migrations for an app
	async fn list(&self, app_label: &str) -> Result<Vec<Migration>>;

	/// Checks if a migration exists
	async fn exists(&self, app_label: &str, name: &str) -> Result<bool> {
		self.get(app_label, name).await.map(|_| true).or(Ok(false))
	}

	/// Deletes a migration (optional, not all repositories support this)
	async fn delete(&mut self, _app_label: &str, _name: &str) -> Result<()> {
		Err(MigrationError::InvalidMigration(
			"Delete operation not supported by this repository".to_string(),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	/// Test helper to create a migration
	fn create_test_migration(app_label: &'static str, name: &'static str) -> Migration {
		Migration {
			app_label,
			name,
			operations: vec![],
			dependencies: vec![],
			atomic: true,
			replaces: vec![],
		}
	}

	/// Test MigrationRepository implementation for unit tests
	struct TestRepository {
		migrations: HashMap<(String, String), Migration>,
	}

	impl TestRepository {
		fn new() -> Self {
			Self {
				migrations: HashMap::new(),
			}
		}
	}

	#[async_trait]
	impl MigrationRepository for TestRepository {
		async fn save(&mut self, migration: &Migration) -> Result<()> {
			let key = (migration.app_label.to_string(), migration.name.to_string());
			self.migrations.insert(key, migration.clone());
			Ok(())
		}

		async fn get(&self, app_label: &str, name: &str) -> Result<Migration> {
			let key = (app_label.to_string(), name.to_string());
			self.migrations
				.get(&key)
				.cloned()
				.ok_or_else(|| MigrationError::NotFound(format!("{}.{}", app_label, name)))
		}

		async fn list(&self, app_label: &str) -> Result<Vec<Migration>> {
			Ok(self
				.migrations
				.values()
				.filter(|m| m.app_label == app_label)
				.cloned()
				.collect())
		}

		async fn delete(&mut self, app_label: &str, name: &str) -> Result<()> {
			let key = (app_label.to_string(), name.to_string());
			self.migrations
				.remove(&key)
				.ok_or_else(|| MigrationError::NotFound(format!("{}.{}", app_label, name)))?;
			Ok(())
		}
	}

	#[tokio::test]
	async fn test_save_and_get() {
		let mut repo = TestRepository::new();
		let migration = create_test_migration("polls", "0001_initial");

		repo.save(&migration).await.unwrap();

		let retrieved = repo.get("polls", "0001_initial").await.unwrap();
		assert_eq!(retrieved.app_label, "polls");
		assert_eq!(retrieved.name, "0001_initial");
	}

	#[tokio::test]
	async fn test_list() {
		let mut repo = TestRepository::new();
		repo.save(&create_test_migration("polls", "0001_initial"))
			.await
			.unwrap();
		repo.save(&create_test_migration("polls", "0002_add_field"))
			.await
			.unwrap();
		repo.save(&create_test_migration("users", "0001_initial"))
			.await
			.unwrap();

		let polls_migrations = repo.list("polls").await.unwrap();
		assert_eq!(polls_migrations.len(), 2);
		assert!(polls_migrations.iter().all(|m| m.app_label == "polls"));

		let users_migrations = repo.list("users").await.unwrap();
		assert_eq!(users_migrations.len(), 1);
	}

	#[tokio::test]
	async fn test_exists() {
		let mut repo = TestRepository::new();
		repo.save(&create_test_migration("polls", "0001_initial"))
			.await
			.unwrap();

		assert!(repo.exists("polls", "0001_initial").await.unwrap());
		assert!(!repo.exists("polls", "0002_nonexistent").await.unwrap());
	}

	#[tokio::test]
	async fn test_delete() {
		let mut repo = TestRepository::new();
		repo.save(&create_test_migration("polls", "0001_initial"))
			.await
			.unwrap();

		assert!(repo.exists("polls", "0001_initial").await.unwrap());

		repo.delete("polls", "0001_initial").await.unwrap();

		assert!(!repo.exists("polls", "0001_initial").await.unwrap());
	}

	#[tokio::test]
	async fn test_get_not_found() {
		let repo = TestRepository::new();
		let result = repo.get("polls", "0001_nonexistent").await;
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), MigrationError::NotFound(_)));
	}
}
