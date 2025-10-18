//! Database audit backend
//!
//! This backend stores audit logs in a SQL database.

use crate::audit::{AccessEntry, AuditBackend, AuditEntry, AuditError, AuditResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{AnyPool, Row};

/// Database audit backend
///
/// Stores audit logs in a SQL database.
/// Supports PostgreSQL, MySQL, and SQLite.
pub struct DatabaseAuditBackend {
    pool: AnyPool,
}

impl DatabaseAuditBackend {
    /// Create a new database audit backend
    ///
    /// # Arguments
    ///
    /// * `database_url` - Database connection URL
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_settings::audit::backends::DatabaseAuditBackend;
    ///
    /// let backend = DatabaseAuditBackend::new("sqlite::memory:").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> AuditResult<Self> {
        let pool = AnyPool::connect(database_url)
            .await
            .map_err(|e| AuditError::Backend(format!("Database connection failed: {}", e)))?;

        let backend = Self { pool };
        backend.init_tables().await?;

        Ok(backend)
    }

    /// Initialize audit tables if they don't exist
    async fn init_tables(&self) -> AuditResult<()> {
        // Create audit_changes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                key TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT,
                changed_by TEXT NOT NULL,
                reason TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to create audit_changes table: {}", e)))?;

        // Create audit_accesses table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_accesses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                key TEXT NOT NULL,
                accessor TEXT NOT NULL,
                success INTEGER NOT NULL,
                context TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AuditError::Backend(format!("Failed to create audit_accesses table: {}", e))
        })?;

        // Create indexes for common queries
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_changes_key ON audit_changes(key)")
            .execute(&self.pool)
            .await;

        let _ =
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_changes_user ON audit_changes(changed_by)")
                .execute(&self.pool)
                .await;

        let _ = sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_changes_timestamp ON audit_changes(timestamp)",
        )
        .execute(&self.pool)
        .await;

        Ok(())
    }
}

#[async_trait]
impl AuditBackend for DatabaseAuditBackend {
    async fn log_change(&mut self, entry: AuditEntry) -> AuditResult<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_changes (timestamp, key, old_value, new_value, changed_by, reason)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(entry.timestamp.to_rfc3339())
        .bind(&entry.key)
        .bind(&entry.old_value)
        .bind(&entry.new_value)
        .bind(&entry.changed_by)
        .bind(&entry.reason)
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to log change: {}", e)))?;

        Ok(())
    }

    async fn log_access(&mut self, entry: AccessEntry) -> AuditResult<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_accesses (timestamp, key, accessor, success, context)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(entry.timestamp.to_rfc3339())
        .bind(&entry.key)
        .bind(&entry.accessor)
        .bind(entry.success as i32)
        .bind(&entry.context)
        .execute(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to log access: {}", e)))?;

        Ok(())
    }

    async fn get_recent_changes(&self, limit: usize) -> AuditResult<Vec<AuditEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT timestamp, key, old_value, new_value, changed_by, reason
            FROM audit_changes
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to fetch changes: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            let timestamp_str: String = row.try_get("timestamp")?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            entries.push(AuditEntry {
                timestamp,
                key: row.try_get("key")?,
                old_value: row.try_get("old_value")?,
                new_value: row.try_get("new_value")?,
                changed_by: row.try_get("changed_by")?,
                reason: row.try_get("reason")?,
            });
        }

        // Reverse to get chronological order
        entries.reverse();
        Ok(entries)
    }

    async fn get_recent_access(&self, limit: usize) -> AuditResult<Vec<AccessEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT timestamp, key, accessor, success, context
            FROM audit_accesses
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to fetch accesses: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            let timestamp_str: String = row.try_get("timestamp")?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let success_int: i32 = row.try_get("success")?;

            entries.push(AccessEntry {
                timestamp,
                key: row.try_get("key")?,
                accessor: row.try_get("accessor")?,
                success: success_int != 0,
                context: row.try_get("context")?,
            });
        }

        entries.reverse();
        Ok(entries)
    }

    async fn get_changes_for_key(&self, key: &str) -> AuditResult<Vec<AuditEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT timestamp, key, old_value, new_value, changed_by, reason
            FROM audit_changes
            WHERE key = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(key)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to fetch changes for key: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            let timestamp_str: String = row.try_get("timestamp")?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            entries.push(AuditEntry {
                timestamp,
                key: row.try_get("key")?,
                old_value: row.try_get("old_value")?,
                new_value: row.try_get("new_value")?,
                changed_by: row.try_get("changed_by")?,
                reason: row.try_get("reason")?,
            });
        }

        Ok(entries)
    }

    async fn get_changes_by_user(&self, user: &str) -> AuditResult<Vec<AuditEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT timestamp, key, old_value, new_value, changed_by, reason
            FROM audit_changes
            WHERE changed_by = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuditError::Backend(format!("Failed to fetch changes by user: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            let timestamp_str: String = row.try_get("timestamp")?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            entries.push(AuditEntry {
                timestamp,
                key: row.try_get("key")?,
                old_value: row.try_get("old_value")?,
                new_value: row.try_get("new_value")?,
                changed_by: row.try_get("changed_by")?,
                reason: row.try_get("reason")?,
            });
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_backend_init() {
        let backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        // Verify tables were created
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_changes")
            .fetch_one(&backend.pool)
            .await
            .unwrap();

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_database_backend_log_change() {
        let mut backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        let entry = AuditEntry {
            timestamp: Utc::now(),
            key: "test_key".to_string(),
            old_value: Some("old".to_string()),
            new_value: Some("new".to_string()),
            changed_by: "test_user".to_string(),
            reason: Some("test".to_string()),
        };

        backend.log_change(entry.clone()).await.unwrap();

        let changes = backend.get_recent_changes(10).await.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key, "test_key");
    }

    #[tokio::test]
    async fn test_database_backend_filter_by_key() {
        let mut backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        for i in 0..5 {
            let entry = AuditEntry {
                timestamp: Utc::now(),
                key: format!("key_{}", i % 2),
                old_value: None,
                new_value: Some(format!("value_{}", i)),
                changed_by: "user".to_string(),
                reason: None,
            };
            backend.log_change(entry).await.unwrap();
        }

        let key_0_changes = backend.get_changes_for_key("key_0").await.unwrap();
        assert_eq!(key_0_changes.len(), 3);

        let key_1_changes = backend.get_changes_for_key("key_1").await.unwrap();
        assert_eq!(key_1_changes.len(), 2);
    }

    #[tokio::test]
    async fn test_database_backend_filter_by_user() {
        let mut backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        for i in 0..5 {
            let entry = AuditEntry {
                timestamp: Utc::now(),
                key: format!("key_{}", i),
                old_value: None,
                new_value: Some(format!("value_{}", i)),
                changed_by: format!("user_{}", i % 2),
                reason: None,
            };
            backend.log_change(entry).await.unwrap();
        }

        let user_0_changes = backend.get_changes_by_user("user_0").await.unwrap();
        assert_eq!(user_0_changes.len(), 3);

        let user_1_changes = backend.get_changes_by_user("user_1").await.unwrap();
        assert_eq!(user_1_changes.len(), 2);
    }
}
