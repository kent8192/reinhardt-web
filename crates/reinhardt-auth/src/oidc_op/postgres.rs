//! PostgreSQL implementation of the issuer's OIDC-specific state.

use super::store::{
	OidcCodeContext, OidcKey, OidcPending, OidcStateStore, PublicRsaJwk, authorization_matches,
};
use crate::database_query::{json_text_path, prepare, schema_operation};
use crate::oauth2_server::{AuthorizationCommit, OAuthServerStore};
use async_trait::async_trait;
use reinhardt_db::migrations::Migration;
use reinhardt_query::prelude::{
	Alias, ColumnDef, Cond, Expr, ExprTrait, IntoIden, OnConflict, Order, Query, Value,
};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction, types::Json};

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
		// Keep the published identity and one statement per operation: PostgreSQL
		// prepares RunSQL operations individually, and rollback runs in reverse order.
		[
			schema_operation(
				Query::create_table()
					.table("oidc_op_subject_reservations")
					.col(ColumnDef::new("sub").text().primary_key(true))
					.take(),
				Query::drop_table()
					.table("oidc_op_subject_reservations")
					.take(),
			),
			schema_operation(
				Query::create_table()
					.table("oidc_op_subjects")
					.col(ColumnDef::new("user_id").text().primary_key(true))
					.col(ColumnDef::new("sub").text().not_null(true).unique(true))
					.foreign_key(["sub"], "oidc_op_subject_reservations", ["sub"], None, None)
					.take(),
				Query::drop_table().table("oidc_op_subjects").take(),
			),
			schema_operation(
				Query::create_table()
					.table("oidc_op_pending")
					.col(ColumnDef::new("id").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.col(ColumnDef::new("expires_at").big_integer().not_null(true))
					.take(),
				Query::drop_table().table("oidc_op_pending").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oidc_op_pending_expiry")
					.table("oidc_op_pending")
					.col("expires_at")
					.take(),
				Query::drop_index().name("oidc_op_pending_expiry").take(),
			),
			schema_operation(
				Query::create_table()
					.table("oidc_op_codes")
					.col(ColumnDef::new("digest").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.col(ColumnDef::new("expires_at").big_integer().not_null(true))
					.take(),
				Query::drop_table().table("oidc_op_codes").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oidc_op_codes_expiry")
					.table("oidc_op_codes")
					.col("expires_at")
					.take(),
				Query::drop_index().name("oidc_op_codes_expiry").take(),
			),
			schema_operation(
				Query::create_table()
					.table("oidc_op_keys")
					.col(ColumnDef::new("kid").text().primary_key(true))
					.col(ColumnDef::new("public").jsonb().not_null(true))
					.col(ColumnDef::new("activated_at").big_integer().not_null(true))
					.col(ColumnDef::new("active").boolean().not_null(true))
					.col(ColumnDef::new("publish_until").big_integer())
					.col(
						ColumnDef::new("compromised")
							.boolean()
							.not_null(true)
							.default(Expr::constant_false().into_simple_expr()),
					)
					.take(),
				Query::drop_table().table("oidc_op_keys").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oidc_op_one_active_key")
					.table("oidc_op_keys")
					.unique()
					.col("active")
					.r#where(Expr::col("active").eq(Expr::constant_true()))
					.take(),
				Query::drop_index().name("oidc_op_one_active_key").take(),
			),
		]
		.into_iter()
		.fold(
			Migration::new("0002_oidc_op", "oidc_op"),
			|migration, operation| migration.add_operation(operation),
		)
	}

	/// Access the underlying pool for host-managed migrations.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}

	/// Delete expired OIDC continuations and code contexts from shared storage.
	/// Run after OAuth expiry maintenance. Code contexts are retained while the
	/// corresponding OAuth code exists so replay can still revoke live tokens.
	pub async fn purge_expired(&self, now: i64) -> Result<u64, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let mut deleted = 0;
		for table in ["oidc_op_pending", "oidc_op_codes"] {
			let mut query = Query::delete();
			query
				.from_table(Alias::new(table))
				.and_where(Expr::col(Alias::new("expires_at").into_iden()).lte(now));
			if table == "oidc_op_codes" {
				let mut codes = Query::select();
				codes
					.column(Alias::new("digest"))
					.from(Alias::new("oauth_server_codes"))
					.and_where(
						Expr::col(("oauth_server_codes", "digest"))
							.eq(Expr::col(("oidc_op_codes", "digest"))),
					);
				query.and_where(Expr::not_exists(codes));
			}
			let (sql, arguments) = prepare(query)?;
			deleted += sqlx::query_with(&sql, arguments)
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

async fn insert_context(
	tx: &mut Transaction<'_, Postgres>,
	context: &OidcCodeContext,
) -> Result<(), String> {
	let (sql, arguments) = prepare(
		Query::insert()
			.into_table(Alias::new("oidc_op_codes"))
			.columns(["digest", "payload", "expires_at"])
			.values(vec![
				context.digest.clone().into(),
				json_value(context)?,
				context.expires_at.into(),
			])?
			.take(),
	)?;
	sqlx::query_with(&sql, arguments)
		.execute(&mut **tx)
		.await
		.map_err(|e| e.to_string())?;
	Ok(())
}

#[async_trait]
impl OidcStateStore for PostgresOidcStore {
	async fn subject_or_insert(&self, user_id: &str, proposed: &str) -> Result<String, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("sub"))
				.from(Alias::new("oidc_op_subjects"))
				.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
				.lock_exclusive()
				.take(),
		)?;
		let existing: Option<(String,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if let Some((subject,)) = existing {
			tx.commit().await.map_err(|e| e.to_string())?;
			return Ok(subject);
		}
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oidc_op_subject_reservations"))
				.columns(["sub"])
				.values(vec![proposed.into()])?
				.on_conflict(OnConflict::column("sub").do_nothing())
				.take(),
		)?;
		let reservation = sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if reservation.rows_affected() != 1 {
			return Err("subject collision".to_owned());
		}
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oidc_op_subjects"))
				.columns(["user_id", "sub"])
				.values(vec![user_id.into(), proposed.into()])?
				.on_conflict(OnConflict::column("user_id").do_nothing())
				.take(),
		)?;
		let inserted = sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let subject = if inserted.rows_affected() == 1 {
			proposed.to_owned()
		} else {
			let (sql, arguments) = prepare(
				Query::select()
					.column(Alias::new("sub"))
					.from(Alias::new("oidc_op_subjects"))
					.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
					.take(),
			)?;
			let (subject,): (String,) = sqlx::query_as_with(&sql, arguments)
				.fetch_one(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
			subject
		};
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(subject)
	}

	async fn subject(&self, user_id: &str) -> Result<Option<String>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("sub"))
				.from(Alias::new("oidc_op_subjects"))
				.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
				.take(),
		)?;
		let row: Option<(String,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(subject,)| subject))
	}

	async fn retire_subject(&self, user_id: &str) -> Result<(), String> {
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oidc_op_subjects"))
				.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn retire_user(&self, user_id: &str) -> Result<(), String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_codes"))
				.value(Alias::new("redeemed"), true)
				.value(Alias::new("replayed"), true)
				.and_where(json_text_path(&["user_id"]).eq(user_id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_tokens"))
				.value(Alias::new("revoked"), true)
				.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oidc_op_codes"))
				.and_where(json_text_path(&["user_id"]).eq(user_id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oidc_op_subjects"))
				.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn put_pending(&self, id: &str, pending: OidcPending) -> Result<(), String> {
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oidc_op_pending"))
				.columns(["id", "payload", "expires_at"])
				.values(vec![
					id.into(),
					json_value(&pending)?,
					pending.expires_at.into(),
				])?
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn pending(&self, id: &str) -> Result<Option<OidcPending>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oidc_op_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
				.take(),
		)?;
		let row: Option<(Json<OidcPending>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(pending),)| pending))
	}
	async fn complete_authorization(
		&self,
		oauth: &dyn OAuthServerStore,
		request: AuthorizationCommit<'_>,
		pending: &OidcPending,
		context: Option<&OidcCodeContext>,
	) -> Result<bool, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let id = &request.pending.request.id;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oidc_op_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id.as_str()))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<OidcPending>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if row.as_ref().map(|(Json(stored),)| stored) != Some(pending)
			|| !authorization_matches(request, pending, context)
		{
			return Ok(false);
		}
		if !oauth
			.complete_pending_in_transaction(&mut tx, request)
			.await?
		{
			return Ok(false);
		}
		if let Some(context) = context {
			insert_context(&mut tx, context).await?;
		}
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oidc_op_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id.as_str()))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(true)
	}
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		now: i64,
	) -> Result<Option<OidcPending>, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oidc_op_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<OidcPending>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let Some((Json(pending),)) = row else {
			return Ok(None);
		};
		if pending.session_digest != session_digest || pending.expires_at <= now {
			return Ok(None);
		}
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oidc_op_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(Some(pending))
	}

	async fn put_code(&self, context: OidcCodeContext) -> Result<(), String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		insert_context(&mut tx, &context).await?;
		tx.commit().await.map_err(|e| e.to_string())
	}

	async fn code(&self, digest: &str) -> Result<Option<OidcCodeContext>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oidc_op_codes"))
				.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
				.take(),
		)?;
		let row: Option<(Json<OidcCodeContext>,)> = sqlx::query_as_with(&sql, arguments)
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
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oidc_op_keys"))
				.value(Alias::new("active"), false)
				.value(Alias::new("publish_until"), now + retention)
				.and_where(Expr::col(Alias::new("active").into_iden()).eq(true))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oidc_op_keys"))
				.columns(["kid", "public", "activated_at", "active", "compromised"])
				.values(vec![
					public.kid.clone().into(),
					json_value(&public)?,
					now.into(),
					true.into(),
					false.into(),
				])?
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}

	async fn active_key(&self) -> Result<Option<OidcKey>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.columns([
					"public",
					"activated_at",
					"active",
					"publish_until",
					"compromised",
				])
				.from(Alias::new("oidc_op_keys"))
				.and_where(Expr::col(Alias::new("active").into_iden()).eq(true))
				.take(),
		)?;
		let row: Option<(Json<PublicRsaJwk>, i64, bool, Option<i64>, bool)> =
			sqlx::query_as_with(&sql, arguments)
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
		let (sql, arguments) = prepare(
			Query::select()
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
				.take(),
		)?;
		let rows: Vec<(Json<PublicRsaJwk>, i64, bool, Option<i64>, bool)> =
			sqlx::query_as_with(&sql, arguments)
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
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oidc_op_keys"))
				.value(Alias::new("active"), false)
				.value(Alias::new("publish_until"), Option::<i64>::None)
				.value(Alias::new("compromised"), true)
				.and_where(Expr::col(Alias::new("kid").into_iden()).eq(kid))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
}
