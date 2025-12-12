//! Migration executor
//!
//! Translated from Django's db/migrations/executor.py

use crate::{
	DatabaseMigrationRecorder, Migration, MigrationPlan, MigrationRecorder, MigrationService,
	Operation, Result, operations::SqlDialect,
};
use indexmap::IndexMap;
use reinhardt_backends::{connection::DatabaseConnection, types::DatabaseType};
use sqlx::SqlitePool;
use std::collections::HashSet;

/// Split SQL string into individual statements while handling:
/// - String literals (single/double quotes)
/// - Comments (line and block)
/// - PostgreSQL dollar-quotes ($$...$$)
fn split_sql_statements(sql: &str) -> Vec<String> {
	let mut statements = Vec::new();
	let mut current = String::new();
	let mut chars = sql.chars().peekable();

	#[derive(Debug, PartialEq)]
	enum State {
		Normal,
		SingleQuote,
		DoubleQuote,
		LineComment,
		BlockComment,
		DollarQuote(String),
	}

	let mut state = State::Normal;

	while let Some(ch) = chars.next() {
		match state {
			State::Normal => {
				if ch == '\'' {
					current.push(ch);
					state = State::SingleQuote;
				} else if ch == '"' {
					current.push(ch);
					state = State::DoubleQuote;
				} else if ch == '-' && chars.peek() == Some(&'-') {
					current.push(ch);
					current.push(chars.next().unwrap());
					state = State::LineComment;
				} else if ch == '/' && chars.peek() == Some(&'*') {
					current.push(ch);
					current.push(chars.next().unwrap());
					state = State::BlockComment;
				} else if ch == '$' {
					// Potential dollar-quote start
					let mut tag = String::from("$");
					current.push(ch);

					// Collect tag until next $
					while let Some(&next_ch) = chars.peek() {
						if next_ch == '$' {
							tag.push(chars.next().unwrap());
							current.push('$');
							state = State::DollarQuote(tag);
							break;
						} else if next_ch.is_alphanumeric() || next_ch == '_' {
							tag.push(chars.next().unwrap());
							current.push(next_ch);
						} else {
							// Not a valid dollar-quote tag
							break;
						}
					}
				} else if ch == ';' {
					// Statement separator - save current statement if non-empty
					let trimmed = current.trim();
					if !trimmed.is_empty() {
						statements.push(trimmed.to_string());
					}
					current.clear();
				} else {
					current.push(ch);
				}
			}
			State::SingleQuote => {
				current.push(ch);
				if ch == '\'' {
					// Check for escaped quote ''
					if chars.peek() == Some(&'\'') {
						current.push(chars.next().unwrap());
					} else {
						state = State::Normal;
					}
				} else if ch == '\\' && chars.peek().is_some() {
					// Escaped character
					current.push(chars.next().unwrap());
				}
			}
			State::DoubleQuote => {
				current.push(ch);
				if ch == '"' {
					state = State::Normal;
				} else if ch == '\\' && chars.peek().is_some() {
					// Escaped character
					current.push(chars.next().unwrap());
				}
			}
			State::LineComment => {
				current.push(ch);
				if ch == '\n' {
					state = State::Normal;
				}
			}
			State::BlockComment => {
				current.push(ch);
				if ch == '*' && chars.peek() == Some(&'/') {
					current.push(chars.next().unwrap());
					state = State::Normal;
				}
			}
			State::DollarQuote(ref tag) => {
				current.push(ch);
				// Check if we're at the closing tag
				if ch == '$' {
					let mut potential_close = String::from("$");
					let mut temp_chars = vec![];

					// Collect potential closing tag
					while let Some(&next_ch) = chars.peek() {
						if next_ch == '$' {
							potential_close.push(chars.next().unwrap());
							temp_chars.push('$');
							break;
						} else if potential_close.len() < tag.len()
							&& (next_ch.is_alphanumeric() || next_ch == '_')
						{
							potential_close.push(chars.next().unwrap());
							temp_chars.push(next_ch);
						} else {
							break;
						}
					}

					// Add collected characters to current
					for temp_ch in &temp_chars {
						current.push(*temp_ch);
					}

					// Check if it matches the opening tag
					if potential_close == *tag {
						state = State::Normal;
					}
				}
			}
		}
	}

	// Add final statement if non-empty
	let trimmed = current.trim();
	if !trimmed.is_empty() {
		statements.push(trimmed.to_string());
	}

	statements
}

pub struct ExecutionResult {
	pub applied: Vec<String>,
	pub failed: Option<String>,
}

pub struct MigrationExecutor {
	pool: SqlitePool,
	recorder: MigrationRecorder,
}

/// Migration executor using DatabaseConnection (supports multiple database types)
pub struct DatabaseMigrationExecutor {
	connection: DatabaseConnection,
	recorder: DatabaseMigrationRecorder,
	db_type: DatabaseType,
}

