//! ContentType management with multi-database support
//!
//! This module provides ContentType management functionality across multiple databases.
//! It maintains independent ContentType caches for each database and supports
//! cross-database queries.

#[cfg(feature = "database")]
use parking_lot::RwLock;
#[cfg(feature = "database")]
use std::collections::HashMap;
#[cfg(feature = "database")]
use std::sync::Arc;

#[cfg(feature = "database")]
use crate::contenttypes::{ContentType, ContentTypeRegistry};
#[cfg(feature = "database")]
use crate::persistence::{ContentTypePersistence, ContentTypePersistenceBackend, PersistenceError};

/// ContentType management in multi-database environments
///
/// Manages independent ContentType registries and persistence backends for each database.
///
/// ## Example
///
/// ```rust,no_run
/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut manager = MultiDbContentTypeManager::new();
///
/// // Register databases
/// manager.add_database("primary", "postgres://localhost/primary").await?;
/// manager.add_database("analytics", "postgres://localhost/analytics").await?;
///
/// // Get ContentType from a specific database
/// let ct = manager.get_or_create("primary", "auth", "User").await?;
/// assert_eq!(ct.app_label, "auth");
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "database")]
#[derive(Clone)]
pub struct MultiDbContentTypeManager {
	/// Persistence backends for each database
	databases: Arc<RwLock<HashMap<String, ContentTypePersistence>>>,
	/// ContentType registry cache for each database
	registries: Arc<RwLock<HashMap<String, Arc<ContentTypeRegistry>>>>,
	/// Default database name
	default_db: Option<String>,
}

#[cfg(feature = "database")]
impl MultiDbContentTypeManager {
	/// Create a new multi-database manager
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// let manager = MultiDbContentTypeManager::new();
	/// ```
	pub fn new() -> Self {
		Self {
			databases: Arc::new(RwLock::new(HashMap::new())),
			registries: Arc::new(RwLock::new(HashMap::new())),
			default_db: None,
		}
	}

	/// Set the default database
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// let manager = MultiDbContentTypeManager::new()
	///     .with_default_db("primary");
	///
	/// assert_eq!(manager.default_db(), Some("primary"));
	/// ```
	pub fn with_default_db(mut self, db_alias: impl Into<String>) -> Self {
		self.default_db = Some(db_alias.into());
		self
	}

	/// Get the default database name
	pub fn default_db(&self) -> Option<&str> {
		self.default_db.as_deref()
	}

	/// Add a database
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn add_database(
		&mut self,
		db_alias: impl Into<String>,
		database_url: &str,
	) -> Result<(), PersistenceError> {
		let db_alias = db_alias.into();
		let persistence = ContentTypePersistence::new(database_url).await?;

		// Create table
		persistence.create_table().await?;

		// Save persistence and registry
		self.databases.write().insert(db_alias.clone(), persistence);
		self.registries
			.write()
			.insert(db_alias.clone(), Arc::new(ContentTypeRegistry::new()));

		// Set the first database as default
		if self.default_db.is_none() {
			self.default_db = Some(db_alias);
		}

		Ok(())
	}

	/// Add a database from an existing persistence backend
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	/// use reinhardt_contenttypes::persistence::ContentTypePersistence;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let persistence = ContentTypePersistence::new("sqlite::memory:?cache=shared").await?;
	/// persistence.create_table().await?;
	///
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database_from_persistence("primary", persistence);
	/// # Ok(())
	/// # }
	/// ```
	pub fn add_database_from_persistence(
		&mut self,
		db_alias: impl Into<String>,
		persistence: ContentTypePersistence,
	) {
		let db_alias = db_alias.into();
		self.databases.write().insert(db_alias.clone(), persistence);
		self.registries
			.write()
			.insert(db_alias.clone(), Arc::new(ContentTypeRegistry::new()));

		if self.default_db.is_none() {
			self.default_db = Some(db_alias);
		}
	}

	/// Get the persistence backend of a database
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	///
	/// let persistence = manager.get_database("primary");
	/// assert!(persistence.is_some());
	/// # Ok(())
	/// # }
	/// ```
	pub fn get_database(&self, db_alias: &str) -> Option<ContentTypePersistence> {
		self.databases.read().get(db_alias).cloned()
	}

	/// Get the registry of a database
	pub fn get_registry(&self, db_alias: &str) -> Option<Arc<ContentTypeRegistry>> {
		self.registries.read().get(db_alias).cloned()
	}

