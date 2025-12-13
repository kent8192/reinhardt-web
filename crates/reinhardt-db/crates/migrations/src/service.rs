//! Migration service layer
//!
//! Provides high-level business logic for migration operations,
//! orchestrating Source and Repository patterns.

use crate::{Migration, MigrationError, MigrationRepository, MigrationSource, Result};
use std::sync::Arc;

/// Migration service that orchestrates Source and Repository
///
/// This service provides high-level operations for:
/// - Loading migrations from various sources
/// - Persisting migrations to storage
/// - Building migration dependency graphs
/// - Detecting migration changes
pub struct MigrationService {
	/// Source for loading migrations
	source: Arc<dyn MigrationSource>,
	/// Repository for persisting migrations
	repository: Arc<tokio::sync::Mutex<dyn MigrationRepository>>,
}

impl MigrationService {
	/// Create a new MigrationService
	///
	/// # Arguments
	///
	/// * `source` - Migration source for loading
	/// * `repository` - Migration repository for persistence
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_migrations::{MigrationService, MigrationRepository, RegistrySource, FilesystemRepository};
	/// use std::sync::Arc;
	/// let source = Arc::new(RegistrySource::new());
	/// let repository = Arc::new(tokio::sync::Mutex::new(
	///     FilesystemRepository::new("./migrations")
	/// ));
	///
	/// let service = MigrationService::new(source, repository);
	/// ```
	pub fn new(
		source: Arc<dyn MigrationSource>,
		repository: Arc<tokio::sync::Mutex<dyn MigrationRepository>>,
	) -> Self {
		Self { source, repository }
	}

	/// Load all migrations from source
	///
	/// # Returns
	///
	/// Vector of all available migrations
	pub async fn load_all(&self) -> Result<Vec<Migration>> {
		self.source.all_migrations().await
	}

	/// Load migrations for a specific app
	///
	/// # Arguments
	///
	/// * `app_label` - App label to filter by
	///
	/// # Returns
	///
	/// Vector of migrations for the specified app
	pub async fn load_for_app(&self, app_label: &str) -> Result<Vec<Migration>> {
		self.source.migrations_for_app(app_label).await
	}

	/// Load a specific migration
	///
	/// # Arguments
	///
	/// * `app_label` - App label
	/// * `name` - Migration name
	///
	/// # Returns
	///
	/// The requested migration
	pub async fn load_migration(&self, app_label: &str, name: &str) -> Result<Migration> {
		self.source.get_migration(app_label, name).await
	}

	/// Save a migration to repository
	///
	/// # Arguments
	///
	/// * `migration` - Migration to save
	pub async fn save_migration(&self, migration: &Migration) -> Result<()> {
		let mut repo = self.repository.lock().await;
		repo.save(migration).await
	}

	/// Check if a migration exists in repository
	///
	/// # Arguments
	///
	/// * `app_label` - App label
	/// * `name` - Migration name
	///
	/// # Returns
	///
	/// `true` if the migration exists, `false` otherwise
	pub async fn migration_exists(&self, app_label: &str, name: &str) -> Result<bool> {
		let repo = self.repository.lock().await;
		repo.exists(app_label, name).await
	}

	/// List all migrations in repository for an app
	///
	/// # Arguments
	///
	/// * `app_label` - App label
	///
	/// # Returns
	///
	/// Vector of migrations in the repository
	pub async fn list_saved_migrations(&self, app_label: &str) -> Result<Vec<Migration>> {
		let repo = self.repository.lock().await;
		repo.list(app_label).await
	}

	/// Delete a migration from repository
	///
	/// # Arguments
	///
	/// * `app_label` - App label
	/// * `name` - Migration name
	pub async fn delete_migration(&self, app_label: &str, name: &str) -> Result<()> {
		let mut repo = self.repository.lock().await;
		repo.delete(app_label, name).await
	}

	/// Build migration dependency graph
	///
	/// Returns migrations sorted by dependencies (leaf nodes first)
	pub async fn build_dependency_graph(&self) -> Result<Vec<Migration>> {
		let migrations = self.load_all().await?;

		// Build adjacency list
		let mut graph: std::collections::HashMap<(String, String), Vec<(String, String)>> =
			std::collections::HashMap::new();
		let mut in_degree: std::collections::HashMap<(String, String), usize> =
			std::collections::HashMap::new();

		// Initialize graph
		for migration in &migrations {
			let key = (migration.app_label.to_string(), migration.name.to_string());
			graph.insert(key.clone(), Vec::new());
			in_degree.insert(key, 0);
		}

		// Build edges
		for migration in &migrations {
			let key = (migration.app_label.to_string(), migration.name.to_string());
			for dep in &migration.dependencies {
				let dep_key = (dep.0.to_string(), dep.1.to_string());
				if let Some(deps) = graph.get_mut(&dep_key) {
					deps.push(key.clone());
				}
				*in_degree.get_mut(&key).unwrap() += 1;
			}
		}

		// Topological sort (Kahn's algorithm)
		let mut queue: Vec<(String, String)> = in_degree
			.iter()
			.filter(|&(_, &degree)| degree == 0)
			.map(|(k, _)| k.clone())
			.collect();

		let mut sorted = Vec::new();

		while let Some(current) = queue.pop() {
			// Find the migration
			if let Some(migration) = migrations
				.iter()
				.find(|m| m.app_label == current.0 && m.name == current.1)
			{
				sorted.push(migration.clone());
			}

			// Update in-degrees
			if let Some(neighbors) = graph.get(&current) {
				for neighbor in neighbors {
					if let Some(degree) = in_degree.get_mut(neighbor) {
						*degree -= 1;
						if *degree == 0 {
							queue.push(neighbor.clone());
						}
					}
				}
			}
		}

		// Check for cycles
		if sorted.len() != migrations.len() {
			return Err(MigrationError::CircularDependency {
				cycle: "Circular dependency detected in migrations".to_string(),
			});
		}

		Ok(sorted)
	}

