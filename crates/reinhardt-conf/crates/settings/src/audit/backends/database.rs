//! Database audit backend
//!
//! This backend stores audit logs in a SQL database.

use crate::audit::{AuditBackend, AuditEvent, ChangeRecord, EventFilter, EventType};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{AnyPool, Row};
use std::collections::HashMap;

#[cfg(feature = "dynamic-database")]
use sea_query::{
    Alias, ColumnDef, Expr, ExprTrait, Index, Order, Query, SqliteQueryBuilder, Table,
};

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
    pub async fn new(database_url: &str) -> Result<Self, String> {
        let pool = AnyPool::connect(database_url)
            .await
            .map_err(|e| format!("Database connection failed: {}", e))?;

        let backend = Self { pool };
        backend.init_tables().await?;

        Ok(backend)
    }

    /// Initialize audit tables if they don't exist
    async fn init_tables(&self) -> Result<(), String> {
        // Create audit_events table
        let stmt = Table::create()
            .table(Alias::new("audit_events"))
            .if_not_exists()
            .col(
                ColumnDef::new(Alias::new("id"))
                    .integer()
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(ColumnDef::new(Alias::new("timestamp")).text().not_null())
            .col(ColumnDef::new(Alias::new("event_type")).text().not_null())
            .col(ColumnDef::new(Alias::new("user")).text())
            .col(ColumnDef::new(Alias::new("changes")).text().not_null())
            .to_owned();
        let sql = stmt.to_string(SqliteQueryBuilder);
        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to create audit_events table: {}", e))?;

        // Create indexes for common queries
        let idx = Index::create()
            .if_not_exists()
            .name("idx_events_timestamp")
            .table(Alias::new("audit_events"))
            .col(Alias::new("timestamp"))
            .to_owned();
        let _ = sqlx::query(&idx.to_string(SqliteQueryBuilder))
            .execute(&self.pool)
            .await;

        let idx = Index::create()
            .if_not_exists()
            .name("idx_events_type")
            .table(Alias::new("audit_events"))
            .col(Alias::new("event_type"))
            .to_owned();
        let _ = sqlx::query(&idx.to_string(SqliteQueryBuilder))
            .execute(&self.pool)
            .await;

        let idx = Index::create()
            .if_not_exists()
            .name("idx_events_user")
            .table(Alias::new("audit_events"))
            .col(Alias::new("user"))
            .to_owned();
        let _ = sqlx::query(&idx.to_string(SqliteQueryBuilder))
            .execute(&self.pool)
            .await;

        Ok(())
    }
}

#[async_trait]
impl AuditBackend for DatabaseAuditBackend {
    async fn log_event(&self, event: AuditEvent) -> Result<(), String> {
        // Serialize changes to JSON
        let changes_json = serde_json::to_string(&event.changes)
            .map_err(|e| format!("Failed to serialize changes: {}", e))?;

        let stmt = Query::insert()
            .into_table(Alias::new("audit_events"))
            .columns([
                Alias::new("timestamp"),
                Alias::new("event_type"),
                Alias::new("user"),
                Alias::new("changes"),
            ])
            .values(
                [
                    Expr::val(event.timestamp.to_rfc3339()),
                    Expr::val(event.event_type.as_str()),
                    Expr::val(event.user.unwrap_or_default()),
                    Expr::val(changes_json),
                ]
                .into_iter()
                .collect::<Vec<Expr>>(),
            )
            .unwrap()
            .to_owned();
        let sql = stmt.to_string(SqliteQueryBuilder);

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to log event: {}", e))?;

        Ok(())
    }

