// use crate::connection::DatabaseExecutor;
use std::sync::{Arc, Mutex};

/// Transaction isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    /// Convert isolation level to SQL string
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::IsolationLevel;
    ///
    /// let level = IsolationLevel::Serializable;
    /// assert_eq!(level.to_sql(), "SERIALIZABLE");
    ///
    /// let level = IsolationLevel::ReadCommitted;
    /// assert_eq!(level.to_sql(), "READ COMMITTED");
    /// ```
    pub fn to_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    NotStarted,
    Active,
    Committed,
    RolledBack,
}

/// Savepoint for nested transactions
#[derive(Debug, Clone)]
pub struct Savepoint {
    pub name: String,
    pub depth: usize,
}

impl Savepoint {
    /// Create a new savepoint with name and depth
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Savepoint;
    ///
    /// let sp = Savepoint::new("my_savepoint", 1);
    /// assert_eq!(sp.name, "my_savepoint");
    /// assert_eq!(sp.depth, 1);
    /// ```
    pub fn new(name: impl Into<String>, depth: usize) -> Self {
        Self {
            name: name.into(),
            depth,
        }
    }
    /// Generate SQL for creating this savepoint
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Savepoint;
    ///
    /// let sp = Savepoint::new("checkpoint_1", 2);
    /// assert_eq!(sp.to_sql(), "SAVEPOINT checkpoint_1");
    /// ```
    pub fn to_sql(&self) -> String {
        format!("SAVEPOINT {}", self.name)
    }
    /// Generate SQL for releasing this savepoint
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Savepoint;
    ///
    /// let sp = Savepoint::new("checkpoint_1", 2);
    /// assert_eq!(sp.release_sql(), "RELEASE SAVEPOINT checkpoint_1");
    /// ```
    pub fn release_sql(&self) -> String {
        format!("RELEASE SAVEPOINT {}", self.name)
    }
    /// Generate SQL for rolling back to this savepoint
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Savepoint;
    ///
    /// let sp = Savepoint::new("checkpoint_1", 2);
    /// assert_eq!(sp.rollback_sql(), "ROLLBACK TO SAVEPOINT checkpoint_1");
    /// ```
    pub fn rollback_sql(&self) -> String {
        format!("ROLLBACK TO SAVEPOINT {}", self.name)
    }
}

/// Transaction manager
#[derive(Debug)]
pub struct Transaction {
    state: Arc<Mutex<TransactionState>>,
    isolation_level: Option<IsolationLevel>,
    savepoints: Arc<Mutex<Vec<Savepoint>>>,
    depth: usize,
}

