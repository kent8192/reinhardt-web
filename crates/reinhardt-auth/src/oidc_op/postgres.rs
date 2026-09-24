//! PostgreSQL implementation of the issuer's OIDC-specific state.

use super::store::{OidcCodeContext, OidcKey, OidcPending, OidcStateStore, PublicRsaJwk};
use async_trait::async_trait;
use reinhardt_db::migrations::{Migration, Operation};
use reinhardt_query::prelude::{
	Alias, Expr, ExprTrait, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use sqlx::{PgPool, types::Json};

/// OIDC subject, continuation, and key state shared by PostgreSQL-backed nodes.
#[derive(Clone)]
pub struct PostgresOidcStore {
	pool: PgPool,
}

impl PostgresOidcStore {
	/// Use a pool whose OIDC schema has been migrated.
	pub fn new(pool: PgPool) -> Self {
		Self { pool }
	}

	/// Return the OIDC migration to register after the OAuth server migration.
	pub fn migration() -> Migration {
		const REVERSE: [&str; 8] = [
			"DROP TABLE oidc_op_subject_reservations",
			"DROP TABLE oidc_op_subjects",
			"DROP TABLE oidc_op_pending",
			"DROP INDEX oidc_op_pending_expiry",
			"DROP TABLE oidc_op_codes",
			"DROP INDEX oidc_op_codes_expiry",
			"DROP TABLE oidc_op_keys",
			"DROP INDEX oidc_op_one_active_key",
		];
		let statements: Vec<_> = include_str!("../../migrations/0002_oidc_op.sql")
			.split(';')
			.map(str::trim)
			.filter(|sql| !sql.is_empty())
			.collect();
		assert_eq!(
			statements.len(),
			REVERSE.len(),
			"OIDC migration reverse count must match"
		);
		statements.into_iter().zip(REVERSE).fold(
			Migration::new("0002_oidc_op", "oidc_op"),
			|migration, (sql, reverse_sql)| {
				migration.add_operation(Operation::RunSQL {
					sql: sql.to_owned(),
					reverse_sql: Some(reverse_sql.to_owned()),
				})
			},
		)
	}

	/// Access the underlying pool for host-managed migrations.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}
}

#[async_trait]
impl OidcStateStore for PostgresOidcStore {
	async fn subject_or_insert(&self, user_id: &str, proposed: &str) -> Result<String, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let existing: Option<(String,)> =
			sqlx::query_as("SELECT sub FROM oidc_op_subjects WHERE user_id=$1 FOR UPDATE")
				.bind(user_id)
				.fetch_optional(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
		if let Some((subject,)) = existing {
			tx.commit().await.map_err(|e| e.to_string())?;
			return Ok(subject);
		}
		let reservation = sqlx::query(
			"INSERT INTO oidc_op_subject_reservations(sub) VALUES ($1) ON CONFLICT DO NOTHING",
		)
		.bind(proposed)
		.execute(&mut *tx)
		.await
		.map_err(|e| e.to_string())?;
		if reservation.rows_affected() != 1 {
			return Err("subject collision".to_owned());
		}
		let inserted = sqlx::query(
			"INSERT INTO oidc_op_subjects(user_id,sub) VALUES ($1,$2) ON CONFLICT (user_id) DO NOTHING",
		)
		.bind(user_id)
		.bind(proposed)
		.execute(&mut *tx)
		.await
		.map_err(|e| e.to_string())?;
		let subject = if inserted.rows_affected() == 1 {
			proposed.to_owned()
		} else {
			let (subject,): (String,) =
				sqlx::query_as("SELECT sub FROM oidc_op_subjects WHERE user_id=$1")
					.bind(user_id)
					.fetch_one(&mut *tx)
					.await
					.map_err(|e| e.to_string())?;
			subject
		};
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(subject)
	}

	async fn subject(&self, user_id: &str) -> Result<Option<String>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("sub"))
			.from(Alias::new("oidc_op_subjects"))
			.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
			.build(PostgresQueryBuilder);
		let row: Option<(String,)> = sqlx::query_as(&sql)
			.bind(user_id)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(subject,)| subject))
	}

	async fn retire_subject(&self, user_id: &str) -> Result<(), String> {
		sqlx::query("DELETE FROM oidc_op_subjects WHERE user_id=$1")
			.bind(user_id)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn put_pending(&self, id: &str, pending: OidcPending) -> Result<(), String> {
		sqlx::query("INSERT INTO oidc_op_pending(id,payload,expires_at) VALUES ($1,$2,$3)")
			.bind(id)
			.bind(Json(&pending))
			.bind(pending.expires_at)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn take_pending(&self, id: &str) -> Result<Option<OidcPending>, String> {
		let row: Option<(Json<OidcPending>,)> =
			sqlx::query_as("DELETE FROM oidc_op_pending WHERE id=$1 RETURNING payload")
				.bind(id)
				.fetch_optional(&self.pool)
				.await
				.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(pending),)| pending))
	}

	async fn put_code(&self, context: OidcCodeContext) -> Result<(), String> {
		sqlx::query("INSERT INTO oidc_op_codes(digest,payload,expires_at) VALUES ($1,$2,$3)")
			.bind(&context.digest)
			.bind(Json(&context))
			.bind(context.expires_at)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn code(&self, digest: &str) -> Result<Option<OidcCodeContext>, String> {
		let row: Option<(Json<OidcCodeContext>,)> =
			sqlx::query_as("SELECT payload FROM oidc_op_codes WHERE digest=$1")
				.bind(digest)
				.fetch_optional(&self.pool)
				.await
				.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(context),)| context))
	}

	async fn rotate_key(
		&self,
		public: PublicRsaJwk,
		now: i64,
		retention: i64,
	) -> Result<(), String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		sqlx::query("UPDATE oidc_op_keys SET active=FALSE,publish_until=$1 WHERE active=TRUE")
			.bind(now + retention)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		sqlx::query(
			"INSERT INTO oidc_op_keys(kid,public,activated_at,active,compromised) VALUES ($1,$2,$3,TRUE,FALSE)",
		)
		.bind(&public.kid)
		.bind(Json(&public))
		.bind(now)
		.execute(&mut *tx)
		.await
		.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn active_key(&self) -> Result<Option<OidcKey>, String> {
		let row: Option<(Json<PublicRsaJwk>, i64, bool, Option<i64>, bool)> = sqlx::query_as(
			"SELECT public,activated_at,active,publish_until,compromised FROM oidc_op_keys WHERE active=TRUE",
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(row.map(
			|(Json(public), activated_at, active, publish_until, compromised)| OidcKey {
				public,
				activated_at,
				active,
				publish_until,
				compromised,
			},
		))
	}

	async fn public_keys(&self, now: i64) -> Result<Vec<OidcKey>, String> {
		let rows: Vec<(Json<PublicRsaJwk>, i64, bool, Option<i64>, bool)> = sqlx::query_as(
			"SELECT public,activated_at,active,publish_until,compromised FROM oidc_op_keys WHERE compromised=FALSE AND (active=TRUE OR publish_until>$1) ORDER BY kid",
		)
		.bind(now)
		.fetch_all(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(rows
			.into_iter()
			.map(
				|(Json(public), activated_at, active, publish_until, compromised)| OidcKey {
					public,
					activated_at,
					active,
					publish_until,
					compromised,
				},
			)
			.collect())
	}

	async fn compromise_key(&self, kid: &str) -> Result<(), String> {
		sqlx::query(
			"UPDATE oidc_op_keys SET active=FALSE,publish_until=NULL,compromised=TRUE WHERE kid=$1",
		)
		.bind(kid)
		.execute(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(())
	}
}
