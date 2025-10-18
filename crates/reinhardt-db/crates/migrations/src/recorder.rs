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
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
    /// use backends::DatabaseConnection;
    ///
    /// let connection = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
    /// let recorder = DatabaseMigrationRecorder::new(connection);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Ensure the migration schema table exists
    ///
    /// Creates the `reinhardt_migrations` table if it doesn't exist.
    /// This follows Django's migration table schema.
    pub async fn ensure_schema_table(&self) -> crate::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS reinhardt_migrations (
                id SERIAL PRIMARY KEY,
                app VARCHAR(255) NOT NULL,
                name VARCHAR(255) NOT NULL,
                applied TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#;

        self.connection
            .execute(sql, vec![])
            .await
            .map_err(|e| crate::MigrationError::DatabaseError(e))?;

        Ok(())
    }

    /// Check if a migration has been applied
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
    /// use backends::DatabaseConnection;
    ///
    /// let connection = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
    /// let recorder = DatabaseMigrationRecorder::new(connection);
    /// recorder.ensure_schema_table().await?;
    ///
    /// let is_applied = recorder.is_applied("myapp", "0001_initial").await?;
    /// println!("Migration applied: {}", is_applied);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_applied(&self, app: &str, name: &str) -> crate::Result<bool> {
        let sql = "SELECT EXISTS(SELECT 1 FROM reinhardt_migrations WHERE app = $1 AND name = $2) as exists_flag";
        let params = vec![
            QueryValue::String(app.to_string()),
            QueryValue::String(name.to_string()),
        ];

        let rows = self
            .connection
            .fetch_all(sql, params)
            .await
            .map_err(|e| crate::MigrationError::DatabaseError(e))?;

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

    /// Record that a migration has been applied
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
    /// use backends::DatabaseConnection;
    ///
    /// let connection = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
    /// let recorder = DatabaseMigrationRecorder::new(connection);
    /// recorder.ensure_schema_table().await?;
    ///
    /// recorder.record_applied("myapp", "0001_initial").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn record_applied(&self, app: &str, name: &str) -> crate::Result<()> {
        let sql =
            "INSERT INTO reinhardt_migrations (app, name, applied) VALUES ($1, $2, CURRENT_TIMESTAMP)";
        let params = vec![
            QueryValue::String(app.to_string()),
            QueryValue::String(name.to_string()),
        ];

        self.connection
            .execute(sql, params)
            .await
            .map_err(|e| crate::MigrationError::DatabaseError(e))?;

        Ok(())
    }

    /// Get all applied migrations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_migrations::recorder::DatabaseMigrationRecorder;
    /// use backends::DatabaseConnection;
    ///
    /// let connection = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
    /// let recorder = DatabaseMigrationRecorder::new(connection);
    /// recorder.ensure_schema_table().await?;
    ///
    /// let migrations = recorder.get_applied_migrations().await?;
    /// for migration in migrations {
    ///     println!("{}.{} applied at {:?}", migration.app, migration.name, migration.applied);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_applied_migrations(&self) -> crate::Result<Vec<MigrationRecord>> {
        let sql = "SELECT app, name, applied FROM reinhardt_migrations ORDER BY applied";

        let rows = self
            .connection
            .fetch_all(sql, vec![])
            .await
            .map_err(|e| crate::MigrationError::DatabaseError(e))?;

        let mut records = Vec::new();
        for row in rows {
            let app: String = row
                .get("app")
                .map_err(|e| crate::MigrationError::DatabaseError(e))?;
            let name: String = row
                .get("name")
                .map_err(|e| crate::MigrationError::DatabaseError(e))?;

            // Parse timestamp from database
            let applied: DateTime<Utc> = row
                .get("applied")
                .map_err(|e| crate::MigrationError::DatabaseError(e))?;

            records.push(MigrationRecord { app, name, applied });
        }

        Ok(records)
    }

    /// Unapply a migration (remove from records)
    ///
    /// Used when rolling back migrations.
    pub async fn unapply(&self, app: &str, name: &str) -> crate::Result<()> {
        let sql = "DELETE FROM reinhardt_migrations WHERE app = $1 AND name = $2";
        let params = vec![
            QueryValue::String(app.to_string()),
            QueryValue::String(name.to_string()),
        ];

        self.connection
            .execute(sql, params)
            .await
            .map_err(|e| crate::MigrationError::DatabaseError(e))?;

        Ok(())
    }
}