impl MigrationExecutor {
	/// Create a new migration executor
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_migrations::executor::MigrationExecutor;
	/// use sqlx::SqlitePool;
	///
	/// # async fn example() {
	/// let pool = SqlitePool::connect(":memory:").await.unwrap();
	/// let executor = MigrationExecutor::new(pool);
	/// // Executor created successfully
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn new(pool: SqlitePool) -> Self {
		Self {
			pool,
			recorder: MigrationRecorder::new(),
		}
	}
	/// Get a reference to the database pool
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_migrations::executor::MigrationExecutor;
	/// use sqlx::SqlitePool;
	///
	/// # async fn example() {
	/// let pool = SqlitePool::connect(":memory:").await.unwrap();
	/// let executor = MigrationExecutor::new(pool);
	/// let pool_ref = executor.get_pool();
	/// // Pool reference retrieved successfully
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn get_pool(&self) -> &SqlitePool {
		&self.pool
	}
	/// Apply a list of migrations
	/// Translated from Django's MigrationExecutor.migrate() and apply_migration()
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_migrations::{Migration, executor::MigrationExecutor};
	/// use sqlx::SqlitePool;
	///
	/// # async fn example() {
	/// let pool = SqlitePool::connect(":memory:").await.unwrap();
	/// let mut executor = MigrationExecutor::new(pool);
	///
	/// let migrations = vec![Migration::new("0001_initial", "myapp")];
	/// let result = executor.apply_migrations(&migrations).await.unwrap();
	/// // Migrations applied successfully
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn apply_migrations(&mut self, migrations: &[Migration]) -> Result<ExecutionResult> {
		let mut applied = Vec::new();

		// Ensure the migration recorder table exists
		self.recorder.ensure_schema_table_async(&self.pool).await?;

		for migration in migrations {
			// Check if already applied
			let is_applied = self
				.recorder
				.is_applied_async(&self.pool, migration.app_label, migration.name)
				.await?;

			if is_applied {
				continue;
			}

			// Apply migration operations
			self.apply_migration(migration).await?;

			// Record migration as applied
			self.recorder
				.record_applied_async(&self.pool, migration.app_label, migration.name)
				.await?;

			applied.push(migration.id());
		}

		Ok(ExecutionResult {
			applied,
			failed: None,
		})
	}

	/// Apply a single migration
	/// Translated from Django's MigrationExecutor.apply_migration()
	async fn apply_migration(&self, migration: &Migration) -> Result<()> {
		// In Django, this uses schema_editor with atomic transaction support
		// TODO: For now, we apply operations directly
		for operation in &migration.operations {
			let sql = operation.to_sql(&SqlDialect::Sqlite);

			// Diagnostic output: Original SQL
			tracing::debug!("=== Original SQL ===");
			tracing::debug!("{}", sql);
			tracing::debug!("SQL length: {} characters", sql.len());
			tracing::debug!("Semicolons: {}", sql.matches(';').count());

			// Split SQL into individual statements to handle PostgreSQL's
			// prepared statement limitation (cannot execute multiple commands)
			let statements = split_sql_statements(&sql);

			// Diagnostic output: Number of split statements
			tracing::debug!("\n=== Split into {} statements ===", statements.len());

			for (i, statement) in statements.iter().enumerate() {
				if !statement.trim().is_empty() {
					// Diagnostic output: Each statement
					tracing::debug!(
						"\n--- Statement {} (length: {} chars) ---",
						i + 1,
						statement.len()
					);
					tracing::debug!("{}", statement);

					sqlx::query(statement).execute(&self.pool).await?;

					tracing::debug!("✅ Statement {} executed successfully", i + 1);
				} else {
					eprintln!("\n--- Statement {} (EMPTY - skipped) ---", i + 1);
				}
			}
		}

		Ok(())
	}
	/// Original apply method for MigrationPlan
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_migrations::{MigrationPlan, executor::MigrationExecutor};
	/// use sqlx::SqlitePool;
	///
	/// # async fn example() {
	/// let pool = SqlitePool::connect(":memory:").await.unwrap();
	/// let mut executor = MigrationExecutor::new(pool);
	///
	/// let plan = MigrationPlan::new();
	/// let result = executor.apply(&plan).await.unwrap();
	/// // Migration plan applied successfully
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn apply(&mut self, plan: &MigrationPlan) -> Result<ExecutionResult> {
		let mut applied = Vec::new();

		for migration in &plan.migrations {
			// Check if already applied
			let is_applied = self
				.recorder
				.is_applied_async(&self.pool, migration.app_label, migration.name)
				.await?;

			if is_applied {
				continue;
			}

			// Apply migration
			for operation in &migration.operations {
				let sql = operation.to_sql(&SqlDialect::Sqlite);

				// Split SQL into individual statements to handle PostgreSQL's
				// prepared statement limitation (cannot execute multiple commands)
				for statement in split_sql_statements(&sql) {
					if !statement.trim().is_empty() {
						sqlx::query(&statement).execute(&self.pool).await?;
					}
				}
			}

			// Record migration
			self.recorder
				.record_applied_async(&self.pool, migration.app_label, migration.name)
				.await?;
			applied.push(migration.id());
		}

		Ok(ExecutionResult {
			applied,
			failed: None,
		})
	}

	/// Build migration plan - returns list of migrations to apply
	///
	/// Returns (app_label, migration_name) tuples in dependency order
	pub async fn build_plan(&self, service: &MigrationService) -> Result<Vec<(String, String)>> {
		let graph = service.build_dependency_graph().await?;
		let mut plan = Vec::new();

		for migration in graph {
			let is_applied = self
				.recorder
				.is_applied_async(&self.pool, migration.app_label, migration.name)
				.await?;

			if !is_applied {
				plan.push((migration.app_label.to_string(), migration.name.to_string()));
			}
		}

		Ok(plan)
	}