    async fn get_events(&self, filter: Option<EventFilter>) -> Result<Vec<AuditEvent>, String> {
        let mut query = Query::select()
            .columns([
                Alias::new("timestamp"),
                Alias::new("event_type"),
                Alias::new("user"),
                Alias::new("changes"),
            ])
            .from(Alias::new("audit_events"))
            .to_owned();

        // Apply filters
        if let Some(ref f) = filter {
            if let Some(ref event_type) = f.event_type {
                query.and_where(Expr::col(Alias::new("event_type")).eq(event_type.as_str()));
            }
            if let Some(ref user) = f.user {
                query.and_where(Expr::col(Alias::new("user")).eq(user.as_str()));
            }
            if let Some(start_time) = f.start_time {
                query.and_where(Expr::col(Alias::new("timestamp")).gte(start_time.to_rfc3339()));
            }
            if let Some(end_time) = f.end_time {
                query.and_where(Expr::col(Alias::new("timestamp")).lte(end_time.to_rfc3339()));
            }
        }

        query.order_by(Alias::new("timestamp"), Order::Desc);

        let sql = query.to_string(SqliteQueryBuilder);

        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to fetch events: {}", e))?;

        let mut events = Vec::new();
        for row in rows {
            let timestamp_str: String = row
                .try_get("timestamp")
                .map_err(|e| format!("Failed to get timestamp: {}", e))?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let event_type_str: String = row
                .try_get("event_type")
                .map_err(|e| format!("Failed to get event_type: {}", e))?;
            let event_type = match event_type_str.as_str() {
                "config_update" => EventType::ConfigUpdate,
                "config_delete" => EventType::ConfigDelete,
                "config_create" => EventType::ConfigCreate,
                "secret_access" => EventType::SecretAccess,
                "secret_rotation" => EventType::SecretRotation,
                _ => EventType::ConfigUpdate, // default
            };

            let user_str: String = row
                .try_get("user")
                .map_err(|e| format!("Failed to get user: {}", e))?;
            let user = if user_str.is_empty() {
                None
            } else {
                Some(user_str)
            };

            let changes_json: String = row
                .try_get("changes")
                .map_err(|e| format!("Failed to get changes: {}", e))?;
            let changes: HashMap<String, ChangeRecord> = serde_json::from_str(&changes_json)
                .map_err(|e| format!("Failed to deserialize changes: {}", e))?;

            events.push(AuditEvent {
                timestamp,
                event_type,
                user,
                changes,
            });
        }

        // Reverse to get chronological order
        events.reverse();
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_database_backend_init() {
        let backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        // Verify tables were created
        let stmt = Query::select()
            .expr(Expr::col(Alias::new("*")).count())
            .from(Alias::new("audit_events"))
            .to_owned();
        let sql = stmt.to_string(SqliteQueryBuilder);

        let row = sqlx::query(&sql).fetch_one(&backend.pool).await.unwrap();
        let count: i64 = row.try_get(0).unwrap();

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_database_backend_log_event() {
        let backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        let mut changes = HashMap::new();
        changes.insert(
            "test_key".to_string(),
            ChangeRecord {
                old_value: Some(json!("old")),
                new_value: Some(json!("new")),
            },
        );

        let event = AuditEvent::new(
            EventType::ConfigUpdate,
            Some("test_user".to_string()),
            changes,
        );

        backend.log_event(event).await.unwrap();

        let events = backend.get_events(None).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::ConfigUpdate);
    }

    #[tokio::test]
    async fn test_database_backend_filter_by_type() {
        let backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        for i in 0..5 {
            let event_type = if i % 2 == 0 {
                EventType::ConfigUpdate
            } else {
                EventType::ConfigCreate
            };

            let mut changes = HashMap::new();
            changes.insert(
                format!("key_{}", i),
                ChangeRecord {
                    old_value: None,
                    new_value: Some(json!(format!("value_{}", i))),
                },
            );

            let event = AuditEvent::new(event_type, Some("user".to_string()), changes);
            backend.log_event(event).await.unwrap();
        }

        let filter = EventFilter {
            event_type: Some(EventType::ConfigUpdate),
            user: None,
            start_time: None,
            end_time: None,
        };

        let update_events = backend.get_events(Some(filter)).await.unwrap();
        assert_eq!(update_events.len(), 3);
    }

    #[tokio::test]
    async fn test_database_backend_filter_by_user() {
        let backend = DatabaseAuditBackend::new("sqlite::memory:").await.unwrap();

        for i in 0..5 {
            let user = if i % 2 == 0 { "alice" } else { "bob" };

            let mut changes = HashMap::new();
            changes.insert(
                format!("key_{}", i),
                ChangeRecord {
                    old_value: None,
                    new_value: Some(json!(i)),
                },
            );

            let event = AuditEvent::new(EventType::ConfigUpdate, Some(user.to_string()), changes);
            backend.log_event(event).await.unwrap();
        }

        let filter = EventFilter {
            event_type: None,
            user: Some("alice".to_string()),
            start_time: None,
            end_time: None,
        };

        let alice_events = backend.get_events(Some(filter)).await.unwrap();
        assert_eq!(alice_events.len(), 3);
    }
}