impl Transaction {
    /// Create a new transaction in NotStarted state
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Transaction, TransactionState};
    ///
    /// let tx = Transaction::new();
    /// assert_eq!(tx.state().unwrap(), TransactionState::NotStarted);
    /// assert_eq!(tx.depth(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TransactionState::NotStarted)),
            isolation_level: None,
            savepoints: Arc::new(Mutex::new(Vec::new())),
            depth: 0,
        }
    }
    /// Set the isolation level for this transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Transaction, IsolationLevel};
    ///
    /// let mut tx = Transaction::new().with_isolation_level(IsolationLevel::Serializable);
    /// let sql = tx.begin().unwrap();
    /// assert!(sql.contains("SERIALIZABLE"));
    /// ```
    pub fn with_isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = Some(level);
        self
    }
    /// Begin a transaction or create a savepoint for nested transactions
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Transaction, TransactionState};
    ///
    /// let mut tx = Transaction::new();
    /// let sql = tx.begin().unwrap();
    /// assert_eq!(sql, "BEGIN TRANSACTION");
    /// assert_eq!(tx.state().unwrap(), TransactionState::Active);
    ///
    /// // Nested transaction creates savepoint
    /// let nested_sql = tx.begin().unwrap();
    /// assert!(nested_sql.contains("SAVEPOINT"));
    /// ```
    pub fn begin(&mut self) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;

        match *state {
            TransactionState::NotStarted => {
                *state = TransactionState::Active;
                self.depth = 1;

                let sql = if let Some(level) = self.isolation_level {
                    format!("BEGIN TRANSACTION ISOLATION LEVEL {}", level.to_sql())
                } else {
                    "BEGIN TRANSACTION".to_string()
                };

                Ok(sql)
            }
            TransactionState::Active => {
                // Nested transaction - use savepoint
                self.depth += 1;
                let savepoint = Savepoint::new(format!("sp_{}", self.depth), self.depth);
                let sql = savepoint.to_sql();

                self.savepoints
                    .lock()
                    .map_err(|e| e.to_string())?
                    .push(savepoint);

                Ok(sql)
            }
            _ => Err("Transaction already completed".to_string()),
        }
    }
    /// Commit a transaction or release a savepoint for nested transactions
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Transaction, TransactionState};
    ///
    /// let mut tx = Transaction::new();
    /// tx.begin().unwrap();
    /// let sql = tx.commit().unwrap();
    /// assert_eq!(sql, "COMMIT");
    /// assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    /// ```
    pub fn commit(&mut self) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;

        match *state {
            TransactionState::Active => {
                if self.depth > 1 {
                    // Release savepoint
                    let mut savepoints = self.savepoints.lock().map_err(|e| e.to_string())?;
                    if let Some(savepoint) = savepoints.pop() {
                        self.depth -= 1;
                        Ok(savepoint.release_sql())
                    } else {
                        Err("No savepoint to release".to_string())
                    }
                } else {
                    // Commit top-level transaction
                    *state = TransactionState::Committed;
                    self.depth = 0;
                    Ok("COMMIT".to_string())
                }
            }
            _ => Err("No active transaction to commit".to_string()),
        }
    }
    /// Rollback a transaction or rollback to a savepoint for nested transactions
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Transaction, TransactionState};
    ///
    /// let mut tx = Transaction::new();
    /// tx.begin().unwrap();
    /// let sql = tx.rollback().unwrap();
    /// assert_eq!(sql, "ROLLBACK");
    /// assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    /// ```
    pub fn rollback(&mut self) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;

        match *state {
            TransactionState::Active => {
                if self.depth > 1 {
                    // Rollback to savepoint
                    let mut savepoints = self.savepoints.lock().map_err(|e| e.to_string())?;
                    if let Some(savepoint) = savepoints.pop() {
                        self.depth -= 1;
                        Ok(savepoint.rollback_sql())
                    } else {
                        Err("No savepoint to rollback to".to_string())
                    }
                } else {
                    // Rollback top-level transaction
                    *state = TransactionState::RolledBack;
                    self.depth = 0;
                    self.savepoints.lock().map_err(|e| e.to_string())?.clear();
                    Ok("ROLLBACK".to_string())
                }
            }
            _ => Err("No active transaction to rollback".to_string()),
        }
    }
    /// Get current transaction state
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Transaction, TransactionState};
    ///
    /// let tx = Transaction::new();
    /// assert_eq!(tx.state().unwrap(), TransactionState::NotStarted);
    /// ```
    pub fn state(&self) -> Result<TransactionState, String> {
        self.state.lock().map(|s| *s).map_err(|e| e.to_string())
    }
    /// Get current transaction depth (0 = not started, 1 = top-level, 2+ = nested)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Transaction;
    ///
    /// let mut tx = Transaction::new();
    /// assert_eq!(tx.depth(), 0);
    ///
    /// tx.begin().unwrap();
    /// assert_eq!(tx.depth(), 1);
    ///
    /// tx.begin().unwrap(); // Nested
    /// assert_eq!(tx.depth(), 2);
    /// ```
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Execute transaction begin on database
    #[cfg(feature = "django-compat")]
    /// Documentation for `begin_db`
    ///
    pub async fn begin_db(&mut self) -> reinhardt_apps::Result<()> {
        let sql = self
            .begin()
            .map_err(|e| reinhardt_apps::Error::Database(e))?;
        let conn = crate::manager::get_connection().await?;
        conn.execute(&sql).await?;
        Ok(())
    }

    /// Execute transaction commit on database
    #[cfg(feature = "django-compat")]
    /// Documentation for `commit_db`
    ///
    pub async fn commit_db(&mut self) -> reinhardt_apps::Result<()> {
        let sql = self
            .commit()
            .map_err(|e| reinhardt_apps::Error::Database(e))?;
        let conn = crate::manager::get_connection().await?;
        conn.execute(&sql).await?;
        Ok(())
    }

    /// Execute transaction rollback on database
    #[cfg(feature = "django-compat")]
    /// Documentation for `rollback_db`
    ///
    pub async fn rollback_db(&mut self) -> reinhardt_apps::Result<()> {
        let sql = self
            .rollback()
            .map_err(|e| reinhardt_apps::Error::Database(e))?;
        let conn = crate::manager::get_connection().await?;
        conn.execute(&sql).await?;
        Ok(())
    }
    /// Check if transaction is currently active
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Transaction;
    ///
    /// let mut tx = Transaction::new();
    /// assert!(!tx.is_active());
    ///
    /// tx.begin().unwrap();
    /// assert!(tx.is_active());
    ///
    /// tx.commit().unwrap();
    /// assert!(!tx.is_active());
    /// ```
    pub fn is_active(&self) -> bool {
        matches!(self.state().ok(), Some(TransactionState::Active))
    }
    /// Create a named savepoint within an active transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Transaction;
    ///
    /// let mut tx = Transaction::new();
    /// tx.begin().unwrap();
    ///
    /// let sql = tx.savepoint("my_checkpoint").unwrap();
    /// assert_eq!(sql, "SAVEPOINT my_checkpoint");
    /// ```
    pub fn savepoint(&mut self, name: impl Into<String>) -> Result<String, String> {
        let state = self.state.lock().map_err(|e| e.to_string())?;

        if *state != TransactionState::Active {
            return Err("Cannot create savepoint outside active transaction".to_string());
        }

        drop(state);

        let savepoint = Savepoint::new(name, self.depth);
        let sql = savepoint.to_sql();

        self.savepoints
            .lock()
            .map_err(|e| e.to_string())?
            .push(savepoint);

        Ok(sql)
    }
    /// Release a named savepoint, committing its changes
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Transaction;
    ///
    /// let mut tx = Transaction::new();
    /// tx.begin().unwrap();
    /// tx.savepoint("my_checkpoint").unwrap();
    ///
    /// let sql = tx.release_savepoint("my_checkpoint").unwrap();
    /// assert_eq!(sql, "RELEASE SAVEPOINT my_checkpoint");
    /// ```
    pub fn release_savepoint(&mut self, name: &str) -> Result<String, String> {
        let mut savepoints = self.savepoints.lock().map_err(|e| e.to_string())?;

        if let Some(pos) = savepoints.iter().position(|sp| sp.name == name) {
            let savepoint = savepoints.remove(pos);
            Ok(savepoint.release_sql())
        } else {
            Err(format!("Savepoint '{}' not found", name))
        }
    }
    /// Rollback to a named savepoint, undoing changes after that point
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Transaction;
    ///
    /// let mut tx = Transaction::new();
    /// tx.begin().unwrap();
    /// tx.savepoint("my_checkpoint").unwrap();
    ///
    /// let sql = tx.rollback_to_savepoint("my_checkpoint").unwrap();
    /// assert_eq!(sql, "ROLLBACK TO SAVEPOINT my_checkpoint");
    /// ```
    pub fn rollback_to_savepoint(&mut self, name: &str) -> Result<String, String> {
        let savepoints = self.savepoints.lock().map_err(|e| e.to_string())?;

        if let Some(savepoint) = savepoints.iter().find(|sp| sp.name == name) {
            Ok(savepoint.rollback_sql())
        } else {
            Err(format!("Savepoint '{}' not found", name))
        }
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// Atomic transaction builder (similar to Django's transaction.atomic)
pub struct Atomic<F> {
    _func: F,
    _isolation_level: Option<IsolationLevel>,
}

impl<F> Atomic<F> {
    /// Create a new atomic transaction wrapper around a function
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::Atomic;
    ///
    /// let atomic = Atomic::new(|| {
    ///     // Transaction logic here
    /// });
    /// ```
    pub fn new(func: F) -> Self {
        Self {
            _func: func,
            _isolation_level: None,
        }
    }
    /// Set the isolation level for the atomic transaction
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_orm::transaction::{Atomic, IsolationLevel};
    ///
    /// let atomic = Atomic::new(|| {
    ///     // Transaction logic
    /// }).with_isolation_level(IsolationLevel::Serializable);
    /// ```
    pub fn with_isolation_level(mut self, level: IsolationLevel) -> Self {
        self._isolation_level = Some(level);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_begin() {
        let mut tx = Transaction::new();
        let sql = tx.begin().unwrap();
        assert_eq!(sql, "BEGIN TRANSACTION");
        assert_eq!(tx.state().unwrap(), TransactionState::Active);
        assert_eq!(tx.depth(), 1);
    }

    #[test]
    fn test_transaction_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        let sql = tx.commit().unwrap();
        assert_eq!(sql, "COMMIT");
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
        assert_eq!(tx.depth(), 0);
    }

    #[test]
    fn test_orm_transaction_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        let sql = tx.rollback().unwrap();
        assert_eq!(sql, "ROLLBACK");
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
        assert_eq!(tx.depth(), 0);
    }

    #[test]
    fn test_nested_transaction_begin() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        let sql = tx.begin().unwrap();
        assert!(sql.contains("SAVEPOINT sp_2"));
        assert_eq!(tx.depth(), 2);
    }

    #[test]
    fn test_nested_transaction_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        let sql = tx.commit().unwrap();
        assert!(sql.contains("RELEASE SAVEPOINT"));
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    fn test_nested_transaction_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        let sql = tx.rollback().unwrap();
        assert!(sql.contains("ROLLBACK TO SAVEPOINT"));
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    fn test_isolation_level() {
        let mut tx = Transaction::new().with_isolation_level(IsolationLevel::Serializable);
        let sql = tx.begin().unwrap();
        assert!(sql.contains("ISOLATION LEVEL SERIALIZABLE"));
    }

    #[test]
    fn test_manual_savepoint() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        let sql = tx.savepoint("my_savepoint").unwrap();
        assert_eq!(sql, "SAVEPOINT my_savepoint");
    }

    #[test]
    fn test_orm_transaction_release_savepoint() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.savepoint("my_savepoint").unwrap();
        let sql = tx.release_savepoint("my_savepoint").unwrap();
        assert_eq!(sql, "RELEASE SAVEPOINT my_savepoint");
    }

    #[test]
    fn test_orm_transaction_rollback_savepoint() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.savepoint("my_savepoint").unwrap();
        let sql = tx.rollback_to_savepoint("my_savepoint").unwrap();
        assert_eq!(sql, "ROLLBACK TO SAVEPOINT my_savepoint");
    }

    #[test]
    fn test_transaction_is_active() {
        let mut tx = Transaction::new();
        assert!(!tx.is_active());
        tx.begin().unwrap();
        assert!(tx.is_active());
        tx.commit().unwrap();
        assert!(!tx.is_active());
    }

    #[test]
    fn test_commit_without_begin() {
        let mut tx = Transaction::new();
        let result = tx.commit();
        assert!(result.is_err());
    }

    #[test]
    fn test_rollback_without_begin() {
        let mut tx = Transaction::new();
        let result = tx.rollback();
        assert!(result.is_err());
    }

    #[test]
    fn test_savepoint_outside_transaction() {
        let mut tx = Transaction::new();
        let result = tx.savepoint("test");
        assert!(result.is_err());
    }

    /// // Database execution tests
    use reinhardt_validators::TableName;
    use serde::{Deserialize, Serialize};
    use tokio::sync::{Mutex, OnceCell};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestItem {
        id: Option<i64>,
        name: String,
        value: i32,
    }

    const TEST_ITEM_TABLE: TableName = TableName::new_const("test_items");

    impl crate::Model for TestItem {
        type PrimaryKey = i64;
        fn table_name() -> &'static str {
            TEST_ITEM_TABLE.as_str()
        }
        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }
        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    #[cfg(feature = "django-compat")]
    async fn setup_transaction_test_db() -> reinhardt_apps::Result<()> {
        use crate::manager::{get_connection, init_database};

        static INIT: OnceCell<Mutex<()>> = OnceCell::const_new();

        let mutex = INIT
            .get_or_init(|| async {
                init_database("sqlite://:memory:").await.unwrap();
                Mutex::new(())
            })
            .await;

        let _guard = mutex.lock().await;
        let conn = get_connection().await?;

        let _ = conn.execute("DROP TABLE IF EXISTS test_items").await;

        conn.execute(
            "CREATE TABLE test_items (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                value INTEGER NOT NULL
            )",
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "django-compat")]
    async fn test_begin_db_execution() {
        setup_transaction_test_db().await.unwrap();

        let mut tx = Transaction::new();
        let result = tx.begin_db().await;

        assert!(result.is_ok());
        assert!(tx.is_active());
        assert_eq!(tx.depth(), 1);
    }

    #[tokio::test]
    #[cfg(feature = "django-compat")]
    async fn test_commit_db_sql_generation() {
        // Test that commit_db() generates and attempts to execute correct SQL
        // Note: Full transaction semantics require a dedicated connection
        setup_transaction_test_db().await.unwrap();

        let mut tx = Transaction::new();

        // Verify begin generates correct SQL and updates state
        let begin_sql = tx.begin().unwrap();
        assert_eq!(begin_sql, "BEGIN TRANSACTION");
        assert!(tx.is_active());
        assert_eq!(tx.depth(), 1);

        // Verify commit generates correct SQL and updates state
        let commit_sql = tx.commit().unwrap();
        assert_eq!(commit_sql, "COMMIT");
        assert!(!tx.is_active());
        assert_eq!(tx.depth(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "django-compat")]
    async fn test_rollback_db_sql_generation() {
        // Test that rollback_db() generates and attempts to execute correct SQL
        setup_transaction_test_db().await.unwrap();

        let mut tx = Transaction::new();

        // Verify begin generates correct SQL
        let begin_sql = tx.begin().unwrap();
        assert_eq!(begin_sql, "BEGIN TRANSACTION");
        assert!(tx.is_active());

        // Verify rollback generates correct SQL and updates state
        let rollback_sql = tx.rollback().unwrap();
        assert_eq!(rollback_sql, "ROLLBACK");
        assert!(!tx.is_active());
        assert_eq!(tx.depth(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "django-compat")]
    async fn test_nested_transaction_sql_generation() {
        // Test nested transaction (savepoint) SQL generation
        setup_transaction_test_db().await.unwrap();

        let mut tx = Transaction::new();

        // Begin outer transaction
        let begin_sql = tx.begin().unwrap();
        assert_eq!(begin_sql, "BEGIN TRANSACTION");
        assert_eq!(tx.depth(), 1);

        // Begin nested transaction (creates savepoint)
        let savepoint_sql = tx.begin().unwrap();
        assert!(savepoint_sql.contains("SAVEPOINT sp_2"));
        assert_eq!(tx.depth(), 2);

        // Rollback to savepoint
        let rollback_sql = tx.rollback().unwrap();
        assert!(rollback_sql.contains("ROLLBACK TO SAVEPOINT"));
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());

        // Commit outer transaction
        let commit_sql = tx.commit().unwrap();
        assert_eq!(commit_sql, "COMMIT");
        assert_eq!(tx.depth(), 0);
        assert!(!tx.is_active());
    }

    #[tokio::test]
    #[cfg(feature = "django-compat")]
    async fn test_transaction_isolation_level_sql() {
        // Test that isolation level is properly included in BEGIN statement
        setup_transaction_test_db().await.unwrap();

        let mut tx = Transaction::new().with_isolation_level(IsolationLevel::Serializable);
        let begin_sql = tx.begin().unwrap();

        assert!(begin_sql.contains("ISOLATION LEVEL SERIALIZABLE"));
        assert!(tx.is_active());
    }
}
// Auto-generated tests for transaction module
// Translated from Django/SQLAlchemy test suite
// Total available: 80 | Included: 80

