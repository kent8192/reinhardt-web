//! MySQL-specific Two-Phase Commit implementation
//!
//! This module implements the `TwoPhaseParticipant` trait for MySQL using
//! the XA transaction protocol (XA START, XA END, XA PREPARE, XA COMMIT, XA ROLLBACK).

use sqlx::{AssertSqlSafe, MySqlPool, Row};
use std::sync::Arc;

use crate::error::{DatabaseError, Result};

/// MySQL Two-Phase Commit participant using XA transactions
///
/// Manages two-phase commit transactions using MySQL's XA transaction protocol.
/// XA transactions in MySQL follow the X/Open XA standard for distributed
/// transaction processing.
///
/// # XA Transaction States
///
/// - ACTIVE: After XA START
/// - IDLE: After XA END
/// - PREPARED: After XA PREPARE
/// - COMMITTED: After XA COMMIT
/// - ROLLBACK: After XA ROLLBACK
///
/// # Examples
///
/// ```no_run
/// use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
/// use sqlx::MySqlPool;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
/// let participant = MySqlTwoPhaseParticipant::new(pool);
///
/// // Start an XA transaction
/// participant.begin("txn_001").await?;
///
/// // ... perform operations ...
///
/// // End the XA transaction
/// participant.end("txn_001").await?;
///
/// // Prepare the transaction
/// participant.prepare("txn_001").await?;
///
/// // Commit the prepared transaction
/// participant.commit("txn_001").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MySqlTwoPhaseParticipant {
    pool: Arc<MySqlPool>,
}

