//! Audit logging for admin actions
//!
//! This module provides audit logging functionality for tracking all admin actions
//! for compliance and security purposes.

use crate::{AdminDatabase, AdminError, AdminResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_query::{Alias, Asterisk, Expr, ExprTrait, PostgresQueryBuilder, Query as SeaQuery, SelectStatement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

/// Type of action performed on a model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditAction {
    /// Create action
    Create,
    /// Update action
    Update,
    /// Delete action
    Delete,
    /// View action
    View,
    /// Bulk delete action
    BulkDelete,
    /// Export action
    Export,
    /// Import action
    Import,
}

impl AuditAction {
    /// Get action name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Create => "create",
            AuditAction::Update => "update",
            AuditAction::Delete => "delete",
            AuditAction::View => "view",
            AuditAction::BulkDelete => "bulk_delete",
            AuditAction::Export => "export",
            AuditAction::Import => "import",
        }
    }

    /// Parse action from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "create" => Some(AuditAction::Create),
            "update" => Some(AuditAction::Update),
            "delete" => Some(AuditAction::Delete),
            "view" => Some(AuditAction::View),
            "bulk_delete" => Some(AuditAction::BulkDelete),
            "export" => Some(AuditAction::Export),
            "import" => Some(AuditAction::Import),
            _ => None,
        }
    }
}

/// Audit log entry
///
/// Records a single admin action with all relevant metadata.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::audit::{AuditLog, AuditAction};
/// use chrono::Utc;
/// use serde_json::json;
///
/// let log = AuditLog::builder()
///     .user_id("admin_user".to_string())
///     .model_name("User".to_string())
///     .object_id("123".to_string())
///     .action(AuditAction::Update)
///     .changes(json!({"email": {"old": "old@example.com", "new": "new@example.com"}}))
///     .build();
///
/// assert_eq!(log.model_name(), "User");
/// assert_eq!(log.action(), AuditAction::Update);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// Audit log ID
    id: Option<i64>,
    /// User ID who performed the action
    user_id: String,
    /// Model name
    model_name: String,
    /// Object ID (primary key of the modified object)
    object_id: String,
    /// Action type
    action: AuditAction,
    /// Timestamp when the action occurred
    timestamp: DateTime<Utc>,
    /// JSON representation of changes (before/after values)
    changes: Option<serde_json::Value>,
    /// IP address of the user
    ip_address: Option<IpAddr>,
    /// User agent string
    user_agent: Option<String>,
}

impl AuditLog {
    /// Create a new audit log builder
    pub fn builder() -> AuditLogBuilder {
        AuditLogBuilder::new()
    }

    /// Get the audit log ID
    pub fn id(&self) -> Option<i64> {
        self.id
    }

    /// Get the user ID
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Get the object ID
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    /// Get the action type
    pub fn action(&self) -> AuditAction {
        self.action
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Get the changes
    pub fn changes(&self) -> Option<&serde_json::Value> {
        self.changes.as_ref()
    }

    /// Get the IP address
    pub fn ip_address(&self) -> Option<IpAddr> {
        self.ip_address
    }

    /// Get the user agent
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    /// Set the audit log ID (used after database insertion)
    pub fn set_id(&mut self, id: i64) {
        self.id = Some(id);
    }

    /// Set the changes
    pub fn set_changes(&mut self, changes: serde_json::Value) {
        self.changes = Some(changes);
    }

    /// Set the IP address
    pub fn set_ip_address(&mut self, ip_address: IpAddr) {
        self.ip_address = Some(ip_address);
    }

    /// Set the user agent
    pub fn set_user_agent(&mut self, user_agent: String) {
        self.user_agent = Some(user_agent);
    }
}

/// Builder for constructing audit logs
///
/// # Examples
///
/// ```
/// use reinhardt_admin::audit::{AuditLogBuilder, AuditAction};
/// use serde_json::json;
///
/// let log = AuditLogBuilder::new()
///     .user_id("admin_user".to_string())
///     .model_name("Article".to_string())
///     .object_id("456".to_string())
///     .action(AuditAction::Create)
///     .changes(json!({"title": "New Article", "status": "draft"}))
///     .build();
///
/// assert_eq!(log.user_id(), "admin_user");
/// ```
pub struct AuditLogBuilder {
    user_id: Option<String>,
    model_name: Option<String>,
    object_id: Option<String>,
    action: Option<AuditAction>,
    timestamp: Option<DateTime<Utc>>,
    changes: Option<serde_json::Value>,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
}

impl AuditLogBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            user_id: None,
            model_name: None,
            object_id: None,
            action: None,
            timestamp: None,
            changes: None,
            ip_address: None,
            user_agent: None,
        }
    }

    /// Set the user ID
    pub fn user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set the model name
    pub fn model_name(mut self, model_name: String) -> Self {
        self.model_name = Some(model_name);
        self
    }

    /// Set the object ID
    pub fn object_id(mut self, object_id: String) -> Self {
        self.object_id = Some(object_id);
        self
    }

    /// Set the action type
    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set the timestamp
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the changes
    pub fn changes(mut self, changes: serde_json::Value) -> Self {
        self.changes = Some(changes);
        self
    }

    /// Set the IP address
    pub fn ip_address(mut self, ip_address: IpAddr) -> Self {
        self.ip_address = Some(ip_address);
        self
    }

    /// Set the user agent
    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Build the audit log
    ///
    /// # Panics
    ///
    /// Panics if required fields (user_id, model_name, object_id, action) are not set.
    pub fn build(self) -> AuditLog {
        AuditLog {
            id: None,
            user_id: self.user_id.expect("user_id is required"),
            model_name: self.model_name.expect("model_name is required"),
            object_id: self.object_id.expect("object_id is required"),
            action: self.action.expect("action is required"),
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
            changes: self.changes,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
        }
    }
}