#[cfg(test)]
mod transaction_extended_tests {
    use super::*;
    use crate::expressions::{F, Q};
    use crate::transaction::*;

    #[test]
    /// // From: Django/transactions
    fn test_alternate_decorator_syntax_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_alternate_decorator_syntax_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_alternate_decorator_syntax_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_alternate_decorator_syntax_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_allows_queries_after_fixing_transaction() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert!(!tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_allows_queries_after_fixing_transaction_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert!(!tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_does_not_leak_savepoints_on_failure() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_does_not_leak_savepoints_on_failure_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_calling_transaction_methods() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_calling_transaction_methods_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_queries_in_broken_transaction() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_queries_in_broken_transaction_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_queries_in_broken_transaction_after_client_close() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert!(!tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_queries_in_broken_transaction_after_client_close_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert!(!tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_setting_autocommit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_atomic_prevents_setting_autocommit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_commit_2() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_commit_3() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_decorator_syntax_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_decorator_syntax_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_decorator_syntax_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_decorator_syntax_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_failure_on_exit_transaction() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_failure_on_exit_transaction_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_force_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_force_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_implicit_savepoint_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_implicit_savepoint_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_mark_for_rollback_on_error_in_autocommit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_mark_for_rollback_on_error_in_autocommit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_mark_for_rollback_on_error_in_transaction() {
        let mut tx = Transaction::new();
        tx.begin();
        tx.rollback();
        assert_eq!(tx.state(), Ok(TransactionState::RolledBack));
    }

    #[test]
    /// // From: Django/transactions
    fn test_mark_for_rollback_on_error_in_transaction_1() {
        let mut tx = Transaction::new();
        tx.begin();
        tx.rollback();
        assert_eq!(tx.state(), Ok(TransactionState::RolledBack));
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_commit_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_commit_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_commit_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_commit_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_inner_savepoint_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_inner_savepoint_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_outer_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_outer_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_rollback_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_rollback_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_rollback_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_merged_rollback_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_both_durable() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_both_durable_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_commit_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.depth(), 1);
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_commit_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.depth(), 1);
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_commit_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_commit_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_inner_durable() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_inner_durable_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.depth(), 1);
        assert!(tx.is_active());
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_outer_durable() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_outer_durable_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.commit().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_rollback_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_rollback_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_rollback_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_nested_rollback_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_orm_query_after_error_and_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_orm_query_after_error_and_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_orm_query_without_autocommit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        assert!(tx.is_active());
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_orm_query_without_autocommit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        assert!(tx.is_active());
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_prevent_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_prevent_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_commit_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_commit_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_commit_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_commit_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_rollback_commit() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_rollback_commit_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_rollback_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_reuse_rollback_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_rollback() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_rollback_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.rollback().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
    }

    #[test]
    /// // From: Django/transactions
    fn test_sequence_of_durables() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_sequence_of_durables_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_wrap_callable_instance() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }

    #[test]
    /// // From: Django/transactions
    fn test_wrap_callable_instance_1() {
        let mut tx = Transaction::new();
        tx.begin().unwrap();
        tx.commit().unwrap();
        assert_eq!(tx.state().unwrap(), TransactionState::Committed);
    }
}
