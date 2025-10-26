//! Migration executor
//!
//! Translated from Django's db/migrations/executor.py

use crate::{
    DatabaseMigrationRecorder, Migration, MigrationPlan, MigrationRecorder, Result,
    operations::SqlDialect,
};
use backends::{connection::DatabaseConnection, types::DatabaseType};
use sqlx::SqlitePool;

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