	/// Get or create a ContentType
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	///
	/// let ct = manager.get_or_create("primary", "auth", "User").await?;
	/// assert_eq!(ct.app_label, "auth");
	/// assert_eq!(ct.model, "User");
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_or_create(
		&self,
		db_alias: &str,
		app_label: &str,
		model: &str,
	) -> Result<ContentType, PersistenceError> {
		// Check registry cache
		if let Some(registry) = self.get_registry(db_alias)
			&& let Some(ct) = registry.get(app_label, model)
		{
			return Ok(ct);
		}

		// Get or create from database
		let persistence = self.get_database(db_alias).ok_or_else(|| {
			PersistenceError::DatabaseError(format!("Database '{}' not found", db_alias))
		})?;

		let ct = persistence.get_or_create(app_label, model).await?;

		// Register in registry
		if let Some(registry) = self.get_registry(db_alias) {
			registry.register(ct.clone());
		}

		Ok(ct)
	}

	/// Get a ContentType from a specific database
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	///
	/// manager.get_or_create("primary", "blog", "Post").await?;
	///
	/// let ct = manager.get("primary", "blog", "Post").await?;
	/// assert!(ct.is_some());
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(
		&self,
		db_alias: &str,
		app_label: &str,
		model: &str,
	) -> Result<Option<ContentType>, PersistenceError> {
		// Check registry cache
		if let Some(registry) = self.get_registry(db_alias)
			&& let Some(ct) = registry.get(app_label, model)
		{
			return Ok(Some(ct));
		}

		// Get from database
		let persistence = self.get_database(db_alias).ok_or_else(|| {
			PersistenceError::DatabaseError(format!("Database '{}' not found", db_alias))
		})?;

		let ct = persistence.get(app_label, model).await?;

		// Register in registry
		if let Some(ct_ref) = &ct
			&& let Some(registry) = self.get_registry(db_alias)
		{
			registry.register(ct_ref.clone());
		}

		Ok(ct)
	}

	/// Get a ContentType by ID
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	///
	/// let ct = manager.get_or_create("primary", "auth", "User").await?;
	/// let id = ct.id.unwrap();
	///
	/// let retrieved = manager.get_by_id("primary", id).await?;
	/// assert!(retrieved.is_some());
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_by_id(
		&self,
		db_alias: &str,
		id: i64,
	) -> Result<Option<ContentType>, PersistenceError> {
		// Check registry cache
		if let Some(registry) = self.get_registry(db_alias)
			&& let Some(ct) = registry.get_by_id(id)
		{
			return Ok(Some(ct));
		}

		// Fetch from database
		let persistence = self.get_database(db_alias).ok_or_else(|| {
			PersistenceError::DatabaseError(format!("Database '{}' not found", db_alias))
		})?;

		let ct = persistence.get_by_id(id).await?;

		// Register in registry
		if let Some(ct_ref) = &ct
			&& let Some(registry) = self.get_registry(db_alias)
		{
			registry.register(ct_ref.clone());
		}

		Ok(ct)
	}

	/// Search for ContentType across all databases
	///
	/// Performs ContentType search spanning multiple databases.
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	/// manager.add_database("analytics", "sqlite::memory:?cache=shared").await?;
	///
	/// manager.get_or_create("primary", "auth", "User").await?;
	/// manager.get_or_create("analytics", "logs", "AccessLog").await?;
	///
	/// let results = manager.search_all_databases("auth", "User").await?;
	/// assert!(!results.is_empty());
	/// # Ok(())
	/// # }
	/// ```
	pub async fn search_all_databases(
		&self,
		app_label: &str,
		model: &str,
	) -> Result<Vec<(String, ContentType)>, PersistenceError> {
		let db_aliases: Vec<String> = self.databases.read().keys().cloned().collect();
		let mut results = Vec::new();

		for db_alias in db_aliases {
			if let Ok(Some(ct)) = self.get(&db_alias, app_label, model).await {
				results.push((db_alias, ct));
			}
		}

		Ok(results)
	}

	/// Get all registered database names
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	/// manager.add_database("replica", "sqlite::memory:?cache=shared").await?;
	///
	/// let databases = manager.list_databases();
	/// assert_eq!(databases.len(), 2);
	/// # Ok(())
	/// # }
	/// ```
	pub fn list_databases(&self) -> Vec<String> {
		self.databases.read().keys().cloned().collect()
	}

	/// Load all ContentTypes from a specific database
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_contenttypes::multi_db::MultiDbContentTypeManager;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut manager = MultiDbContentTypeManager::new();
	/// manager.add_database("primary", "sqlite::memory:?cache=shared").await?;
	///
	/// let content_types = manager.load_all("primary").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn load_all(&self, db_alias: &str) -> Result<Vec<ContentType>, PersistenceError> {
		let persistence = self.get_database(db_alias).ok_or_else(|| {
			PersistenceError::DatabaseError(format!("Database '{}' not found", db_alias))
		})?;

		let content_types = persistence.load_all().await?;

		// Register all in registry
		if let Some(registry) = self.get_registry(db_alias) {
			for ct in &content_types {
				registry.register(ct.clone());
			}
		}