impl MySqlTwoPhaseParticipant {
    /// Create a new MySQL two-phase commit participant
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// use sqlx::MySqlPool;
    ///
    /// # async fn example() -> Result<(), sqlx::Error> {
    /// let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// let participant = MySqlTwoPhaseParticipant::new(pool);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Create from an Arc<MySqlPool>
    pub fn from_pool_arc(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Start an XA transaction
    ///
    /// This executes `XA START 'xid'` in MySQL.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// participant.begin("txn_001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn begin(&self, xid: &str) -> Result<()> {
        let sql = format!("XA START '{}'", Self::escape_xid(xid));
        sqlx::query(AssertSqlSafe(&*sql))
            .execute(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;
        Ok(())
    }

    /// End an XA transaction
    ///
    /// This executes `XA END 'xid'` in MySQL. Must be called before XA PREPARE.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// participant.begin("txn_001").await?;
    /// // ... perform operations ...
    /// participant.end("txn_001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn end(&self, xid: &str) -> Result<()> {
        let sql = format!("XA END '{}'", Self::escape_xid(xid));
        sqlx::query(AssertSqlSafe(&*sql))
            .execute(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Prepare an XA transaction for two-phase commit
    ///
    /// This executes `XA PREPARE 'xid'` in MySQL.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The XA transaction is not in IDLE state (must call `end()` first)
    /// - The transaction ID does not exist
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// participant.begin("txn_001").await?;
    /// // ... perform operations ...
    /// participant.end("txn_001").await?;
    /// participant.prepare("txn_001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prepare(&self, xid: &str) -> Result<()> {
        let sql = format!("XA PREPARE '{}'", Self::escape_xid(xid));
        sqlx::query(AssertSqlSafe(&*sql))
            .execute(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Commit a prepared XA transaction
    ///
    /// This executes `XA COMMIT 'xid'` in MySQL.
    ///
    /// # Errors
    ///
    /// Returns an error if the prepared transaction does not exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// participant.commit("txn_001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn commit(&self, xid: &str) -> Result<()> {
        let sql = format!("XA COMMIT '{}'", Self::escape_xid(xid));
        sqlx::query(AssertSqlSafe(&*sql))
            .execute(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Commit an XA transaction with one-phase optimization
    ///
    /// This executes `XA COMMIT 'xid' ONE PHASE` in MySQL. This is an optimization
    /// for single-phase commit when the transaction is in IDLE state (after XA END).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// participant.begin("txn_001").await?;
    /// // ... perform operations ...
    /// participant.end("txn_001").await?;
    /// participant.commit_one_phase("txn_001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn commit_one_phase(&self, xid: &str) -> Result<()> {
        let sql = format!("XA COMMIT '{}' ONE PHASE", Self::escape_xid(xid));
        sqlx::query(AssertSqlSafe(&*sql))
            .execute(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Rollback a prepared XA transaction
    ///
    /// This executes `XA ROLLBACK 'xid'` in MySQL.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// participant.rollback("txn_001").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rollback(&self, xid: &str) -> Result<()> {
        let sql = format!("XA ROLLBACK '{}'", Self::escape_xid(xid));
        sqlx::query(AssertSqlSafe(&*sql))
            .execute(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Query all prepared XA transactions using XA RECOVER
    ///
    /// Returns a list of prepared transaction IDs. This is useful for recovery
    /// scenarios where you need to find orphaned prepared transactions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// let prepared_txns = participant.list_prepared_transactions().await?;
    /// for txn_info in prepared_txns {
    ///     println!("Prepared XA transaction: {:?}", txn_info);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_prepared_transactions(&self) -> Result<Vec<XaTransactionInfo>> {
        let rows = sqlx::query("XA RECOVER")
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(DatabaseError::from)?;

        let mut transactions = Vec::new();
        for row in rows {
            // XA RECOVER returns: formatID, gtrid_length, bqual_length, data
            let format_id: i32 = row.try_get("formatID").map_err(DatabaseError::from)?;
            let gtrid_length: i32 = row.try_get("gtrid_length").map_err(DatabaseError::from)?;
            let bqual_length: i32 = row.try_get("bqual_length").map_err(DatabaseError::from)?;
            let data: Vec<u8> = row.try_get("data").map_err(DatabaseError::from)?;

            transactions.push(XaTransactionInfo {
                format_id,
                gtrid_length,
                bqual_length,
                data: data.clone(),
                xid: String::from_utf8_lossy(&data).to_string(),
            });
        }

        Ok(transactions)
    }

    /// Find a specific prepared XA transaction
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// if let Some(info) = participant.find_prepared_transaction("txn_001").await? {
    ///     println!("Found prepared XA transaction: {:?}", info);
    ///     // Decide whether to commit or rollback
    ///     participant.commit("txn_001").await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_prepared_transaction(&self, xid: &str) -> Result<Option<XaTransactionInfo>> {
        let all_txns = self.list_prepared_transactions().await?;
        Ok(all_txns.into_iter().find(|txn| txn.xid == xid))
    }

    /// Cleanup stale prepared XA transactions
    ///
    /// This method queries all prepared transactions and attempts to rollback
    /// those matching a specific pattern. Use with caution as this may affect
    /// in-progress distributed transactions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_db_backends::backends::mysql::two_phase::MySqlTwoPhaseParticipant;
    /// # use sqlx::MySqlPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = MySqlPool::connect("mysql://localhost/mydb").await?;
    /// # let participant = MySqlTwoPhaseParticipant::new(pool);
    /// // Cleanup all transactions starting with "stale_"
    /// let cleaned = participant.cleanup_stale_transactions("stale_").await?;
    /// println!("Cleaned up {} stale XA transactions", cleaned);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cleanup_stale_transactions(&self, prefix: &str) -> Result<usize> {
        let all_txns = self.list_prepared_transactions().await?;
        let mut cleaned = 0;

        for txn in all_txns {
            if txn.xid.starts_with(prefix) {
                if self.rollback(&txn.xid).await.is_ok() {
                    cleaned += 1;
                }
            }
        }

        Ok(cleaned)
    }

    /// Escape XID to prevent SQL injection
    ///
    /// Note: MySQL XA transaction IDs have specific format requirements.
    /// This is a simple escaping mechanism that removes single quotes.
    fn escape_xid(xid: &str) -> String {
        xid.replace('\'', "''")
    }
}

/// Information about an XA transaction from XA RECOVER
#[derive(Debug, Clone, PartialEq)]
pub struct XaTransactionInfo {
    /// Format identifier
    pub format_id: i32,
    /// Global transaction ID length
    pub gtrid_length: i32,
    /// Branch qualifier length
    pub bqual_length: i32,
    /// Raw transaction data
    pub data: Vec<u8>,
    /// String representation of the XID
    pub xid: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xa_transaction_info_creation() {
        let info = XaTransactionInfo {
            format_id: 1,
            gtrid_length: 7,
            bqual_length: 0,
            data: b"txn_001".to_vec(),
            xid: "txn_001".to_string(),
        };

        assert_eq!(info.format_id, 1);
        assert_eq!(info.gtrid_length, 7);
        assert_eq!(info.bqual_length, 0);
        assert_eq!(info.xid, "txn_001");
    }

    #[test]
    fn test_escape_xid() {
        assert_eq!(MySqlTwoPhaseParticipant::escape_xid("simple"), "simple");
        assert_eq!(
            MySqlTwoPhaseParticipant::escape_xid("it's"),
            "it''s"
        );
        assert_eq!(
            MySqlTwoPhaseParticipant::escape_xid("a'b'c"),
            "a''b''c"
        );
    }

    #[test]
    fn test_participant_clone() {
        // Test that MySqlTwoPhaseParticipant can be cloned
        let pool = Arc::new(
            MySqlPool::connect_lazy("mysql://localhost/testdb")
                .expect("Failed to create lazy pool"),
        );
        let participant1 = MySqlTwoPhaseParticipant::from_pool_arc(pool.clone());
        let participant2 = participant1.clone();

        // Both should reference the same pool
        assert!(Arc::ptr_eq(&participant1.pool, &participant2.pool));
    }
}