	/// Record a migration as applied without actually running it
	pub fn record_migration(&mut self, app_label: &str, migration_name: &str) -> Result<()> {
		self.recorder.record_applied(app_label, migration_name);
		Ok(())
	}

	/// Execute a migration by loading it from the service
	pub async fn execute_migration(
		&mut self,
		app_label: &str,
		migration_name: &str,
		service: &MigrationService,
	) -> Result<()> {
		let migration = service.load_migration(app_label, migration_name).await?;

		// Apply operations
		self.apply_migration(&migration).await?;

		// Record as applied
		self.recorder
			.record_applied_async(&self.pool, migration.app_label, migration.name)
			.await?;

		Ok(())
	}
}

impl DatabaseMigrationExecutor {
	/// Create a new migration executor with DatabaseConnection
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::executor::DatabaseMigrationExecutor;
	/// use reinhardt_backends::{DatabaseConnection, DatabaseType};
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// // For doctest purposes, using SQLite in-memory instead of PostgreSQL
	/// let db = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let executor = DatabaseMigrationExecutor::new(db.clone(), DatabaseType::Sqlite);
	/// # });
	/// ```
	pub fn new(connection: DatabaseConnection, db_type: DatabaseType) -> Self {
		let recorder = DatabaseMigrationRecorder::new(connection.clone());
		Self {
			connection,
			recorder,
			db_type,
		}
	}

	/// Get a reference to the database connection
	pub fn connection(&self) -> &DatabaseConnection {
		&self.connection
	}

	/// Get the database type
	pub fn database_type(&self) -> DatabaseType {
		self.db_type
	}

	/// Check if a table exists in the database
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_migrations::executor::DatabaseMigrationExecutor;
	/// use reinhardt_backends::{DatabaseConnection, DatabaseType};
	///
	/// # async fn example() {
	/// let db = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let executor = DatabaseMigrationExecutor::new(db, DatabaseType::Sqlite);
	/// let exists = executor.table_exists("users").await.unwrap();
	/// # }
	/// ```
	async fn table_exists(&self, table_name: &str) -> Result<bool> {
		match self.db_type {
			DatabaseType::Postgres => {
				let query = format!(
					"SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '{}')",
					table_name
				);

				// For PostgreSQL, EXISTS returns a boolean value
				let result = self.connection.fetch_one(&query, vec![]).await?;
				match result.data.get("exists") {
					Some(reinhardt_backends::types::QueryValue::Bool(b)) => Ok(*b),
					_ => Ok(false),
				}
			}
			DatabaseType::Sqlite => {
				let query = format!(
					"SELECT name FROM sqlite_master WHERE type='table' AND name = '{}'",
					table_name
				);

				// For SQLite, check if any row is returned
				let result = self.connection.fetch_optional(&query, vec![]).await?;
				Ok(result.is_some())
			}
			DatabaseType::Mysql => {
				let query = format!(
					"SELECT TABLE_NAME FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = '{}'",
					table_name
				);

				// For MySQL, check if any row is returned
				let result = self.connection.fetch_optional(&query, vec![]).await?;
				Ok(result.is_some())
			}
			#[cfg(feature = "mongodb-backend")]
			DatabaseType::MongoDB => {
				// MongoDB is schemaless, tables always "exist" conceptually
				Ok(true)
			}
		}
	}

	/// Apply a list of migrations
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_migrations::{Migration, executor::DatabaseMigrationExecutor};
	/// use reinhardt_backends::{DatabaseConnection, DatabaseType};
	///
	/// # async fn example() {
	/// let db = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let mut executor = DatabaseMigrationExecutor::new(db, DatabaseType::Sqlite);
	///
	/// let migrations = vec![Migration::new("0001_initial", "myapp")];
	/// let result = executor.apply_migrations(&migrations).await.unwrap();
	/// # }
	/// ```
	pub async fn apply_migrations(&mut self, migrations: &[Migration]) -> Result<ExecutionResult> {
		let mut applied = Vec::new();

		// Ensure the migration recorder table exists
		self.recorder.ensure_schema_table().await?;

		for migration in migrations {
			// Check if already applied
			if self
				.recorder
				.is_applied(migration.app_label, migration.name)
				.await?
			{
				continue;
			}

			// Apply migration operations
			self.apply_migration(migration).await?;

			// Record migration as applied
			self.recorder
				.record_applied(migration.app_label, migration.name)
				.await?;

			applied.push(migration.id());
		}

		Ok(ExecutionResult {
			applied,
			failed: None,
		})
	}

