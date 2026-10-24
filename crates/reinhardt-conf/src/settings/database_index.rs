//! Shared index initialization for SQL settings and audit storage.

use sqlx::{AnyPool, mysql::MySqlDatabaseError};

/// Execute index DDL, accepting only MySQL's duplicate-index-name error.
///
/// MySQL has no `CREATE INDEX IF NOT EXISTS`. Handling error 1061 after
/// creation also makes concurrent initialization safe without a check/create race.
pub(super) async fn create_index(pool: &AnyPool, sql: &str) -> Result<(), sqlx::Error> {
	match sqlx::query(sql).execute(pool).await {
		Err(sqlx::Error::Database(error))
			if error
				.try_downcast_ref::<MySqlDatabaseError>()
				.is_some_and(|error| error.number() == 1061) =>
		{
			Ok(())
		}
		result => result.map(|_| ()),
	}
}