impl Default for AuditLogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for audit logging backends
///
/// Implement this trait to provide custom audit log storage backends.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::audit::{AuditLogger, AuditLog, AuditLogQuery};
/// use async_trait::async_trait;
///
/// struct CustomAuditLogger;
///
/// #[async_trait]
/// impl AuditLogger for CustomAuditLogger {
///     async fn log(&self, entry: AuditLog) -> Result<AuditLog, Box<dyn std::error::Error + Send + Sync>> {
///         // Custom logging implementation
///         Ok(entry)
///     }
///
///     async fn query(&self, query: &AuditLogQuery) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
///         // Custom query implementation
///         Ok(vec![])
///     }
///
///     async fn count(&self, query: &AuditLogQuery) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
///         Ok(0)
///     }
/// }
/// ```
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log an audit entry
    async fn log(
        &self,
        entry: AuditLog,
    ) -> Result<AuditLog, Box<dyn std::error::Error + Send + Sync>>;

    /// Query audit logs
    async fn query(
        &self,
        query: &AuditLogQuery,
    ) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>>;

    /// Count audit logs matching the query
    async fn count(
        &self,
        query: &AuditLogQuery,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>;
}

/// Query builder for audit logs
///
/// # Examples
///
/// ```
/// use reinhardt_admin::audit::{AuditLogQuery, AuditAction};
/// use chrono::{Utc, Duration};
///
/// let query = AuditLogQuery::builder()
///     .user_id("admin_user".to_string())
///     .model_name("User".to_string())
///     .action(AuditAction::Update)
///     .limit(100)
///     .build();
///
/// assert_eq!(query.limit(), 100);
/// ```
#[derive(Debug, Clone)]
pub struct AuditLogQuery {
    /// Filter by user ID
    user_id: Option<String>,
    /// Filter by model name
    model_name: Option<String>,
    /// Filter by object ID
    object_id: Option<String>,
    /// Filter by action type
    action: Option<AuditAction>,
    /// Filter by date range (start)
    start_date: Option<DateTime<Utc>>,
    /// Filter by date range (end)
    end_date: Option<DateTime<Utc>>,
    /// Maximum number of results
    limit: usize,
    /// Offset for pagination
    offset: usize,
}

impl AuditLogQuery {
    /// Create a new query builder
    pub fn builder() -> AuditLogQueryBuilder {
        AuditLogQueryBuilder::new()
    }

    /// Get the user ID filter
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Get the model name filter
    pub fn model_name(&self) -> Option<&str> {
        self.model_name.as_deref()
    }

    /// Get the object ID filter
    pub fn object_id(&self) -> Option<&str> {
        self.object_id.as_deref()
    }

    /// Get the action filter
    pub fn action(&self) -> Option<AuditAction> {
        self.action
    }

    /// Get the start date filter
    pub fn start_date(&self) -> Option<DateTime<Utc>> {
        self.start_date
    }