	/// Apply a single migration
	async fn apply_migration(&self, migration: &Migration) -> Result<()> {
		// Convert SqlDialect based on database type
		#[cfg(feature = "mongodb-backend")]
		if matches!(self.db_type, DatabaseType::MongoDB) {
			// MongoDB is schemaless, so structural migrations don't apply
			// Only data migrations and index operations are relevant
			// Skip SQL-based schema operations for MongoDB
			return Ok(());
		}

		let dialect = match self.db_type {
			DatabaseType::Postgres => SqlDialect::Postgres,
			DatabaseType::Sqlite => SqlDialect::Sqlite,
			DatabaseType::Mysql => SqlDialect::Mysql,
			#[cfg(feature = "mongodb-backend")]
			DatabaseType::MongoDB => unreachable!("MongoDB handled above"),
		};

		for operation in &migration.operations {
			// Check if this is a CreateTable operation and if the table already exists
			if let Operation::CreateTable { name, .. } = operation {
				let table_exists = self.table_exists(name).await?;
				if table_exists {
					eprintln!(
						"⏭️  Table '{}' already exists, skipping CREATE TABLE operation",
						name
					);
					continue;
				}
			}

			let sql = operation.to_sql(&dialect);

			// 診断出力: 元のSQL
			eprintln!("=== DatabaseMigrationExecutor: Original SQL ===");
			eprintln!("{}", sql);
			eprintln!("SQL length: {} characters", sql.len());
			eprintln!("Semicolons: {}", sql.matches(';').count());
			eprintln!("Database type: {:?}", self.db_type);

			// Split SQL into individual statements to handle PostgreSQL's
			// prepared statement limitation (cannot execute multiple commands)
			let statements = split_sql_statements(&sql);

			// Diagnostic output: Number of split statements
			tracing::debug!("\n=== Split into {} statements ===", statements.len());

			for (i, statement) in statements.iter().enumerate() {
				if !statement.trim().is_empty() {
					// Diagnostic output: Each statement
					tracing::debug!(
						"\n--- Statement {} (length: {} chars) ---",
						i + 1,
						statement.len()
					);
					tracing::debug!("{}", statement);

					self.connection.execute(statement, vec![]).await?;

					tracing::debug!("✅ Statement {} executed successfully", i + 1);
				} else {
					eprintln!("\n--- Statement {} (EMPTY - skipped) ---", i + 1);
				}
			}
		}

		Ok(())
	}

	/// Apply migrations from a MigrationPlan
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MigrationPlan, executor::DatabaseMigrationExecutor};
	/// use reinhardt_backends::{DatabaseConnection, DatabaseType};
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// // For doctest purposes, using SQLite in-memory instead of PostgreSQL
	/// let db = DatabaseConnection::connect_sqlite(":memory:").await.unwrap();
	/// let mut executor = DatabaseMigrationExecutor::new(db, DatabaseType::Sqlite);
	///
	/// let plan = MigrationPlan::new();
	/// let result = executor.apply(&plan).await.unwrap();
	/// # });
	/// ```
	pub async fn apply(&mut self, plan: &MigrationPlan) -> Result<ExecutionResult> {
		let mut applied = Vec::new();

		// Ensure the migration recorder table exists
		self.recorder.ensure_schema_table().await?;

		let dialect = match self.db_type {
			DatabaseType::Postgres => SqlDialect::Postgres,
			DatabaseType::Sqlite => SqlDialect::Sqlite,
			DatabaseType::Mysql => SqlDialect::Mysql,
			#[cfg(feature = "mongodb-backend")]
			DatabaseType::MongoDB => SqlDialect::Postgres, // Placeholder for MongoDB
		};

		for migration in &plan.migrations {
			// Check if already applied
			if self
				.recorder
				.is_applied(migration.app_label, migration.name)
				.await?
			{
				continue;
			}

			// Apply migration
			#[cfg(feature = "mongodb-backend")]
			let is_mongodb = matches!(self.db_type, DatabaseType::MongoDB);
			#[cfg(not(feature = "mongodb-backend"))]
			let is_mongodb = false;

			if !is_mongodb {
				for operation in &migration.operations {
					// Check if this is a CreateTable operation and if the table already exists
					if let Operation::CreateTable { name, .. } = operation {
						let table_exists = self.table_exists(name).await?;
						if table_exists {
							eprintln!(
								"⏭️  Table '{}' already exists, skipping CREATE TABLE operation",
								name
							);
							continue;
						}
					}

					let sql = operation.to_sql(&dialect);

					// Split SQL into individual statements to handle PostgreSQL's
					// prepared statement limitation (cannot execute multiple commands)
					for statement in split_sql_statements(&sql) {
						if !statement.trim().is_empty() {
							self.connection.execute(&statement, vec![]).await?;
						}
					}
				}
			}
			// For MongoDB, skip SQL operations (schemaless)

			// Record migration as applied
			self.recorder
				.record_applied(migration.app_label, migration.name)
				.await?;

			applied.push(migration.id());
		}

		Ok(ExecutionResult {
			applied,
			failed: None,
		})
	}

	/// Build migration plan - returns list of migrations to apply
	///
	/// Returns (app_label, migration_name) tuples in dependency order
	#[allow(dead_code)]
	pub async fn build_plan(&self, service: &MigrationService) -> Result<Vec<(String, String)>> {
		let graph = service.build_dependency_graph().await?;
		let mut plan = Vec::new();

		for migration in graph {
			let is_applied = self
				.recorder
				.is_applied(migration.app_label, migration.name)
				.await?;

			if !is_applied {
				plan.push((migration.app_label.to_string(), migration.name.to_string()));
			}
		}

		Ok(plan)
	}

	/// Record a migration as applied without actually running it
	pub async fn record_migration(&mut self, app_label: &str, migration_name: &str) -> Result<()> {
		self.recorder
			.record_applied(app_label, migration_name)
			.await?;
		Ok(())
	}

	/// Execute a migration by loading it from the service
	#[allow(dead_code)]
	pub async fn execute_migration(
		&mut self,
		app_label: &str,
		migration_name: &str,
		service: &MigrationService,
	) -> Result<()> {
		let migration = service.load_migration(app_label, migration_name).await?;

		// Apply operations
		self.apply_migration(&migration).await?;

		// Record as applied
		self.recorder
			.record_applied(migration.app_label, migration.name)
			.await?;

		Ok(())
	}
}

