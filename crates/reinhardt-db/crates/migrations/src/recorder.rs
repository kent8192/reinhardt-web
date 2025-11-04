//! Migration recorder

use backends::{DatabaseConnection, QueryValue};
use chrono::{DateTime, Utc};

/// Migration record
#[derive(Debug, Clone)]
pub struct MigrationRecord {
	pub app: String,
	pub name: String,
	pub applied: DateTime<Utc>,
}

/// Migration recorder (in-memory only, for backward compatibility)
pub struct MigrationRecorder {
	records: Vec<MigrationRecord>,
}

/// Database-backed migration recorder
pub struct DatabaseMigrationRecorder {
	connection: DatabaseConnection,
}

impl MigrationRecorder {
	pub fn new() -> Self {
		Self {
			records: Vec::new(),
		}
	}

	pub fn record_applied(&mut self, app: String, name: String) {
		self.records.push(MigrationRecord {
			app,
			name,
			applied: Utc::now(),
		});
	}

	pub fn get_applied_migrations(&self) -> &[MigrationRecord] {
		&self.records
	}

	pub fn is_applied(&self, app: &str, name: &str) -> bool {
		self.records.iter().any(|r| r.app == app && r.name == name)
	}

	pub fn ensure_schema_table(&self) {
		// Ensure migration schema table exists
	}

	// Async versions for database operations
	pub async fn ensure_schema_table_async<T>(&self, _pool: &T) -> crate::Result<()> {
		Ok(())
	}

	pub async fn is_applied_async<T>(
		&self,
		_pool: &T,
		app: &str,
		name: &str,
	) -> crate::Result<bool> {
		Ok(self.is_applied(app, name))
	}

	pub async fn record_applied_async<T>(
		&mut self,
		_pool: &T,
		app: String,
		name: String,
	) -> crate::Result<()> {
		self.record_applied(app, name);
		Ok(())
	}
}

impl Default for MigrationRecorder {
	fn default() -> Self {
		Self::new()
	}
}

