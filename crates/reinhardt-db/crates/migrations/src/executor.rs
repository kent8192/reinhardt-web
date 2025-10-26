//! Migration executor
//!
//! Translated from Django's db/migrations/executor.py

use crate::{
    operations::SqlDialect, DatabaseMigrationRecorder, Migration, MigrationPlan, MigrationRecorder,
    Operation, Result,
};
use backends::{connection::DatabaseConnection, types::DatabaseType};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

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
    /// # }
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
    /// # }
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
    /// # }
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
    /// # }
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
    /// ```no_run
    /// use reinhardt_migrations::executor::DatabaseMigrationExecutor;
    /// use backends::{DatabaseConnection, DatabaseType};
    ///
    /// # async fn example() {
    /// let db = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await.unwrap();
    /// let executor = DatabaseMigrationExecutor::new(db.clone(), DatabaseType::Postgres);
    /// # }
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
    /// use backends::{DatabaseConnection, DatabaseType};
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
        let dialect = match self.db_type {
            DatabaseType::Postgres => SqlDialect::Postgres,
            DatabaseType::Sqlite => SqlDialect::Sqlite,
            DatabaseType::Mysql => SqlDialect::Mysql,
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
    /// ```no_run
    /// use reinhardt_migrations::{MigrationPlan, executor::DatabaseMigrationExecutor};
    /// use backends::{DatabaseConnection, DatabaseType};
    ///
    /// # async fn example() {
    /// let db = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await.unwrap();
    /// let mut executor = DatabaseMigrationExecutor::new(db, DatabaseType::Postgres);
    ///
    /// let plan = MigrationPlan::new();
    /// let result = executor.apply(&plan).await.unwrap();
    /// # }
    /// ```
    pub async fn apply(&mut self, plan: &MigrationPlan) -> Result<ExecutionResult> {
        let mut applied = Vec::new();

        // Ensure the migration recorder table exists
        self.recorder.ensure_schema_table().await?;

        let dialect = match self.db_type {
            DatabaseType::Postgres => SqlDialect::Postgres,
            DatabaseType::Sqlite => SqlDialect::Sqlite,
            DatabaseType::Mysql => SqlDialect::Mysql,
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
            for operation in &migration.operations {
                let sql = operation.to_sql(&dialect);
                self.connection.execute(&sql, vec![]).await?;
            }

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
        // 1. CreateTable
        // 2. AddColumn
        // 3. AlterColumn
        // 4. CreateIndex
        // 5. AddConstraint
        // 6. RunSQL
        // 7. RenameColumn
        // 8. DropColumn
        // 9. DropTable

        // First pass: Create tables
        let mut i = 0;
        while i < remaining.len() {
            if let Operation::CreateTable { name, .. } = &remaining[i] {
                created_tables.insert(name.clone());
                ordered.push(remaining.remove(i));
            } else {
                i += 1;
            }
        }

        // Second pass: Add columns (only for created tables)
        i = 0;
        while i < remaining.len() {
            if let Operation::AddColumn { table, .. } = &remaining[i] {
                if created_tables.contains(table) {
                    ordered.push(remaining.remove(i));
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // Third pass: Other operations
        ordered.extend(remaining);

        ordered
    }

    /// Group similar operations together
    fn group_similar_operations(&self, operations: Vec<Operation>) -> Vec<Operation> {
        let mut by_table: HashMap<String, Vec<Operation>> = HashMap::new();
        let mut other_ops = Vec::new();

        for op in operations {
            match &op {
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

        // Add table-specific operations grouped by table
        for (_, ops) in by_table {
            grouped.extend(ops);
        }

        // Add other operations
        grouped.extend(other_ops);

        grouped
    }

    /// Remove redundant operations
    fn remove_redundant_operations(&self, operations: Vec<Operation>) -> Vec<Operation> {
        let mut optimized = Vec::new();
        let mut seen_tables = HashSet::new();

        for operation in operations {
            match &operation {
                Operation::CreateTable { name, .. } => {
                    if !seen_tables.contains(name) {
                        seen_tables.insert(name.clone());
                        optimized.push(operation);
                    }
                }
                _ => {
                    optimized.push(operation);
                }
            }
        }

        optimized
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