/// Operation optimizer for migration execution
///
/// Reorders and optimizes operations for better performance and safety.
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::executor::OperationOptimizer;
/// use reinhardt_migrations::{Operation, ColumnDefinition, FieldType};
///
/// let ops = vec![
///     Operation::AddColumn {
///         table: "users",
///         column: ColumnDefinition::new("name", FieldType::VarChar(100)),
///     },
///     Operation::CreateTable {
///         name: "users",
///         columns: vec![],
///         constraints: vec![],
///     },
/// ];
///
/// let optimizer = OperationOptimizer::new();
/// let optimized = optimizer.optimize(ops);
/// // CreateTable should come before AddColumn
/// ```
pub struct OperationOptimizer {
	_private: (),
}

impl OperationOptimizer {
	/// Create a new operation optimizer
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_migrations::executor::OperationOptimizer;
	///
	/// let optimizer = OperationOptimizer::new();
	/// ```
	pub fn new() -> Self {
		Self { _private: () }
	}

	/// Optimize and reorder operations
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_migrations::executor::OperationOptimizer;
	/// use reinhardt_migrations::{Operation, ColumnDefinition};
	///
	/// let ops = vec![
	///     Operation::CreateTable {
	///         name: "users",
	///         columns: vec![],
	///         constraints: vec![],
	///     },
	/// ];
	///
	/// let optimizer = OperationOptimizer::new();
	/// let optimized = optimizer.optimize(ops);
	/// assert_eq!(optimized.len(), 1);
	/// ```
	pub fn optimize(&self, operations: Vec<Operation>) -> Vec<Operation> {
		let mut optimized = operations;

		// Step 1: Reorder operations by dependency
		optimized = self.reorder_by_dependency(optimized);

		// Step 2: Group similar operations
		optimized = self.group_similar_operations(optimized);

		// Step 3: Remove redundant operations
		optimized = self.remove_redundant_operations(optimized);

		optimized
	}

	/// Reorder operations to respect dependencies
	fn reorder_by_dependency(&self, operations: Vec<Operation>) -> Vec<Operation> {
		let mut ordered = Vec::new();
		let mut remaining = operations;
		let mut created_tables = HashSet::new();

		// Priority order:
		// 1. CreateTable (sorted by foreign key dependencies)
		// 2. AddColumn
		// 3. AlterColumn
		// 4. CreateIndex
		// 5. AddConstraint
		// 6. RunSQL
		// 7. RenameColumn
		// 8. DropColumn
		// 9. DropTable

		// First pass: Create tables (respecting foreign key dependencies)
		// Extract all CreateTable operations
		let mut create_table_ops = Vec::new();
		let mut i = 0;
		while i < remaining.len() {
			if matches!(&remaining[i], Operation::CreateTable { .. }) {
				create_table_ops.push(remaining.remove(i));
			} else {
				i += 1;
			}
		}

		// Sort CreateTable operations by dependencies using topological sort
		while !create_table_ops.is_empty() {
			let mut found_independent = false;

			for i in 0..create_table_ops.len() {
				if let Operation::CreateTable {
					name, constraints, ..
				} = &create_table_ops[i]
				{
					// Extract foreign key references from constraints
					let mut depends_on_uncreated = false;
					for constraint in constraints {
						if let Some(referenced_table) =
							self.extract_foreign_key_reference(constraint)
						{
							// Check if the referenced table has been created
							if !created_tables.contains(&referenced_table.as_str())
								&& referenced_table != *name
							{
								depends_on_uncreated = true;
								break;
							}
						}
					}

					// If this table doesn't depend on any uncreated table, we can create it now
					if !depends_on_uncreated {
						// Copy the name before removing the operation
						let name_copy = *name;
						let op = create_table_ops.remove(i);
						created_tables.insert(name_copy);
						ordered.push(op);
						found_independent = true;
						break;
					}
				}
			}

			// If we couldn't find any independent table, just add the remaining tables
			// (this handles circular dependencies or malformed constraints)
			if !found_independent {
				for op in create_table_ops.drain(..) {
					if let Operation::CreateTable { name, .. } = op {
						created_tables.insert(name);
					}
					ordered.push(op);
				}
				break;
			}
		}

		// Second pass: Add columns (for all tables)
		i = 0;
		while i < remaining.len() {
			if let Operation::AddColumn { .. } = &remaining[i] {
				ordered.push(remaining.remove(i));
			} else {
				i += 1;
			}
		}

		// Third pass: Other operations
		ordered.extend(remaining);

		ordered
	}

	/// Extract the referenced table name from a FOREIGN KEY constraint
	/// Returns the referenced table name if the constraint is a ForeignKey
	fn extract_foreign_key_reference(&self, constraint: &crate::Constraint) -> Option<String> {
		match constraint {
			crate::Constraint::ForeignKey {
				referenced_table, ..
			} => Some(referenced_table.clone()),
			_ => None,
		}
	}

