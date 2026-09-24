//! PostgreSQL implementation of the issuer's OIDC-specific state.

use super::store::{OidcCodeContext, OidcKey, OidcPending, OidcStateStore, PublicRsaJwk};
use async_trait::async_trait;
use reinhardt_db::migrations::{Migration, Operation};
use reinhardt_query::prelude::{
	Alias, Cond, Expr, ExprTrait, IntoIden, OnConflict, Order, PostgresQueryBuilder, Query,
	QueryStatementBuilder, Value,
};
use serde::Serialize;
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

	/// Delete expired OIDC continuations and code contexts from shared storage.
	/// Run this periodically from the host maintenance job.
	pub async fn purge_expired(&self, now: i64) -> Result<u64, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let mut deleted = 0;
		for table in ["oidc_op_pending", "oidc_op_codes"] {
			let (sql, _) = Query::delete()
				.from_table(Alias::new(table))
				.and_where(Expr::col(Alias::new("expires_at").into_iden()).lte(now))
				.build(PostgresQueryBuilder);
			deleted += sqlx::query(&sql)
				.bind(now)
				.execute(&mut *tx)
				.await
				.map_err(|e| e.to_string())?
				.rows_affected();
		}
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(deleted)
	}
}

fn json_value<T: Serialize>(value: &T) -> Result<Value, String> {
	serde_json::to_value(value)
		.map(|json| Value::Json(Some(Box::new(json))))
		.map_err(|e| e.to_string())
}

#[async_trait]
impl OidcStateStore for PostgresOidcStore {
	async fn subject_or_insert(&self, user_id: &str, proposed: &str) -> Result<String, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, _) = Query::select()
			.column(Alias::new("sub"))
			.from(Alias::new("oidc_op_subjects"))
			.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let existing: Option<(String,)> = sqlx::query_as(&sql)
			.bind(user_id)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if let Some((subject,)) = existing {
			tx.commit().await.map_err(|e| e.to_string())?;
			return Ok(subject);
		}
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oidc_op_subject_reservations"))
			.columns(["sub"])
			.values(vec![proposed.into()])?
			.on_conflict(OnConflict::column("sub").do_nothing())
			.build(PostgresQueryBuilder);
		let reservation = sqlx::query(&sql)
			.bind(proposed)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if reservation.rows_affected() != 1 {
			return Err("subject collision".to_owned());
		}
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oidc_op_subjects"))
			.columns(["user_id", "sub"])
			.values(vec![user_id.into(), proposed.into()])?
			.on_conflict(OnConflict::column("user_id").do_nothing())
			.build(PostgresQueryBuilder);
		let inserted = sqlx::query(&sql)
			.bind(user_id)
			.bind(proposed)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let subject = if inserted.rows_affected() == 1 {
			proposed.to_owned()
		} else {
			let (sql, _) = Query::select()
				.column(Alias::new("sub"))
				.from(Alias::new("oidc_op_subjects"))
				.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
				.build(PostgresQueryBuilder);
			let (subject,): (String,) = sqlx::query_as(&sql)
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
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oidc_op_subjects"))
			.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(user_id)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn retire_user(&self, user_id: &str) -> Result<(), String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_codes"))
			.value_expr(Alias::new("redeemed"), Expr::cust("TRUE"))
			.value_expr(Alias::new("replayed"), Expr::cust("TRUE"))
			.and_where(Expr::cust_with_values("payload->>'user_id' = ?", [user_id]))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(user_id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_tokens"))
			.value_expr(Alias::new("revoked"), Expr::cust("TRUE"))
			.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(user_id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oidc_op_codes"))
			.and_where(Expr::cust_with_values("payload->>'user_id' = ?", [user_id]))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(user_id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oidc_op_subjects"))
			.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(user_id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn put_pending(&self, id: &str, pending: OidcPending) -> Result<(), String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oidc_op_pending"))
			.columns(["id", "payload", "expires_at"])
			.values(vec![
				id.into(),
				json_value(&pending)?,
				pending.expires_at.into(),
			])?
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(id)
			.bind(Json(&pending))
			.bind(pending.expires_at)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		now: i64,
	) -> Result<Option<OidcPending>, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oidc_op_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<OidcPending>,)> = sqlx::query_as(&sql)
			.bind(id)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let Some((Json(pending),)) = row else {
			return Ok(None);
		};
		if pending.session_digest != session_digest || pending.expires_at <= now {
			return Ok(None);
		}
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oidc_op_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(Some(pending))
	}

	async fn put_code(&self, context: OidcCodeContext) -> Result<(), String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oidc_op_codes"))
			.columns(["digest", "payload", "expires_at"])
			.values(vec![
				context.digest.clone().into(),
				json_value(&context)?,
				context.expires_at.into(),
			])?
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(&context.digest)
			.bind(Json(&context))
			.bind(context.expires_at)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn code(&self, digest: &str) -> Result<Option<OidcCodeContext>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oidc_op_codes"))
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<OidcCodeContext>,)> = sqlx::query_as(&sql)
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
		let (sql, _) = Query::update()
			.table(Alias::new("oidc_op_keys"))
			.value_expr(Alias::new("active"), Expr::cust("FALSE"))
			.value(Alias::new("publish_until"), now + retention)
			.and_where(Expr::col(Alias::new("active").into_iden()).eq(true))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(now + retention)
			.bind(true)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oidc_op_keys"))
			.columns(["kid", "public", "activated_at", "active", "compromised"])
			.values(vec![
				public.kid.clone().into(),
				json_value(&public)?,
				now.into(),
				true.into(),
				false.into(),
			])?
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(&public.kid)
			.bind(Json(&public))
			.bind(now)
			.bind(true)
			.bind(false)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn active_key(&self) -> Result<Option<OidcKey>, String> {
		let (sql, _) = Query::select()
			.columns([
				"public",
				"activated_at",
				"active",
				"publish_until",
				"compromised",
			])
			.from(Alias::new("oidc_op_keys"))
			.and_where(Expr::col(Alias::new("active").into_iden()).eq(true))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<PublicRsaJwk>, i64, bool, Option<i64>, bool)> = sqlx::query_as(&sql)
			.bind(true)
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
		let (sql, _) = Query::select()
			.columns([
				"public",
				"activated_at",
				"active",
				"publish_until",
				"compromised",
			])
			.from(Alias::new("oidc_op_keys"))
			.and_where(Expr::col(Alias::new("compromised").into_iden()).eq(false))
			.cond_where(
				Cond::any()
					.add(Expr::col(Alias::new("active").into_iden()).eq(true))
					.add(Expr::col(Alias::new("publish_until").into_iden()).gt(now)),
			)
			.order_by(Alias::new("kid"), Order::Asc)
			.build(PostgresQueryBuilder);
		let rows: Vec<(Json<PublicRsaJwk>, i64, bool, Option<i64>, bool)> = sqlx::query_as(&sql)
			.bind(false)
			.bind(true)
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
		let (sql, _) = Query::update()
			.table(Alias::new("oidc_op_keys"))
			.value_expr(Alias::new("active"), Expr::cust("FALSE"))
			.value_expr(Alias::new("publish_until"), Expr::cust("NULL"))
			.value_expr(Alias::new("compromised"), Expr::cust("TRUE"))
			.and_where(Expr::col(Alias::new("kid").into_iden()).eq(kid))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(kid)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
}
