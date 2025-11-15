//! Migration executor
//!
//! Translated from Django's db/migrations/executor.py

use crate::{
	DatabaseMigrationRecorder, Migration, MigrationPlan, MigrationRecorder, Operation, Result,
	operations::SqlDialect,
};
use indexmap::IndexMap;
use reinhardt_backends::{connection::DatabaseConnection, types::DatabaseType};
use sqlx::SqlitePool;
use std::collections::HashSet;

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
				.is_applied_async(&self.pool, &migration.app_label, &migration.name)
				.await?;

			if is_applied {
				continue;
			}

			// Apply migration operations
			self.apply_migration(migration).await?;

			// Record migration as applied
			self.recorder
				.record_applied_async(
					&self.pool,
					migration.app_label.clone(),
					migration.name.clone(),
				)
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
		// For now, we apply operations directly
		for operation in &migration.operations {
			let sql = operation.to_sql(&SqlDialect::Sqlite);
			sqlx::query(&sql).execute(&self.pool).await?;
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
				.is_applied_async(&self.pool, &migration.app_label, &migration.name)
				.await?;

			if is_applied {
				continue;
			}

			// Apply migration
			for operation in &migration.operations {
				let sql = operation.to_sql(&SqlDialect::Sqlite);
				sqlx::query(&sql).execute(&self.pool).await?;
			}

			// Record migration
			self.recorder
				.record_applied_async(
					&self.pool,
					migration.app_label.clone(),
					migration.name.clone(),
				)
				.await?;
			applied.push(migration.id());
		}

		Ok(ExecutionResult {
			applied,
			failed: None,
		})
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
				.is_applied(&migration.app_label, &migration.name)
				.await?
			{
				continue;
			}

			// Apply migration operations
			self.apply_migration(migration).await?;

			// Record migration as applied
			self.recorder
				.record_applied(&migration.app_label, &migration.name)
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
			let sql = operation.to_sql(&dialect);
			self.connection.execute(&sql, vec![]).await?;
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
				.is_applied(&migration.app_label, &migration.name)
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
					let sql = operation.to_sql(&dialect);
					self.connection.execute(&sql, vec![]).await?;
				}
			}
			// For MongoDB, skip SQL operations (schemaless)

			// Record migration as applied
			self.recorder
				.record_applied(&migration.app_label, &migration.name)
				.await?;

			applied.push(migration.id());
		}

		Ok(ExecutionResult {
			applied,
			failed: None,
		})
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
/// use reinhardt_migrations::{Operation, ColumnDefinition};
///
/// let ops = vec![
///     Operation::AddColumn {
///         table: "users".to_string(),
///         column: ColumnDefinition::new("name", "VARCHAR(100)"),
///     },
///     Operation::CreateTable {
///         name: "users".to_string(),
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
	///         name: "users".to_string(),
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
							if !created_tables.contains(&referenced_table)
								&& referenced_table != *name
							{
								depends_on_uncreated = true;
								break;
							}
						}
					}

					// If this table doesn't depend on any uncreated table, we can create it now
					if !depends_on_uncreated {
						created_tables.insert(name.clone());
						ordered.push(create_table_ops.remove(i));
						found_independent = true;
						break;
					}
				}
			}

			// If we couldn't find any independent table, just add the remaining tables
			// (this handles circular dependencies or malformed constraints)
			if !found_independent {
				for op in create_table_ops.drain(..) {
					if let Operation::CreateTable { name, .. } = &op {
						created_tables.insert(name.clone());
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

	/// Extract the referenced table name from a FOREIGN KEY constraint string
	/// Example: "FOREIGN KEY (table1_id) REFERENCES table1(id)" -> Some("table1")
	fn extract_foreign_key_reference(&self, constraint: &str) -> Option<String> {
		// Simple regex-like parsing for "REFERENCES table_name"
		const REFERENCES_KEYWORD: &str = "REFERENCES";
		let constraint_upper = constraint.to_uppercase();

		if let Some(references_pos) = constraint_upper.find(REFERENCES_KEYWORD) {
			// Extract substring after "REFERENCES" keyword
			let start_pos = references_pos + REFERENCES_KEYWORD.len();
			let after_references = constraint[start_pos..].trim_start();

			// Extract table name (everything before '(' or whitespace)
			let table_name = after_references
				.split(|c: char| c == '(' || c.is_whitespace())
				.next()
				.unwrap_or("")
				.trim();

			if !table_name.is_empty() {
				return Some(table_name.to_string());
			}
		}
		None
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
					by_table.entry(table.clone()).or_default().push(op);
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
					create_table_map.insert(name.clone(), operation.clone());
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
					let key = (table.clone(), column.clone());
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
		let mut rename_chain: IndexMap<String, String> = IndexMap::new(); // original_name -> current_name

		for operation in merged {
			match &operation {
				Operation::RenameTable { old_name, new_name } => {
					// Find if any existing chain ends with this old_name
					let mut found_chain = None;
					for (original, current) in &rename_chain {
						if current == old_name {
							found_chain = Some(original.clone());
							break;
						}
					}

					if let Some(original) = found_chain {
						// Extend existing chain: original -> new_name
						rename_chain.insert(original, new_name.clone());
					} else {
						// Start new chain: old_name -> new_name
						rename_chain.insert(old_name.clone(), new_name.clone());
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
	use crate::ColumnDefinition;

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
				table: "users".to_string(),
				column: ColumnDefinition::new("name", "VARCHAR(100)"),
			},
			Operation::CreateTable {
				name: "users".to_string(),
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
				name: "users".to_string(),
				columns: vec![],
				constraints: vec![],
			},
			Operation::CreateTable {
				name: "users".to_string(),
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
				table: "users".to_string(),
				column: ColumnDefinition::new("name", "VARCHAR(100)"),
			},
			Operation::CreateTable {
				name: "posts".to_string(),
				columns: vec![],
				constraints: vec![],
			},
			Operation::AddColumn {
				table: "users".to_string(),
				column: ColumnDefinition::new("email", "VARCHAR(255)"),
			},
		];

		let optimized = optimizer.optimize(ops);
		assert_eq!(optimized.len(), 3);
	}
}