	/// Group similar operations together
	fn group_similar_operations(&self, operations: Vec<Operation>) -> Vec<Operation> {
		let mut by_table: IndexMap<String, Vec<Operation>> = IndexMap::new();
		let mut create_ops = Vec::new();
		let mut other_ops = Vec::new();

		for op in operations {
			match &op {
				Operation::CreateTable { .. } => {
					// CreateTable operations go first
					create_ops.push(op);
				}
				Operation::AddColumn { table, .. }
				| Operation::DropColumn { table, .. }
				| Operation::AlterColumn { table, .. } => {
					by_table.entry(table.to_string()).or_default().push(op);
				}
				_ => {
					other_ops.push(op);
				}
			}
		}

		let mut grouped = Vec::new();

		// Add create table operations first
		grouped.extend(create_ops);

		// Add table-specific operations grouped by table
		for (_, ops) in by_table {
			grouped.extend(ops);
		}

		// Add other operations
		grouped.extend(other_ops);

		grouped
	}

	/// Remove redundant operations by detecting cancellations and merging similar operations
	fn remove_redundant_operations(&self, operations: Vec<Operation>) -> Vec<Operation> {
		let mut optimized = Vec::new();
		let mut removed_indices = HashSet::new();

		// Pass 1: Detect and remove operation cancellations
		for i in 0..operations.len() {
			if removed_indices.contains(&i) {
				continue;
			}

			let op = &operations[i];
			let mut found_cancellation = false;

			// Search forward for cancelling operations
			for (j, next_op) in operations.iter().enumerate().skip(i + 1) {
				if removed_indices.contains(&j) {
					continue;
				}

				// Check for cancellation patterns
				let cancels = match (op, next_op) {
					// CreateTable + DropTable
					(
						Operation::CreateTable { name: n1, .. },
						Operation::DropTable { name: n2 },
					) if n1 == n2 => true,
					// AddColumn + DropColumn
					(
						Operation::AddColumn {
							table: t1,
							column: col1,
						},
						Operation::DropColumn {
							table: t2,
							column: col2,
						},
					) if t1 == t2 && col1.name == *col2 => true,
					// CreateIndex + DropIndex
					(
						Operation::CreateIndex {
							table: t1,
							columns: c1,
							..
						},
						Operation::DropIndex {
							table: t2,
							columns: c2,
						},
					) if t1 == t2 && c1 == c2 => true,
					// AddConstraint + DropConstraint (approximate match by table)
					(
						Operation::AddConstraint {
							table: t1,
							constraint_sql: _,
						},
						Operation::DropConstraint {
							table: t2,
							constraint_name: _,
						},
					) if t1 == t2 => {
						// NOTE: Perfect matching requires parsing constraint SQL to extract name
						// This is approximate optimization - matches by table only
						true
					}
					_ => false,
				};

				if cancels {
					removed_indices.insert(i);
					removed_indices.insert(j);
					found_cancellation = true;
					break;
				}
			}

			if !found_cancellation {
				optimized.push(op.clone());
			}
		}

		// Pass 1.5: Remove duplicate CreateTable operations (keep last occurrence)
		let mut deduped = Vec::new();
		let mut create_table_map: IndexMap<String, Operation> = IndexMap::new();

		for operation in optimized {
			match &operation {
				Operation::CreateTable { name, .. } => {
					// Last CreateTable for same table wins
					create_table_map.insert(name.to_string(), operation.clone());
				}
				_ => {
					// Flush accumulated CreateTable operations before non-CreateTable operation
					for (_, create_op) in create_table_map.drain(..) {
						deduped.push(create_op);
					}
					deduped.push(operation);
				}
			}
		}

		// Flush remaining CreateTable operations
		for (_, create_op) in create_table_map {
			deduped.push(create_op);
		}

		// Pass 2: Merge consecutive AlterColumn operations on same column
		let mut merged = Vec::new();
		let mut alter_column_map: IndexMap<(String, String), Operation> = IndexMap::new();

		for operation in deduped {
			match &operation {
				Operation::AlterColumn {
					table,
					column,
					new_definition: _,
				} => {
					let key = (table.to_string(), column.to_string());
					// Last AlterColumn wins (overwrites previous)
					alter_column_map.insert(key, operation.clone());
				}
				_ => {
					// Flush accumulated AlterColumn operations before non-AlterColumn operation
					for (_, alter_op) in alter_column_map.drain(..) {
						merged.push(alter_op);
					}
					merged.push(operation);
				}
			}
		}

		// Flush remaining AlterColumn operations
		for (_, alter_op) in alter_column_map {
			merged.push(alter_op);
		}

		// Pass 3: Chain consecutive RenameTable operations
		let mut chained = Vec::new();
		let mut rename_chain: IndexMap<&'static str, &'static str> = IndexMap::new(); // original_name -> current_name

		for operation in merged {
			match &operation {
				Operation::RenameTable { old_name, new_name } => {
					// Find if any existing chain ends with this old_name
					let mut found_chain = None;
					for (original, current) in &rename_chain {
						if current == old_name {
							found_chain = Some(original);
							break;
						}
					}

					if let Some(original) = found_chain {
						// Extend existing chain: original -> new_name
						rename_chain
							.insert(original, Box::leak(new_name.to_string().into_boxed_str()));
					} else {
						// Start new chain: old_name -> new_name
						rename_chain.insert(
							Box::leak(old_name.to_string().into_boxed_str()),
							Box::leak(new_name.to_string().into_boxed_str()),
						);
					}
				}
				_ => {
					// Flush accumulated RenameTable chains before non-RenameTable operation
					for (original_name, final_name) in rename_chain.drain(..) {
						chained.push(Operation::RenameTable {
							old_name: original_name,
							new_name: final_name,
						});
					}
					chained.push(operation);
				}
			}
		}

		// Flush remaining RenameTable chains
		for (original_name, final_name) in rename_chain {
			chained.push(Operation::RenameTable {
				old_name: original_name,
				new_name: final_name,
			});
		}

		chained
	}
}