    /// Get the end date filter
    pub fn end_date(&self) -> Option<DateTime<Utc>> {
        self.end_date
    }

    /// Get the limit
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Get the offset
    pub fn offset(&self) -> usize {
        self.offset
    }
}

/// Builder for audit log queries
pub struct AuditLogQueryBuilder {
    user_id: Option<String>,
    model_name: Option<String>,
    object_id: Option<String>,
    action: Option<AuditAction>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    limit: usize,
    offset: usize,
}

impl AuditLogQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            user_id: None,
            model_name: None,
            object_id: None,
            action: None,
            start_date: None,
            end_date: None,
            limit: 100,
            offset: 0,
        }
    }

    /// Filter by user ID
    pub fn user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Filter by model name
    pub fn model_name(mut self, model_name: String) -> Self {
        self.model_name = Some(model_name);
        self
    }

    /// Filter by object ID
    pub fn object_id(mut self, object_id: String) -> Self {
        self.object_id = Some(object_id);
        self
    }

    /// Filter by action type
    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Filter by start date
    pub fn start_date(mut self, start_date: DateTime<Utc>) -> Self {
        self.start_date = Some(start_date);
        self
    }

    /// Filter by end date
    pub fn end_date(mut self, end_date: DateTime<Utc>) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Set the limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set the offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Build the query
    pub fn build(self) -> AuditLogQuery {
        AuditLogQuery {
            user_id: self.user_id,
            model_name: self.model_name,
            object_id: self.object_id,
            action: self.action,
            start_date: self.start_date,
            end_date: self.end_date,
            limit: self.limit,
            offset: self.offset,
        }
    }
}

impl Default for AuditLogQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory audit logger implementation
///
/// Stores audit logs in memory. Suitable for testing and development.
///
/// # Examples
///
/// ```
/// use reinhardt_admin::audit::{MemoryAuditLogger, AuditLogger, AuditLog, AuditAction, AuditLogQuery};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let logger = MemoryAuditLogger::new();
///
/// let log = AuditLog::builder()
///     .user_id("admin".to_string())
///     .model_name("User".to_string())
///     .object_id("1".to_string())
///     .action(AuditAction::Create)
///     .build();
///
/// logger.log(log).await?;
///
/// let query = AuditLogQuery::builder()
///     .user_id("admin".to_string())
///     .build();
///
/// let logs = logger.query(&query).await?;
/// assert_eq!(logs.len(), 1);
/// # Ok(())
/// # }
/// ```
pub struct MemoryAuditLogger {
    logs: Arc<parking_lot::Mutex<Vec<AuditLog>>>,
    next_id: Arc<parking_lot::Mutex<i64>>,
    // Indexes for fast lookups
    user_index: Arc<parking_lot::Mutex<HashMap<String, Vec<usize>>>>,
    model_index: Arc<parking_lot::Mutex<HashMap<String, Vec<usize>>>>,
    action_index: Arc<parking_lot::Mutex<HashMap<AuditAction, Vec<usize>>>>,
}