		Ok(content_types)
	}
}

#[cfg(feature = "database")]
impl Default for MultiDbContentTypeManager {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(all(test, feature = "database"))]
mod tests {
	use super::*;
	use std::sync::Once;

	static INIT_DRIVERS: Once = Once::new();

	fn init_drivers() {
		INIT_DRIVERS.call_once(|| {
			sqlx::any::install_default_drivers();
		});
	}

	#[tokio::test]
	async fn test_multi_db_manager_creation() {
		init_drivers();
		let manager = MultiDbContentTypeManager::new();
		assert!(manager.default_db().is_none());
		assert_eq!(manager.list_databases().len(), 0);
	}

	#[tokio::test]
	async fn test_multi_db_add_database() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add database");

		assert_eq!(manager.default_db(), Some("primary"));
		assert_eq!(manager.list_databases().len(), 1);
	}

	#[tokio::test]
	async fn test_multi_db_multiple_databases() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add primary");
		manager
			.add_database("analytics", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add analytics");

		let databases = manager.list_databases();
		assert_eq!(databases.len(), 2);
		assert!(databases.contains(&"primary".to_string()));
		assert!(databases.contains(&"analytics".to_string()));
	}

	#[tokio::test]
	async fn test_multi_db_get_or_create() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add database");

		let ct = manager
			.get_or_create("primary", "auth", "User")
			.await
			.expect("Failed to get_or_create");

		assert_eq!(ct.app_label, "auth");
		assert_eq!(ct.model, "User");
		assert!(ct.id.is_some());
	}

	#[tokio::test]
	async fn test_multi_db_get() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add database");

		// Create
		manager
			.get_or_create("primary", "blog", "Post")
			.await
			.expect("Failed to create");

		// Retrieve
		let ct = manager
			.get("primary", "blog", "Post")
			.await
			.expect("Failed to get");

		assert!(ct.is_some());
		assert_eq!(ct.unwrap().model, "Post");
	}

	#[tokio::test]
	async fn test_multi_db_get_by_id() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add database");

		let ct = manager
			.get_or_create("primary", "shop", "Product")
			.await
			.expect("Failed to create");
		let id = ct.id.unwrap();

		let retrieved = manager
			.get_by_id("primary", id)
			.await
			.expect("Failed to get by id");

		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().model, "Product");
	}

	#[tokio::test]
	async fn test_multi_db_search_all_databases() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("db1", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add db1");
		manager
			.add_database("db2", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add db2");

		// Create same ContentType in both databases
		manager
			.get_or_create("db1", "common", "Model")
			.await
			.expect("Failed to create in db1");
		manager
			.get_or_create("db2", "common", "Model")
			.await
			.expect("Failed to create in db2");

		let results = manager
			.search_all_databases("common", "Model")
			.await
			.expect("Failed to search");

		assert_eq!(results.len(), 2);
	}

	#[tokio::test]
	async fn test_multi_db_load_all() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add database");

		// Create multiple ContentTypes
		manager
			.get_or_create("primary", "app1", "Model1")
			.await
			.expect("Failed to create");
		manager
			.get_or_create("primary", "app2", "Model2")
			.await
			.expect("Failed to create");

		let all = manager
			.load_all("primary")
			.await
			.expect("Failed to load all");

		assert_eq!(all.len(), 2);
	}

	#[tokio::test]
	async fn test_multi_db_registry_caching() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("primary", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add database");

		// First call (fetch from database)
		let ct1 = manager
			.get_or_create("primary", "test", "CacheTest")
			.await
			.expect("Failed to create");

		// Second call (fetch from cache)
		let ct2 = manager
			.get_or_create("primary", "test", "CacheTest")
			.await
			.expect("Failed to get");

		assert_eq!(ct1.id, ct2.id);
	}

	#[tokio::test]
	async fn test_multi_db_default_db() {
		init_drivers();
		let manager = MultiDbContentTypeManager::new().with_default_db("main");

		assert_eq!(manager.default_db(), Some("main"));
	}

	#[tokio::test]
	async fn test_multi_db_nonexistent_database() {
		init_drivers();
		let manager = MultiDbContentTypeManager::new();

		let result = manager.get("nonexistent", "app", "Model").await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_multi_db_isolated_registries() {
		init_drivers();
		let mut manager = MultiDbContentTypeManager::new();
		manager
			.add_database("db1", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add db1");
		manager
			.add_database("db2", "sqlite::memory:?mode=rwc&cache=shared")
			.await
			.expect("Failed to add db2");

		// Create ContentType in db1 only
		manager
			.get_or_create("db1", "isolated", "Model")
			.await
			.expect("Failed to create");

		// Does not exist in db2's registry
		let registry2 = manager.get_registry("db2").unwrap();
		assert!(registry2.get("isolated", "Model").is_none());
	}
}