impl Default for OperationOptimizer {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod optimizer_tests {
	use super::*;
	use crate::{ColumnDefinition, FieldType};

	#[test]
	fn test_optimizer_creation() {
		let optimizer = OperationOptimizer::new();
		let ops = vec![];
		let optimized = optimizer.optimize(ops);
		assert_eq!(optimized.len(), 0);
	}

	#[test]
	fn test_reorder_create_before_add() {
		let optimizer = OperationOptimizer::new();

		let ops = vec![
			Operation::AddColumn {
				table: "users",
				column: ColumnDefinition::new("name", FieldType::VarChar(100)),
			},
			Operation::CreateTable {
				name: "users",
				columns: vec![],
				constraints: vec![],
			},
		];

		let optimized = optimizer.optimize(ops);

		// CreateTable should come before AddColumn
		assert!(matches!(optimized[0], Operation::CreateTable { .. }));
		assert!(matches!(optimized[1], Operation::AddColumn { .. }));
	}

	#[test]
	fn test_remove_duplicate_create_table() {
		let optimizer = OperationOptimizer::new();

		let ops = vec![
			Operation::CreateTable {
				name: "users",
				columns: vec![],
				constraints: vec![],
			},
			Operation::CreateTable {
				name: "users",
				columns: vec![],
				constraints: vec![],
			},
		];

		let optimized = optimizer.optimize(ops);
		assert_eq!(optimized.len(), 1);
	}

	#[test]
	fn test_group_operations_by_table() {
		let optimizer = OperationOptimizer::new();

		let ops = vec![
			Operation::AddColumn {
				table: "users",
				column: ColumnDefinition::new("name", FieldType::VarChar(100)),
			},
			Operation::CreateTable {
				name: "posts",
				columns: vec![],
				constraints: vec![],
			},
			Operation::AddColumn {
				table: "users",
				column: ColumnDefinition::new("email", FieldType::VarChar(255)),
			},
		];

		let optimized = optimizer.optimize(ops);
		assert_eq!(optimized.len(), 3);
	}

	#[cfg(test)]
	mod split_sql_tests {
		use super::super::*;

		#[test]
		fn test_split_simple_statements() {
			let sql = "CREATE TABLE t1 (id INT); CREATE TABLE t2 (id INT);";
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert_eq!(result[0], "CREATE TABLE t1 (id INT)");
			assert_eq!(result[1], "CREATE TABLE t2 (id INT)");
		}

		#[test]
		fn test_split_with_string_literals() {
			let sql = r#"INSERT INTO t VALUES ('a;b'); INSERT INTO t VALUES ('c;d');"#;
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert_eq!(result[0], "INSERT INTO t VALUES ('a;b')");
			assert_eq!(result[1], "INSERT INTO t VALUES ('c;d')");
		}

		#[test]
		fn test_split_with_line_comments() {
			// Line comment after semicolon becomes part of next statement
			let sql =
				"CREATE TABLE t1 (id INT); -- comment; with semicolon\nCREATE TABLE t2 (id INT);";
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert_eq!(result[0], "CREATE TABLE t1 (id INT)");
			assert!(result[1].contains("-- comment"));
			assert!(result[1].contains("CREATE TABLE t2"));
		}

		#[test]
		fn test_split_with_block_comments() {
			let sql =
				"CREATE TABLE t1 (id INT); /* comment; with semicolon */ CREATE TABLE t2 (id INT);";
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert!(result[0].contains("CREATE TABLE t1"));
			assert!(result[1].contains("CREATE TABLE t2"));
		}

		#[test]
		fn test_split_with_dollar_quotes() {
			let sql = r#"CREATE FUNCTION f() RETURNS text AS $$SELECT 'value; with semicolon';$$ LANGUAGE sql; CREATE TABLE t1 (id INT);"#;
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert!(result[0].contains("CREATE FUNCTION"));
			assert!(result[0].contains("value; with semicolon"));
			assert!(result[1].contains("CREATE TABLE t1"));
		}

		#[test]
		fn test_split_with_escaped_quotes() {
			let sql = r#"INSERT INTO t VALUES ('it''s a test; value'); INSERT INTO t VALUES ('another');"#;
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert!(result[0].contains("it''s a test; value"));
			assert!(result[1].contains("another"));
		}

		#[test]
		fn test_split_empty_statements() {
			let sql = ";;; CREATE TABLE t1 (id INT); ;";
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 1);
			assert_eq!(result[0], "CREATE TABLE t1 (id INT)");
		}

		#[test]
		fn test_split_no_semicolon() {
			let sql = "CREATE TABLE t1 (id INT)";
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 1);
			assert_eq!(result[0], "CREATE TABLE t1 (id INT)");
		}