impl MemoryAuditLogger {
    /// Create a new in-memory audit logger
    pub fn new() -> Self {
        Self {
            logs: Arc::new(parking_lot::Mutex::new(Vec::new())),
            next_id: Arc::new(parking_lot::Mutex::new(1)),
            user_index: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            model_index: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            action_index: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }

    /// Clear all stored logs
    pub fn clear(&self) {
        self.logs.lock().clear();
        *self.next_id.lock() = 1;
        self.user_index.lock().clear();
        self.model_index.lock().clear();
        self.action_index.lock().clear();
    }

    /// Get the number of stored logs
    pub fn len(&self) -> usize {
        self.logs.lock().len()
    }

    /// Check if the logger is empty
    pub fn is_empty(&self) -> bool {
        self.logs.lock().is_empty()
    }
}

impl Default for MemoryAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditLogger for MemoryAuditLogger {
    async fn log(
        &self,
        mut entry: AuditLog,
    ) -> Result<AuditLog, Box<dyn std::error::Error + Send + Sync>> {
        let id = {
            let mut next_id = self.next_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        entry.set_id(id);

        // Get index position before pushing
        let index = {
            let mut logs = self.logs.lock();
            let idx = logs.len();
            logs.push(entry.clone());
            idx
        };

        // Update indexes for fast lookup
        self.user_index
            .lock()
            .entry(entry.user_id.clone())
            .or_insert_with(Vec::new)
            .push(index);

        self.model_index
            .lock()
            .entry(entry.model_name.clone())
            .or_insert_with(Vec::new)
            .push(index);

        self.action_index
            .lock()
            .entry(entry.action)
            .or_insert_with(Vec::new)
            .push(index);

        Ok(entry)
    }

    async fn query(
        &self,
        query: &AuditLogQuery,
    ) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        // Use indexes for fast lookup when specific filters are provided
        let candidate_indices: Option<Vec<usize>> = if let Some(ref user_id) = query.user_id {
            self.user_index.lock().get(user_id).cloned()
        } else if let Some(ref model_name) = query.model_name {
            self.model_index.lock().get(model_name).cloned()
        } else if let Some(action) = query.action {
            self.action_index.lock().get(&action).cloned()
        } else {
            None
        };

        let logs = self.logs.lock();

        // Use indexed lookup if available, otherwise scan all logs
        let mut results: Vec<AuditLog> = match candidate_indices {
            Some(indices) => indices
                .into_iter()
                .filter_map(|idx| logs.get(idx))
                .filter(|log| {
                    // Apply remaining filters
                    if let Some(ref user_id) = query.user_id {
                        if log.user_id != *user_id {
                            return false;
                        }
                    }

                    if let Some(ref model_name) = query.model_name {
                        if log.model_name != *model_name {
                            return false;
                        }
                    }

                    if let Some(ref object_id) = query.object_id {
                        if log.object_id != *object_id {
                            return false;
                        }
                    }

                    if let Some(action) = query.action {
                        if log.action != action {
                            return false;
                        }
                    }

                    if let Some(start_date) = query.start_date {
                        if log.timestamp < start_date {
                            return false;
                        }
                    }

                    if let Some(end_date) = query.end_date {
                        if log.timestamp > end_date {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect(),
            None => logs
                .iter()
                .filter(|log| {
                    // Apply all filters
                    if let Some(ref user_id) = query.user_id {
                        if log.user_id != *user_id {
                            return false;
                        }
                    }

                    if let Some(ref model_name) = query.model_name {
                        if log.model_name != *model_name {
                            return false;
                        }
                    }

                    if let Some(ref object_id) = query.object_id {
                        if log.object_id != *object_id {
                            return false;
                        }
                    }

                    if let Some(action) = query.action {
                        if log.action != action {
                            return false;
                        }
                    }

                    if let Some(start_date) = query.start_date {
                        if log.timestamp < start_date {
                            return false;
                        }
                    }

                    if let Some(end_date) = query.end_date {
                        if log.timestamp > end_date {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect(),
        };

        // Sort by timestamp descending (most recent first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let results: Vec<AuditLog> = results
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        Ok(results)
    }

    async fn count(
        &self,
        query: &AuditLogQuery,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let logs = self.logs.lock();
        let count = logs
            .iter()
            .filter(|log| {
                // Same filtering logic as query()
                if let Some(ref user_id) = query.user_id {
                    if log.user_id != *user_id {
                        return false;
                    }
                }

                if let Some(ref model_name) = query.model_name {
                    if log.model_name != *model_name {
                        return false;
                    }
                }

                if let Some(ref object_id) = query.object_id {
                    if log.object_id != *object_id {
                        return false;
                    }
                }

                if let Some(action) = query.action {
                    if log.action != action {
                        return false;
                    }
                }

                if let Some(start_date) = query.start_date {
                    if log.timestamp < start_date {
                        return false;
                    }
                }

                if let Some(end_date) = query.end_date {
                    if log.timestamp > end_date {
                        return false;
                    }
                }

                true
            })
            .count();

        Ok(count)
    }
}

/// Database-backed audit logger implementation
///
/// Stores audit logs in a database table. Suitable for production use.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_admin::audit::{DatabaseAuditLogger, AuditLogger};
/// use reinhardt_admin::AdminDatabase;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let db = Arc::new(AdminDatabase::new(/* connection */));
/// let logger = DatabaseAuditLogger::new(db, "audit_logs".to_string());
///
/// // Use the logger
/// # Ok(())
/// # }
/// ```
pub struct DatabaseAuditLogger {
    database: Arc<AdminDatabase>,
    table_name: String,
}

impl DatabaseAuditLogger {
    /// Create a new database audit logger
    pub fn new(database: Arc<AdminDatabase>, table_name: String) -> Self {
        Self {
            database,
            table_name,
        }
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

#[async_trait]
impl AuditLogger for DatabaseAuditLogger {
    async fn log(
        &self,
        mut entry: AuditLog,
    ) -> Result<AuditLog, Box<dyn std::error::Error + Send + Sync>> {
        use sea_query::{Alias, Asterisk, PostgresQueryBuilder, Query};

        // Build INSERT query
        let mut query = Query::insert()
            .into_table(Alias::new(&self.table_name))
            .columns([
                Alias::new("user_id"),
                Alias::new("model_name"),
                Alias::new("object_id"),
                Alias::new("action"),
                Alias::new("timestamp"),
                Alias::new("changes"),
                Alias::new("ip_address"),
                Alias::new("user_agent"),
            ])
            .to_owned();

        // Convert IP address to string if present
        let ip_string = entry.ip_address.map(|ip| ip.to_string());

        // Convert changes to JSON string if present
        let changes_json = entry.changes.as_ref().map(|c| c.to_string());

        // Add values - convert to Exprs for sea-query v1.0
        query.values_panic([
            sea_query::Value::String(Some(entry.user_id.clone())).into(),
            sea_query::Value::String(Some(entry.model_name.clone())).into(),
            sea_query::Value::String(Some(entry.object_id.clone())).into(),
            sea_query::Value::String(Some(entry.action.as_str().to_string())).into(),
            sea_query::Value::String(Some(entry.timestamp.to_rfc3339())).into(),
            match changes_json {
                Some(json) => sea_query::Value::String(Some(json)),
                None => sea_query::Value::String(None),
            }.into(),
            match ip_string {
                Some(ip) => sea_query::Value::String(Some(ip)),
                None => sea_query::Value::String(None),
            }.into(),
            match entry.user_agent.clone() {
                Some(ua) => sea_query::Value::String(Some(ua)),
                None => sea_query::Value::String(None),
            }.into(),
        ]);

        // Add RETURNING clause to get the inserted row
        query.returning(Query::returning().columns([Asterisk]));

        // Execute query
        let sql = query.to_string(PostgresQueryBuilder);
        let row = self
            .database
            .connection()
            .query_one(&sql)
            .await
            .map_err(|e| format!("{}", e))?;

        // Parse returned row data to extract ID
        // Note: In a real implementation, we would properly parse the row data
        // For now, we'll use a placeholder approach
        if let Some(id_value) = row.data.get("id") {
            if let Some(id) = id_value.as_i64() {
                entry.set_id(id);
            }
        }

        Ok(entry)
    }

    async fn query(
        &self,
        query: &AuditLogQuery,
    ) -> Result<Vec<AuditLog>, Box<dyn std::error::Error + Send + Sync>> {
        use sea_query::{Alias, Condition, Expr, PostgresQueryBuilder, Query as SeaQuery};

        // Build SELECT query
        let mut select = SeaQuery::select()
            .columns([
                Alias::new("id"),
                Alias::new("user_id"),
                Alias::new("model_name"),
                Alias::new("object_id"),
                Alias::new("action"),
                Alias::new("timestamp"),
                Alias::new("changes"),
                Alias::new("ip_address"),
                Alias::new("user_agent"),
            ])
            .from(Alias::new(&self.table_name))
            .to_owned();

        // Build WHERE conditions
        let mut condition = Condition::all();

        // Filter by user_id
        if let Some(ref user_id) = query.user_id {
            condition = condition.add(Expr::col(Alias::new("user_id")).eq(user_id.as_str()));
        }

        // Filter by model_name
        if let Some(ref model_name) = query.model_name {
            condition =
                condition.add(Expr::col(Alias::new("model_name")).eq(model_name.as_str()));
        }

        // Filter by object_id
        if let Some(ref object_id) = query.object_id {
            condition =
                condition.add(Expr::col(Alias::new("object_id")).eq(object_id.as_str()));
        }

        // Filter by action
        if let Some(action) = query.action {
            condition = condition.add(Expr::col(Alias::new("action")).eq(action.as_str()));
        }

        // Filter by start_date
        if let Some(start_date) = query.start_date {
            condition = condition
                .add(Expr::col(Alias::new("timestamp")).gte(start_date.to_rfc3339()));
        }

        // Filter by end_date
        if let Some(end_date) = query.end_date {
            condition =
                condition.add(Expr::col(Alias::new("timestamp")).lte(end_date.to_rfc3339()));
        }

        // Apply WHERE conditions
        select.cond_where(condition);

        // Add ORDER BY timestamp DESC
        select.order_by(Alias::new("timestamp"), sea_query::Order::Desc);

        // Add OFFSET
        if query.offset > 0 {
            select.offset(query.offset as u64);
        }

        // Add LIMIT
        select.limit(query.limit as u64);

        // Execute query
        let sql = select.to_string(PostgresQueryBuilder);
        let rows = self
            .database
            .connection()
            .query(&sql)
            .await
            .map_err(|e| format!("{}", e))?;

        // Parse rows into AuditLog instances
        let mut results = Vec::new();
        for row in rows {
            // Extract id
            let id = row
                .data
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or("Missing or invalid id field")?;

            // Extract user_id
            let user_id = row
                .data
                .get("user_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid user_id field")?
                .to_string();

            // Extract model_name
            let model_name = row
                .data
                .get("model_name")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid model_name field")?
                .to_string();

            // Extract object_id
            let object_id = row
                .data
                .get("object_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid object_id field")?
                .to_string();

            // Extract action
            let action_str = row
                .data
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid action field")?;
            let action =
                AuditAction::from_str(action_str).ok_or("Invalid action value in database")?;

            // Extract timestamp
            let timestamp_str = row
                .data
                .get("timestamp")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid timestamp field")?;
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                .map_err(|e| format!("Invalid timestamp format: {}", e))?
                .with_timezone(&Utc);

            // Extract changes (optional)
            let changes = row
                .data
                .get("changes")
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str(s).ok());

            // Extract ip_address (optional)
            let ip_address = row
                .data
                .get("ip_address")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<IpAddr>().ok());

            // Extract user_agent (optional)
            let user_agent = row
                .data
                .get("user_agent")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Build AuditLog
            let mut log = AuditLog::builder()
                .user_id(user_id)
                .model_name(model_name)
                .object_id(object_id)
                .action(action)
                .timestamp(timestamp)
                .build();

            log.set_id(id);

            if let Some(changes) = changes {
                log.set_changes(changes);
            }

            if let Some(ip_address) = ip_address {
                log.set_ip_address(ip_address);
            }

            if let Some(user_agent) = user_agent {
                log.set_user_agent(user_agent);
            }

            results.push(log);
        }

        Ok(results)
    }

    async fn count(
        &self,
        query: &AuditLogQuery,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        use sea_query::{Alias, Asterisk, Condition, Expr, PostgresQueryBuilder, Query};

        // Build SELECT COUNT(*) query
        let mut select = SeaQuery::select();
        select
            .expr(Expr::col(Asterisk).count())
            .from(Alias::new(&self.table_name));

        // Build WHERE conditions (same as query method)
        let mut condition = Condition::all();

        // Filter by user_id
        if let Some(ref user_id) = query.user_id {
            condition = condition.add(Expr::col(Alias::new("user_id")).eq(user_id.as_str()));
        }

        // Filter by model_name
        if let Some(ref model_name) = query.model_name {
            condition =
                condition.add(Expr::col(Alias::new("model_name")).eq(model_name.as_str()));
        }

        // Filter by object_id
        if let Some(ref object_id) = query.object_id {
            condition =
                condition.add(Expr::col(Alias::new("object_id")).eq(object_id.as_str()));
        }

        // Filter by action
        if let Some(action) = query.action {
            condition = condition.add(Expr::col(Alias::new("action")).eq(action.as_str()));
        }

        // Filter by start_date
        if let Some(start_date) = query.start_date {
            condition = condition
                .add(Expr::col(Alias::new("timestamp")).gte(start_date.to_rfc3339()));
        }

        // Filter by end_date
        if let Some(end_date) = query.end_date {
            condition =
                condition.add(Expr::col(Alias::new("timestamp")).lte(end_date.to_rfc3339()));
        }

        // Apply WHERE conditions
        select.cond_where(condition);

        // Execute query
        let sql = select.to_string(PostgresQueryBuilder);
        let row = self
            .database
            .connection()
            .query_one(&sql)
            .await
            .map_err(|e| format!("{}", e))?;

        // Parse count from result
        // The count value is typically in the first column or a "count" key
        let count = if let Some(count_value) = row.data.get("count") {
            count_value.as_i64().unwrap_or(0) as usize
        } else if let Some(obj) = row.data.as_object() {
            obj.values().next().and_then(|v| v.as_i64()).unwrap_or(0) as usize
        } else {
            0
        };

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_as_str() {
        assert_eq!(AuditAction::Create.as_str(), "create");
        assert_eq!(AuditAction::Update.as_str(), "update");
        assert_eq!(AuditAction::Delete.as_str(), "delete");
        assert_eq!(AuditAction::View.as_str(), "view");
        assert_eq!(AuditAction::BulkDelete.as_str(), "bulk_delete");
        assert_eq!(AuditAction::Export.as_str(), "export");
        assert_eq!(AuditAction::Import.as_str(), "import");
    }

    #[test]
    fn test_audit_action_from_str() {
        assert_eq!(AuditAction::from_str("create"), Some(AuditAction::Create));
        assert_eq!(AuditAction::from_str("update"), Some(AuditAction::Update));
        assert_eq!(AuditAction::from_str("delete"), Some(AuditAction::Delete));
        assert_eq!(AuditAction::from_str("view"), Some(AuditAction::View));
        assert_eq!(
            AuditAction::from_str("bulk_delete"),
            Some(AuditAction::BulkDelete)
        );
        assert_eq!(AuditAction::from_str("export"), Some(AuditAction::Export));
        assert_eq!(AuditAction::from_str("import"), Some(AuditAction::Import));
        assert_eq!(AuditAction::from_str("invalid"), None);
    }

    #[test]
    fn test_audit_log_builder() {
        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("123".to_string())
            .action(AuditAction::Create)
            .build();

        assert_eq!(log.user_id(), "admin");
        assert_eq!(log.model_name(), "User");
        assert_eq!(log.object_id(), "123");
        assert_eq!(log.action(), AuditAction::Create);
        assert!(log.id().is_none());
        assert!(log.changes().is_none());
        assert!(log.ip_address().is_none());
        assert!(log.user_agent().is_none());
    }

    #[test]
    fn test_audit_log_builder_with_changes() {
        let changes = serde_json::json!({
            "email": {
                "old": "old@example.com",
                "new": "new@example.com"
            }
        });

        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("123".to_string())
            .action(AuditAction::Update)
            .changes(changes.clone())
            .build();

        assert_eq!(log.action(), AuditAction::Update);
        assert_eq!(log.changes(), Some(&changes));
    }

    #[test]
    fn test_audit_log_builder_with_ip_and_user_agent() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let user_agent = "Mozilla/5.0".to_string();

        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("123".to_string())
            .action(AuditAction::View)
            .ip_address(ip)
            .user_agent(user_agent.clone())
            .build();

        assert_eq!(log.ip_address(), Some(ip));
        assert_eq!(log.user_agent(), Some(user_agent.as_str()));
    }

    #[test]
    fn test_audit_log_query_builder() {
        let query = AuditLogQuery::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .action(AuditAction::Update)
            .limit(50)
            .offset(10)
            .build();

        assert_eq!(query.user_id(), Some("admin"));
        assert_eq!(query.model_name(), Some("User"));
        assert_eq!(query.action(), Some(AuditAction::Update));
        assert_eq!(query.limit(), 50);
        assert_eq!(query.offset(), 10);
    }

    #[tokio::test]
    async fn test_memory_audit_logger_log() {
        let logger = MemoryAuditLogger::new();

        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("1".to_string())
            .action(AuditAction::Create)
            .build();

        let result = logger.log(log).await;
        assert!(result.is_ok());

        let logged = result.unwrap();
        assert_eq!(logged.id(), Some(1));
        assert_eq!(logger.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_audit_logger_query_by_user() {
        let logger = MemoryAuditLogger::new();

        // Log multiple entries
        for i in 1..=3 {
            let user_id = if i == 2 { "user2" } else { "user1" };
            let log = AuditLog::builder()
                .user_id(user_id.to_string())
                .model_name("User".to_string())
                .object_id(i.to_string())
                .action(AuditAction::Create)
                .build();
            logger.log(log).await.unwrap();
        }

        // Query by user1
        let query = AuditLogQuery::builder()
            .user_id("user1".to_string())
            .build();

        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|log| log.user_id() == "user1"));
    }

    #[tokio::test]
    async fn test_memory_audit_logger_query_by_model_and_action() {
        let logger = MemoryAuditLogger::new();

        // Log different actions and models
        let log1 = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("1".to_string())
            .action(AuditAction::Create)
            .build();

        let log2 = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("1".to_string())
            .action(AuditAction::Update)
            .build();

        let log3 = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("Article".to_string())
            .object_id("1".to_string())
            .action(AuditAction::Create)
            .build();

        logger.log(log1).await.unwrap();
        logger.log(log2).await.unwrap();
        logger.log(log3).await.unwrap();

        // Query User model with Create action
        let query = AuditLogQuery::builder()
            .model_name("User".to_string())
            .action(AuditAction::Create)
            .build();

        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].model_name(), "User");
        assert_eq!(results[0].action(), AuditAction::Create);
    }

    #[tokio::test]
    async fn test_memory_audit_logger_query_with_date_range() {
        let logger = MemoryAuditLogger::new();

        let now = Utc::now();
        let past = now - chrono::Duration::hours(2);
        let future = now + chrono::Duration::hours(2);

        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("1".to_string())
            .action(AuditAction::Create)
            .build();

        logger.log(log).await.unwrap();

        // Query with date range that includes the log
        let query = AuditLogQuery::builder()
            .start_date(past)
            .end_date(future)
            .build();

        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);

        // Query with date range that excludes the log
        let query = AuditLogQuery::builder()
            .start_date(future)
            .build();

        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_audit_logger_pagination() {
        let logger = MemoryAuditLogger::new();

        // Log 10 entries
        for i in 1..=10 {
            let log = AuditLog::builder()
                .user_id("admin".to_string())
                .model_name("User".to_string())
                .object_id(i.to_string())
                .action(AuditAction::Create)
                .build();
            logger.log(log).await.unwrap();
        }

        // Query first page
        let query = AuditLogQuery::builder().limit(5).offset(0).build();

        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 5);

        // Query second page
        let query = AuditLogQuery::builder().limit(5).offset(5).build();

        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_memory_audit_logger_count() {
        let logger = MemoryAuditLogger::new();

        // Log multiple entries
        for i in 1..=5 {
            let log = AuditLog::builder()
                .user_id("admin".to_string())
                .model_name("User".to_string())
                .object_id(i.to_string())
                .action(AuditAction::Create)
                .build();
            logger.log(log).await.unwrap();
        }

        // Count all
        let query = AuditLogQuery::builder().build();
        let count = logger.count(&query).await.unwrap();
        assert_eq!(count, 5);

        // Count with filter
        let query = AuditLogQuery::builder()
            .action(AuditAction::Create)
            .build();
        let count = logger.count(&query).await.unwrap();
        assert_eq!(count, 5);

        let query = AuditLogQuery::builder()
            .action(AuditAction::Update)
            .build();
        let count = logger.count(&query).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_memory_audit_logger_clear() {
        let logger = MemoryAuditLogger::new();

        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("1".to_string())
            .action(AuditAction::Create)
            .build();

        logger.log(log).await.unwrap();
        assert_eq!(logger.len(), 1);

        logger.clear();
        assert_eq!(logger.len(), 0);
        assert!(logger.is_empty());
    }

    #[test]
    fn test_database_audit_logger_creation() {
        // Create a dummy database for testing
        use reinhardt_orm::{DatabaseBackend, DatabaseConnection};

        let conn = DatabaseConnection::new(DatabaseBackend::Postgres);
        let db = Arc::new(AdminDatabase::new(Arc::new(conn)));
        let logger = DatabaseAuditLogger::new(db, "audit_logs".to_string());

        assert_eq!(logger.table_name(), "audit_logs");
    }

    #[tokio::test]
    async fn test_audit_log_with_changes() {
        let logger = MemoryAuditLogger::new();

        let changes = serde_json::json!({
            "username": {
                "old": "oldname",
                "new": "newname"
            },
            "email": {
                "old": "old@example.com",
                "new": "new@example.com"
            }
        });

        let log = AuditLog::builder()
            .user_id("admin".to_string())
            .model_name("User".to_string())
            .object_id("123".to_string())
            .action(AuditAction::Update)
            .changes(changes.clone())
            .build();

        let result = logger.log(log).await.unwrap();
        assert_eq!(result.changes(), Some(&changes));

        // Query and verify
        let query = AuditLogQuery::builder().build();
        let results = logger.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].changes(), Some(&changes));
    }
}