	/// Detect new migrations (in source but not in repository)
	///
	/// # Arguments
	///
	/// * `app_label` - App label to check
	///
	/// # Returns
	///
	/// Vector of new migrations that haven't been saved yet
	pub async fn detect_new_migrations(&self, app_label: &str) -> Result<Vec<Migration>> {
		let source_migrations = self.load_for_app(app_label).await?;
		let saved_migrations = self.list_saved_migrations(app_label).await?;

		let saved_names: std::collections::HashSet<_> =
			saved_migrations.iter().map(|m| &m.name).collect();

		Ok(source_migrations
			.into_iter()
			.filter(|m| !saved_names.contains(&m.name))
			.collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::MigrationSource;
	use async_trait::async_trait;
	use std::collections::HashMap;
	use tokio::sync::Mutex;

	/// Test source implementation
	struct TestSource {
		migrations: Vec<Migration>,
	}

	#[async_trait]
	impl MigrationSource for TestSource {
		async fn all_migrations(&self) -> Result<Vec<Migration>> {
			Ok(self.migrations.clone())
		}
	}

	/// Test repository implementation
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

		async fn exists(&self, app_label: &str, name: &str) -> Result<bool> {
			let key = (app_label.to_string(), name.to_string());
			Ok(self.migrations.contains_key(&key))
		}

		async fn delete(&mut self, app_label: &str, name: &str) -> Result<()> {
			let key = (app_label.to_string(), name.to_string());
			self.migrations
				.remove(&key)
				.ok_or_else(|| MigrationError::NotFound(format!("{}.{}", app_label, name)))?;
			Ok(())
		}
	}

	fn create_test_migration(app_label: &'static str, name: &'static str) -> Migration {
		Migration {
			app_label,
			name,
			operations: vec![],
			dependencies: vec![],
			atomic: true,
			initial: None,
			replaces: vec![],
		}
	}

	#[tokio::test]
	async fn test_migration_service_load_all() {
		let source = Arc::new(TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				create_test_migration("users", "0001_initial"),
			],
		});
		let repository = Arc::new(Mutex::new(TestRepository::new()));
		let service = MigrationService::new(source, repository);

		let migrations = service.load_all().await.unwrap();
		assert_eq!(migrations.len(), 2);
	}

	#[tokio::test]
	async fn test_migration_service_load_for_app() {
		let source = Arc::new(TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				create_test_migration("polls", "0002_add_field"),
				create_test_migration("users", "0001_initial"),
			],
		});
		let repository = Arc::new(Mutex::new(TestRepository::new()));
		let service = MigrationService::new(source, repository);

		let polls_migrations = service.load_for_app("polls").await.unwrap();
		assert_eq!(polls_migrations.len(), 2);
	}

	#[tokio::test]
	async fn test_migration_service_save_and_load() {
		let source = Arc::new(TestSource {
			migrations: vec![create_test_migration("polls", "0001_initial")],
		});
		let repository = Arc::new(Mutex::new(TestRepository::new()));
		let service = MigrationService::new(source, repository);

		let migration = create_test_migration("polls", "0001_initial");
		service.save_migration(&migration).await.unwrap();

		assert!(
			service
				.migration_exists("polls", "0001_initial")
				.await
				.unwrap()
		);
	}

	#[tokio::test]
	async fn test_migration_service_dependency_graph() {
		let source = Arc::new(TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				Migration {
					app_label: "polls",
					name: "0002_add_field",
					operations: vec![],
					dependencies: vec![("polls", "0001_initial")],
					atomic: true,
					initial: None,
					replaces: vec![],
				},
			],
		});
		let repository = Arc::new(Mutex::new(TestRepository::new()));
		let service = MigrationService::new(source, repository);

		let sorted = service.build_dependency_graph().await.unwrap();
		assert_eq!(sorted.len(), 2);
		// 0001_initial should come before 0002_add_field
		assert_eq!(sorted[0].name, "0001_initial");
		assert_eq!(sorted[1].name, "0002_add_field");
	}

	#[tokio::test]
	async fn test_migration_service_detect_new_migrations() {
		let source = Arc::new(TestSource {
			migrations: vec![
				create_test_migration("polls", "0001_initial"),
				create_test_migration("polls", "0002_add_field"),
			],
		});
		let repository = Arc::new(Mutex::new(TestRepository::new()));
		let service = MigrationService::new(source.clone(), repository);

		// Save only 0001_initial
		service
			.save_migration(&create_test_migration("polls", "0001_initial"))
			.await
			.unwrap();

		// Detect new migrations
		let new_migrations = service.detect_new_migrations("polls").await.unwrap();
		assert_eq!(new_migrations.len(), 1);
		assert_eq!(new_migrations[0].name, "0002_add_field");
	}

	#[tokio::test]
	async fn test_migration_service_delete() {
		let source = Arc::new(TestSource {
			migrations: vec![create_test_migration("polls", "0001_initial")],
		});
		let repository = Arc::new(Mutex::new(TestRepository::new()));
		let service = MigrationService::new(source, repository);

		// Save a migration
		let migration = create_test_migration("polls", "0001_initial");
		service.save_migration(&migration).await.unwrap();

		// Verify it exists
		assert!(
			service
				.migration_exists("polls", "0001_initial")
				.await
				.unwrap()
		);

		// Delete it
		service
			.delete_migration("polls", "0001_initial")
			.await
			.unwrap();

		// Verify it's gone
		assert!(
			!service
				.migration_exists("polls", "0001_initial")
				.await
				.unwrap()
		);
	}
}