		#[test]
		fn test_split_whitespace_handling() {
			let sql = "  CREATE TABLE t1 (id INT)  ;  \n\n  CREATE TABLE t2 (id INT)  ;  ";
			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);
			assert_eq!(result[0], "CREATE TABLE t1 (id INT)");
			assert_eq!(result[1], "CREATE TABLE t2 (id INT)");
		}

		#[test]
		fn test_split_seaquery_migration_sql() {
			// Actual SQL generated by SeaQuery for polls migration (from diagnostic test)
			let sql = r###"CREATE TABLE "questions_table" ( "id" bigint GENERATED BY DEFAULT AS IDENTITY NOT NULL PRIMARY KEY, "question_text" text NOT NULL, "pub_date" timestamp with time zone NOT NULL );

CREATE TABLE "choices_table" ( "id" bigint GENERATED BY DEFAULT AS IDENTITY NOT NULL PRIMARY KEY, "question_id" bigint NOT NULL, "choice_text" text NOT NULL, "votes" integer NOT NULL DEFAULT 0, FOREIGN KEY ("question_id") REFERENCES "questions_table" ("id") ON DELETE CASCADE );"###;
			let result = split_sql_statements(sql);

			// Should split into exactly 2 statements (not 3 with empty string)
			assert_eq!(
				result.len(),
				2,
				"Expected 2 statements, got {}",
				result.len()
			);

			// First statement should be CREATE TABLE questions_table
			assert!(
				result[0].contains("questions_table"),
				"First statement should contain 'questions_table'"
			);
			assert!(
				result[0].contains("question_text"),
				"First statement should contain 'question_text'"
			);
			assert!(
				!result[0].contains("choices_table"),
				"First statement should not contain 'choices_table'"
			);

			// Second statement should be CREATE TABLE choices_table
			assert!(
				result[1].contains("choices_table"),
				"Second statement should contain 'choices_table'"
			);
			assert!(
				result[1].contains("choice_text"),
				"Second statement should contain 'choice_text'"
			);
			// Verify reference to questions_table, as FOREIGN KEY constraint contains referenced table name
			assert!(
				result[1].contains("FOREIGN KEY"),
				"Second statement should contain FOREIGN KEY constraint"
			);
			assert!(
				result[1].contains("REFERENCES \"questions_table\""),
				"FOREIGN KEY should reference questions_table"
			);
		}

		#[test]
		fn test_split_multiple_foreign_keys() {
			// Case where table has multiple FOREIGN KEY constraints
			let sql = r###"CREATE TABLE "posts" ("id" bigint PRIMARY KEY);
CREATE TABLE "users" ("id" bigint PRIMARY KEY);
CREATE TABLE "comments" (
	"id" bigint PRIMARY KEY,
	"post_id" bigint,
	"user_id" bigint,
	FOREIGN KEY ("post_id") REFERENCES "posts" ("id"),
	FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);"###;

			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 3, "Expected 3 statements");

			// Third statement contains 2 FOREIGN KEY constraints
			assert_eq!(
				result[2].matches("FOREIGN KEY").count(),
				2,
				"Third statement should contain 2 FOREIGN KEY constraints"
			);
			assert!(
				result[2].contains("REFERENCES \"posts\""),
				"Should reference posts table"
			);
			assert!(
				result[2].contains("REFERENCES \"users\""),
				"Should reference users table"
			);
		}

		#[test]
		fn test_split_mixed_constraints() {
			// Case with mixed CHECK constraint and FOREIGN KEY
			let sql = r###"CREATE TABLE "tasks" ("id" bigint PRIMARY KEY);
CREATE TABLE "task_status" (
	"id" bigint PRIMARY KEY,
	"task_id" bigint,
	"status" text CHECK (status IN ('pending', 'completed')),
	FOREIGN KEY ("task_id") REFERENCES "tasks" ("id")
);"###;

			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 2);

			// Second statement contains both CHECK constraint and FOREIGN KEY constraint
			assert!(
				result[1].contains("CHECK"),
				"Second statement should contain CHECK constraint"
			);
			assert!(
				result[1].contains("FOREIGN KEY"),
				"Second statement should contain FOREIGN KEY constraint"
			);
		}

		#[test]
		fn test_split_self_referencing_foreign_key() {
			// Case with self-referencing FOREIGN KEY
			let sql = r###"CREATE TABLE "categories" (
	"id" bigint PRIMARY KEY,
	"parent_id" bigint,
	FOREIGN KEY ("parent_id") REFERENCES "categories" ("id")
);"###;

			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 1);

			// FOREIGN KEY referencing the same table
			assert!(
				result[0].contains("REFERENCES \"categories\""),
				"Should self-reference categories table"
			);
		}

		#[test]
		fn test_split_create_index_statements() {
			// Splitting CREATE INDEX statements
			let sql = r###"CREATE TABLE "products" ("id" bigint PRIMARY KEY, "name" text);
CREATE INDEX "idx_products_name" ON "products" ("name");
CREATE UNIQUE INDEX "idx_products_id" ON "products" ("id");"###;

			let result = split_sql_statements(sql);
			assert_eq!(result.len(), 3);

			assert!(
				result[0].contains("CREATE TABLE"),
				"First statement should be CREATE TABLE"
			);
			assert!(
				result[1].contains("CREATE INDEX"),
				"Second statement should be CREATE INDEX"
			);
			assert!(
				result[2].contains("CREATE UNIQUE INDEX"),
				"Third statement should be CREATE UNIQUE INDEX"
			);
		}
	}
}
