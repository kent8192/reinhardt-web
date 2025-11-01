//! PostgreSQL-specific Two-Phase Commit implementation
//!
//! This module implements the `TwoPhaseParticipant` trait for PostgreSQL using
//! the PREPARE TRANSACTION, COMMIT PREPARED, and ROLLBACK PREPARED SQL commands.

use sqlx::{PgPool, Row};
use std::sync::Arc;

use crate::error::{DatabaseError, Result};

/// PostgreSQL Two-Phase Commit participant
///
/// Manages two-phase commit transactions using PostgreSQL's PREPARE TRANSACTION
/// functionality. PostgreSQL requires `max_prepared_transactions` to be set to
/// a non-zero value in postgresql.conf for this feature to work.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
/// use sqlx::PgPool;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = PgPool::connect("postgresql://localhost/mydb").await?;
/// let participant = PostgresTwoPhaseParticipant::new(pool);
///
/// // Begin a transaction
/// participant.begin("txn_001").await?;
///
/// // ... perform operations ...
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
pub struct PostgresTwoPhaseParticipant {
	pool: Arc<PgPool>,
}

impl PostgresTwoPhaseParticipant {
	/// Create a new PostgreSQL two-phase commit participant
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// use sqlx::PgPool;
	///
	/// # async fn example() -> Result<(), sqlx::Error> {
	/// let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// let participant = PostgresTwoPhaseParticipant::new(pool);
	/// # Ok(())
	/// # }
	/// ```
	pub fn new(pool: PgPool) -> Self {
		Self {
			pool: Arc::new(pool),
		}
	}

	/// Create from an Arc<PgPool>
	pub fn from_pool_arc(pool: Arc<PgPool>) -> Self {
		Self { pool }
	}