impl DatabaseMigrationRecorder {
	/// Create a new database-backed migration recorder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
	/// use backends::DatabaseConnection;
	///
	/// # async fn example() {
	/// // For doctest purposes, using mock connection (URL is ignored in current implementation)
	/// let connection = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let recorder = DatabaseMigrationRecorder::new(connection);
	/// // Verify recorder was created successfully
	/// recorder.ensure_schema_table().await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn new(connection: DatabaseConnection) -> Self {
		Self { connection }
	}

	/// Ensure the migration schema table exists
	///
	/// Creates the `reinhardt_migrations` table if it doesn't exist.
	/// This follows Django's migration table schema.
	pub async fn ensure_schema_table(&self) -> crate::Result<()> {
		use backends::types::DatabaseType;
		use sea_query::{
			Alias, ColumnDef, MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder, Table,
		};

		// Handle MongoDB separately (early return)
		#[cfg(feature = "mongodb")]
		if self.connection.database_type() == DatabaseType::MongoDB {
			use bson::doc;

			let backend = self.connection.backend();
			let backend_any = backend.as_any();

			if let Some(mongo_backend) =
				backend_any.downcast_ref::<backends::drivers::mongodb::MongoDBBackend>()
			{
				let db = mongo_backend.database();
				let collection = db.collection::<bson::Document>("_reinhardt_migrations");

				let indexes = collection.list_index_names().await.map_err(|e| {
					crate::MigrationError::DatabaseError(backends::DatabaseError::ConnectionError(
						e.to_string(),
					))
				})?;

				if indexes.is_empty() {
					use mongodb::IndexModel;
					use mongodb::options::IndexOptions;

					let index = IndexModel::builder()
						.keys(doc! { "app": 1, "name": 1 })
						.options(IndexOptions::builder().unique(true).build())
						.build();

					collection.create_index(index).await.map_err(|e| {
						crate::MigrationError::DatabaseError(
							backends::DatabaseError::ConnectionError(e.to_string()),
						)
					})?;
				}

				return Ok(());
			} else {
				return Err(crate::MigrationError::DatabaseError(
					backends::DatabaseError::ConnectionError(
						"Failed to downcast to MongoDBBackend".to_string(),
					),
				));
			}
		}

		// Build SQL using appropriate query builder based on database type
		// Scope stmt to ensure it's dropped before await
		let sql = {
			let stmt = Table::create()
				.table(Alias::new("reinhardt_migrations"))
				.if_not_exists()
				.col(
					ColumnDef::new(Alias::new("id"))
						.integer()
						.not_null()
						.auto_increment()
						.primary_key(),
				)
				.col(ColumnDef::new(Alias::new("app")).string_len(255).not_null())
				.col(
					ColumnDef::new(Alias::new("name"))
						.string_len(255)
						.not_null(),
				)
				.col(
					ColumnDef::new(Alias::new("applied"))
						.timestamp()
						.not_null()
						.default("CURRENT_TIMESTAMP"),
				)
				.to_owned();

			match self.connection.database_type() {
				DatabaseType::Postgres => stmt.to_string(PostgresQueryBuilder),
				DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				DatabaseType::Sqlite => stmt.to_string(SqliteQueryBuilder),
				#[cfg(feature = "mongodb")]
				DatabaseType::MongoDB => unreachable!("MongoDB handled above"),
			}
		}; // stmt is dropped here, before await

		self.connection
			.execute(&sql, vec![])
			.await
			.map_err(crate::MigrationError::DatabaseError)?;

		Ok(())
	}

	/// Check if a migration has been applied
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
	/// use backends::DatabaseConnection;
	///
	/// # async fn example() {
	/// // For doctest purposes, using mock connection (URL is ignored in current implementation)
	/// let connection = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let recorder = DatabaseMigrationRecorder::new(connection);
	/// recorder.ensure_schema_table().await.unwrap();
	///
	/// let is_applied = recorder.is_applied("myapp", "0001_initial").await.unwrap();
	/// assert!(!is_applied); // Initially not applied
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn is_applied(&self, app: &str, name: &str) -> crate::Result<bool> {
		#[cfg(feature = "mongodb")]
		use backends::types::DatabaseType;

		match self.connection.database_type() {
			#[cfg(feature = "mongodb")]
			DatabaseType::MongoDB => {
				use bson::doc;

				let backend = self.connection.backend();
				let backend_any = backend.as_any();

				if let Some(mongo_backend) =
					backend_any.downcast_ref::<backends::drivers::mongodb::MongoDBBackend>()
				{
					let db = mongo_backend.database();
					let collection = db.collection::<bson::Document>("_reinhardt_migrations");

					let filter = doc! {
						"app": app,
						"name": name
					};

					let count = collection.count_documents(filter).await.map_err(|e| {
						crate::MigrationError::DatabaseError(backends::DatabaseError::QueryError(
							e.to_string(),
						))
					})?;

					Ok(count > 0)
				} else {
					Err(crate::MigrationError::DatabaseError(
						backends::DatabaseError::ConnectionError(
							"Failed to downcast to MongoDBBackend".to_string(),
						),
					))
				}
			}
			_ => {
				let sql = "SELECT EXISTS(SELECT 1 FROM reinhardt_migrations WHERE app = $1 AND name = $2) as exists_flag";
				let params = vec![
					QueryValue::String(app.to_string()),
					QueryValue::String(name.to_string()),
				];

				let rows = self
					.connection
					.fetch_all(sql, params)
					.await
					.map_err(crate::MigrationError::DatabaseError)?;

				if rows.is_empty() {
					return Ok(false);
				}

				let row = &rows[0];

				// Try to get as bool first, then as i64 for databases that return int
				if let Ok(exists) = row.get::<bool>("exists_flag") {
					Ok(exists)
				} else if let Ok(exists_int) = row.get::<i64>("exists_flag") {
					Ok(exists_int > 0)
				} else {
					Ok(false)
				}
			}
		}
	}

	/// Record that a migration has been applied
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
	/// use backends::DatabaseConnection;
	///
	/// # async fn example() {
	/// // For doctest purposes, using mock connection (URL is ignored in current implementation)
	/// let connection = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let recorder = DatabaseMigrationRecorder::new(connection);
	/// recorder.ensure_schema_table().await.unwrap();
	///
	/// recorder.record_applied("myapp", "0001_initial").await.unwrap();
	/// // Verify migration was recorded
	/// let is_applied = recorder.is_applied("myapp", "0001_initial").await.unwrap();
	/// assert!(is_applied);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn record_applied(&self, app: &str, name: &str) -> crate::Result<()> {
		#[cfg(feature = "mongodb")]
		use backends::types::DatabaseType;

		match self.connection.database_type() {
			#[cfg(feature = "mongodb")]
			DatabaseType::MongoDB => {
				use bson::doc;
				use chrono::Utc;

				let backend = self.connection.backend();
				let backend_any = backend.as_any();

				if let Some(mongo_backend) =
					backend_any.downcast_ref::<backends::drivers::mongodb::MongoDBBackend>()
				{
					let db = mongo_backend.database();
					let collection = db.collection::<bson::Document>("_reinhardt_migrations");

					let doc = doc! {
						"app": app,
						"name": name,
						"applied": bson::DateTime::from_millis(Utc::now().timestamp_millis())
					};

					collection.insert_one(doc).await.map_err(|e| {
						crate::MigrationError::DatabaseError(backends::DatabaseError::QueryError(
							e.to_string(),
						))
					})?;

					Ok(())
				} else {
					Err(crate::MigrationError::DatabaseError(
						backends::DatabaseError::ConnectionError(
							"Failed to downcast to MongoDBBackend".to_string(),
						),
					))
				}
			}
			_ => {
				let sql = "INSERT INTO reinhardt_migrations (app, name, applied) VALUES ($1, $2, CURRENT_TIMESTAMP)";
				let params = vec![
					QueryValue::String(app.to_string()),
					QueryValue::String(name.to_string()),
				];

				self.connection
					.execute(sql, params)
					.await
					.map_err(crate::MigrationError::DatabaseError)?;

				Ok(())
			}
		}
	}

	/// Get all applied migrations
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
	/// use backends::DatabaseConnection;
	///
	/// # async fn example() {
	/// // For doctest purposes, using mock connection (URL is ignored in current implementation)
	/// let connection = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let recorder = DatabaseMigrationRecorder::new(connection);
	/// recorder.ensure_schema_table().await.unwrap();
	///
	/// let migrations = recorder.get_applied_migrations().await.unwrap();
	/// assert!(migrations.is_empty()); // Initially no migrations applied
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn get_applied_migrations(&self) -> crate::Result<Vec<MigrationRecord>> {
		#[cfg(feature = "mongodb")]
		use backends::types::DatabaseType;

		match self.connection.database_type() {
			#[cfg(feature = "mongodb")]
			DatabaseType::MongoDB => {
				use bson::doc;
				use futures::stream::TryStreamExt;

				let backend = self.connection.backend();
				let backend_any = backend.as_any();

				if let Some(mongo_backend) =
					backend_any.downcast_ref::<backends::drivers::mongodb::MongoDBBackend>()
				{
					let db = mongo_backend.database();
					let collection = db.collection::<bson::Document>("_reinhardt_migrations");

					let find_options = mongodb::options::FindOptions::builder()
						.sort(doc! { "applied": 1 })
						.build();

					let mut cursor = collection
						.find(doc! {})
						.with_options(find_options)
						.await
						.map_err(|e| {
							crate::MigrationError::DatabaseError(
								backends::DatabaseError::QueryError(e.to_string()),
							)
						})?;

					let mut records = Vec::new();
					while let Some(doc) = cursor.try_next().await.map_err(|e| {
						crate::MigrationError::DatabaseError(backends::DatabaseError::QueryError(
							e.to_string(),
						))
					})? {
						let app = doc
							.get_str("app")
							.map_err(|e| {
								crate::MigrationError::DatabaseError(
									backends::DatabaseError::QueryError(e.to_string()),
								)
							})?
							.to_string();

						let name = doc
							.get_str("name")
							.map_err(|e| {
								crate::MigrationError::DatabaseError(
									backends::DatabaseError::QueryError(e.to_string()),
								)
							})?
							.to_string();

						let applied_bson = doc.get_datetime("applied").map_err(|e| {
							crate::MigrationError::DatabaseError(
								backends::DatabaseError::QueryError(e.to_string()),
							)
						})?;

						let applied = chrono::DateTime::from_timestamp_millis(
							applied_bson.timestamp_millis(),
						)
						.ok_or_else(|| {
							crate::MigrationError::DatabaseError(
								backends::DatabaseError::QueryError(
									"Invalid timestamp".to_string(),
								),
							)
						})?;

						records.push(MigrationRecord { app, name, applied });
					}

					Ok(records)
				} else {
					Err(crate::MigrationError::DatabaseError(
						backends::DatabaseError::ConnectionError(
							"Failed to downcast to MongoDBBackend".to_string(),
						),
					))
				}
			}
			_ => {
				let sql = "SELECT app, name, applied FROM reinhardt_migrations ORDER BY applied";

				let rows = self
					.connection
					.fetch_all(sql, vec![])
					.await
					.map_err(crate::MigrationError::DatabaseError)?;

				let mut records = Vec::new();
				for row in rows {
					let app: String = row
						.get("app")
						.map_err(crate::MigrationError::DatabaseError)?;
					let name: String = row
						.get("name")
						.map_err(crate::MigrationError::DatabaseError)?;

					// Parse timestamp from database
					let applied: DateTime<Utc> = row
						.get("applied")
						.map_err(crate::MigrationError::DatabaseError)?;

					records.push(MigrationRecord { app, name, applied });
				}

				Ok(records)
			}
		}
	}

	/// Unapply a migration (remove from records)
	///
	/// Used when rolling back migrations.
	pub async fn unapply(&self, app: &str, name: &str) -> crate::Result<()> {
		#[cfg(feature = "mongodb")]
		use backends::types::DatabaseType;

		match self.connection.database_type() {
			#[cfg(feature = "mongodb")]
			DatabaseType::MongoDB => {
				use bson::doc;

				let backend = self.connection.backend();
				let backend_any = backend.as_any();

				if let Some(mongo_backend) =
					backend_any.downcast_ref::<backends::drivers::mongodb::MongoDBBackend>()
				{
					let db = mongo_backend.database();
					let collection = db.collection::<bson::Document>("_reinhardt_migrations");

					let filter = doc! {
						"app": app,
						"name": name
					};

					collection.delete_one(filter).await.map_err(|e| {
						crate::MigrationError::DatabaseError(backends::DatabaseError::QueryError(
							e.to_string(),
						))
					})?;

					Ok(())
				} else {
					Err(crate::MigrationError::DatabaseError(
						backends::DatabaseError::ConnectionError(
							"Failed to downcast to MongoDBBackend".to_string(),
						),
					))
				}
			}
			_ => {
				let sql = "DELETE FROM reinhardt_migrations WHERE app = $1 AND name = $2";
				let params = vec![
					QueryValue::String(app.to_string()),
					QueryValue::String(name.to_string()),
				];

				self.connection
					.execute(sql, params)
					.await
					.map_err(crate::MigrationError::DatabaseError)?;

				Ok(())
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Utc;

	#[test]
	fn test_migration_recorder_creation() {
		let recorder = MigrationRecorder::new();
		assert_eq!(recorder.get_applied_migrations().len(), 0);
	}

	#[test]
	fn test_record_applied() {
		let mut recorder = MigrationRecorder::new();
		recorder.record_applied("auth".to_string(), "0001_initial".to_string());

		assert_eq!(recorder.get_applied_migrations().len(), 1);
		assert!(recorder.is_applied("auth", "0001_initial"));
	}

	#[test]
	fn test_is_applied() {
		let mut recorder = MigrationRecorder::new();

		assert!(!recorder.is_applied("auth", "0001_initial"));

		recorder.record_applied("auth".to_string(), "0001_initial".to_string());

		assert!(recorder.is_applied("auth", "0001_initial"));
		assert!(!recorder.is_applied("auth", "0002_add_field"));
	}

	#[test]
	fn test_get_applied_migrations() {
		let mut recorder = MigrationRecorder::new();

		recorder.record_applied("auth".to_string(), "0001_initial".to_string());
		recorder.record_applied("users".to_string(), "0001_initial".to_string());
		recorder.record_applied("auth".to_string(), "0002_add_field".to_string());

		let migrations = recorder.get_applied_migrations();
		assert_eq!(migrations.len(), 3);

		// Verify all migrations were recorded
		assert!(
			migrations
				.iter()
				.any(|m| m.app == "auth" && m.name == "0001_initial")
		);
		assert!(
			migrations
				.iter()
				.any(|m| m.app == "users" && m.name == "0001_initial")
		);
		assert!(
			migrations
				.iter()
				.any(|m| m.app == "auth" && m.name == "0002_add_field")
		);
	}

	#[test]
	fn test_migration_record_contains_timestamp() {
		let mut recorder = MigrationRecorder::new();
		let before = Utc::now();

		recorder.record_applied("auth".to_string(), "0001_initial".to_string());

		let after = Utc::now();
		let migrations = recorder.get_applied_migrations();

		assert_eq!(migrations.len(), 1);
		let record = &migrations[0];

		// Check timestamp is within expected range
		assert!(record.applied >= before);
		assert!(record.applied <= after);
	}

	#[test]
	fn test_multiple_apps_migrations() {
		let mut recorder = MigrationRecorder::new();

		recorder.record_applied("auth".to_string(), "0001_initial".to_string());
		recorder.record_applied("auth".to_string(), "0002_add_field".to_string());
		recorder.record_applied("users".to_string(), "0001_initial".to_string());
		recorder.record_applied("posts".to_string(), "0001_initial".to_string());

		assert!(recorder.is_applied("auth", "0001_initial"));
		assert!(recorder.is_applied("auth", "0002_add_field"));
		assert!(recorder.is_applied("users", "0001_initial"));
		assert!(recorder.is_applied("posts", "0001_initial"));

		assert!(!recorder.is_applied("comments", "0001_initial"));
	}

	#[tokio::test]
	async fn test_async_record_applied() {
		let mut recorder = MigrationRecorder::new();

		recorder
			.record_applied_async(&(), "auth".to_string(), "0001_initial".to_string())
			.await
			.unwrap();

		assert!(recorder.is_applied("auth", "0001_initial"));
	}

	#[tokio::test]
	async fn test_async_is_applied() {
		let mut recorder = MigrationRecorder::new();

		recorder.record_applied("auth".to_string(), "0001_initial".to_string());

		let result = recorder
			.is_applied_async(&(), "auth", "0001_initial")
			.await
			.unwrap();

		assert!(result);

		let result_not_applied = recorder
			.is_applied_async(&(), "auth", "0002_add_field")
			.await
			.unwrap();

		assert!(!result_not_applied);
	}

	#[tokio::test]
	async fn test_ensure_schema_table_async() {
		let recorder = MigrationRecorder::new();
		let result = recorder.ensure_schema_table_async(&()).await;
		assert!(result.is_ok());
	}
}