	/// Begin a transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// participant.begin("txn_001").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn begin(&self, _xid: &str) -> Result<()> {
		sqlx::query("BEGIN")
			.execute(self.pool.as_ref())
			.await
			.map_err(DatabaseError::from)?;
		Ok(())
	}

	/// Prepare a transaction for two-phase commit
	///
	/// This executes `PREPARE TRANSACTION 'xid'` in PostgreSQL.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The transaction is not active
	/// - `max_prepared_transactions` is set to 0
	/// - The transaction ID already exists
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// participant.begin("txn_001").await?;
	/// // ... perform operations ...
	/// participant.prepare("txn_001").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn prepare(&self, xid: &str) -> Result<()> {
		// Use Box::leak to convert String to &'static str for sqlx compatibility
		// This is acceptable as prepared transaction IDs are typically short-lived
		let xid_escaped = pg_escape::quote_literal(xid);
		let sql = format!("PREPARE TRANSACTION {}", xid_escaped);
		let sql_static: &'static str = Box::leak(sql.into_boxed_str());
		sqlx::query(sql_static)
			.execute(self.pool.as_ref())
			.await
			.map_err(DatabaseError::from)?;
		Ok(())
	}

	/// Commit a prepared transaction
	///
	/// This executes `COMMIT PREPARED 'xid'` in PostgreSQL.
	///
	/// # Errors
	///
	/// Returns an error if the prepared transaction does not exist.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// participant.commit("txn_001").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn commit(&self, xid: &str) -> Result<()> {
		// Use Box::leak to convert String to &'static str for sqlx compatibility
		let xid_escaped = pg_escape::quote_literal(xid);
		let sql = format!("COMMIT PREPARED {}", xid_escaped);
		let sql_static: &'static str = Box::leak(sql.into_boxed_str());
		sqlx::query(sql_static)
			.execute(self.pool.as_ref())
			.await
			.map_err(DatabaseError::from)?;
		Ok(())
	}

	/// Rollback a prepared transaction
	///
	/// This executes `ROLLBACK PREPARED 'xid'` in PostgreSQL.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// participant.rollback("txn_001").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn rollback(&self, xid: &str) -> Result<()> {
		// Use Box::leak to convert String to &'static str for sqlx compatibility
		let xid_escaped = pg_escape::quote_literal(xid);
		let sql = format!("ROLLBACK PREPARED {}", xid_escaped);
		let sql_static: &'static str = Box::leak(sql.into_boxed_str());
		sqlx::query(sql_static)
			.execute(self.pool.as_ref())
			.await
			.map_err(DatabaseError::from)?;
		Ok(())
	}

	/// Query all prepared transactions from pg_prepared_xacts
	///
	/// Returns a list of prepared transaction IDs. This is useful for recovery
	/// scenarios where you need to find orphaned prepared transactions.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// let prepared_txns = participant.list_prepared_transactions().await?;
	/// for txn_info in prepared_txns {
	///     println!("Prepared transaction: {}", txn_info.gid);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list_prepared_transactions(&self) -> Result<Vec<PreparedTransactionInfo>> {
		let rows = sqlx::query(
			"SELECT gid, prepared, owner, database FROM pg_prepared_xacts ORDER BY prepared",
		)
		.fetch_all(self.pool.as_ref())
		.await
		.map_err(DatabaseError::from)?;

		let mut transactions = Vec::new();
		for row in rows {
			transactions.push(PreparedTransactionInfo {
				gid: row.try_get("gid").map_err(DatabaseError::from)?,
				prepared: row.try_get("prepared").map_err(DatabaseError::from)?,
				owner: row.try_get("owner").map_err(DatabaseError::from)?,
				database: row.try_get("database").map_err(DatabaseError::from)?,
			});
		}

		Ok(transactions)
	}

	/// Recover a specific prepared transaction
	///
	/// This is a convenience method that checks if a transaction is prepared
	/// and returns its information.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// if let Some(info) = participant.find_prepared_transaction("txn_001").await? {
	///     println!("Found prepared transaction: {:?}", info);
	///     // Decide whether to commit or rollback
	///     participant.commit("txn_001").await?;
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn find_prepared_transaction(
		&self,
		xid: &str,
	) -> Result<Option<PreparedTransactionInfo>> {
		let row = sqlx::query(
			"SELECT gid, prepared, owner, database FROM pg_prepared_xacts WHERE gid = $1",
		)
		.bind(xid)
		.fetch_optional(self.pool.as_ref())
		.await
		.map_err(DatabaseError::from)?;

		if let Some(row) = row {
			Ok(Some(PreparedTransactionInfo {
				gid: row.try_get("gid").map_err(DatabaseError::from)?,
				prepared: row.try_get("prepared").map_err(DatabaseError::from)?,
				owner: row.try_get("owner").map_err(DatabaseError::from)?,
				database: row.try_get("database").map_err(DatabaseError::from)?,
			}))
		} else {
			Ok(None)
		}
	}

	/// Cleanup stale prepared transactions older than the specified duration
	///
	/// This is useful for recovering from failures where prepared transactions
	/// were abandoned. Use with caution as this may affect in-progress distributed
	/// transactions.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::backends::postgresql::two_phase::PostgresTwoPhaseParticipant;
	/// # use sqlx::PgPool;
	/// # use std::time::Duration;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let pool = PgPool::connect("postgresql://localhost/mydb").await?;
	/// # let participant = PostgresTwoPhaseParticipant::new(pool);
	/// // Rollback transactions older than 1 hour
	/// let cleaned = participant.cleanup_stale_transactions(Duration::from_secs(3600)).await?;
	/// println!("Cleaned up {} stale transactions", cleaned);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn cleanup_stale_transactions(&self, max_age: std::time::Duration) -> Result<usize> {
		let max_age_secs = max_age.as_secs() as i32;
		let rows = sqlx::query(
			"SELECT gid FROM pg_prepared_xacts
             WHERE EXTRACT(EPOCH FROM (NOW() - prepared)) > $1",
		)
		.bind(max_age_secs)
		.fetch_all(self.pool.as_ref())
		.await
		.map_err(DatabaseError::from)?;

		let mut cleaned = 0;
		for row in rows {
			let gid: String = row.try_get("gid").map_err(DatabaseError::from)?;
			if self.rollback(&gid).await.is_ok() {
				cleaned += 1;
			}
		}

		Ok(cleaned)
	}
}

/// Information about a prepared transaction
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedTransactionInfo {
	/// Global transaction identifier
	pub gid: String,
	/// Timestamp when the transaction was prepared
	pub prepared: chrono::NaiveDateTime,
	/// Owner (role) of the transaction
	pub owner: String,
	/// Database name
	pub database: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_prepared_transaction_info_creation() {
		let info = PreparedTransactionInfo {
			gid: "txn_001".to_string(),
			prepared: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
			owner: "postgres".to_string(),
			database: "testdb".to_string(),
		};

		assert_eq!(info.gid, "txn_001");
		assert_eq!(info.owner, "postgres");
		assert_eq!(info.database, "testdb");
	}

	#[tokio::test]
	async fn test_participant_clone() {
		// Test that PostgresTwoPhaseParticipant can be cloned
		// This is important for use in multi-threaded contexts
		let pool = Arc::new(
			PgPool::connect_lazy("postgresql://localhost/testdb")
				.expect("Failed to create lazy pool"),
		);
		let participant1 = PostgresTwoPhaseParticipant::from_pool_arc(pool.clone());
		let participant2 = participant1.clone();

		// Both should reference the same pool
		assert!(Arc::ptr_eq(&participant1.pool, &participant2.pool));
	}
}
