//! PostgreSQL state store. The migration is registered with Reinhardt's migration engine.

use super::protocol::TokenPrincipal;
use super::store::{
	AuthorizationCommit, ClientRegistration, CodeInspection, CodeRedemption, CodeRedemptionRequest,
	OAuthServerStore, PendingRecord, ResourceRegistration, StoredCode, StoredToken,
	token_matches_code,
};
use crate::database_query::{json_text_path, prepare, schema_operation};
use async_trait::async_trait;
use reinhardt_db::migrations::Migration;
use reinhardt_query::prelude::{
	Alias, ColumnDef, Expr, ExprTrait, IntoIden, OnConflict, Query, Value,
};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction, types::Json};

/// OAuth state store shared by PostgreSQL-backed server instances.
#[derive(Clone)]
pub struct PostgresOAuthStore {
	pool: PgPool,
}
impl PostgresOAuthStore {
	/// Use a PostgreSQL pool whose schema has been migrated.
	pub fn new(pool: PgPool) -> Self {
		Self { pool }
	}
	/// Return the initial PostgreSQL migration for the host's migration graph.
	pub fn migration() -> Migration {
		// Keep the published identity and one statement per operation: PostgreSQL
		// prepares RunSQL operations individually, and rollback runs in reverse order.
		[
			schema_operation(
				Query::create_table()
					.table("oauth_server_clients")
					.col(ColumnDef::new("client_id").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.take(),
				Query::drop_table().table("oauth_server_clients").take(),
			),
			schema_operation(
				Query::create_table()
					.table("oauth_server_resources")
					.col(ColumnDef::new("resource_id").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.take(),
				Query::drop_table().table("oauth_server_resources").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_resource_audience")
					.table("oauth_server_resources")
					.unique()
					.expr(json_text_path(&["audience"]))
					.take(),
				Query::drop_index()
					.name("oauth_server_resource_audience")
					.take(),
			),
			schema_operation(
				Query::create_table()
					.table("oauth_server_pending")
					.col(ColumnDef::new("id").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.col(ColumnDef::new("expires_at").big_integer().not_null(true))
					.take(),
				Query::drop_table().table("oauth_server_pending").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_pending_expiry")
					.table("oauth_server_pending")
					.col("expires_at")
					.take(),
				Query::drop_index()
					.name("oauth_server_pending_expiry")
					.take(),
			),
			schema_operation(
				Query::create_table()
					.table("oauth_server_codes")
					.col(ColumnDef::new("digest").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.col(ColumnDef::new("client_id").text().not_null(true))
					.col(ColumnDef::new("expires_at").big_integer().not_null(true))
					.col(
						ColumnDef::new("redeemed")
							.boolean()
							.not_null(true)
							.default(Expr::constant_false().into_simple_expr()),
					)
					.col(
						ColumnDef::new("replayed")
							.boolean()
							.not_null(true)
							.default(Expr::constant_false().into_simple_expr()),
					)
					.take(),
				Query::drop_table().table("oauth_server_codes").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_codes_expiry")
					.table("oauth_server_codes")
					.col("expires_at")
					.take(),
				Query::drop_index().name("oauth_server_codes_expiry").take(),
			),
			schema_operation(
				Query::create_table()
					.table("oauth_server_tokens")
					.col(ColumnDef::new("digest").text().primary_key(true))
					.col(ColumnDef::new("payload").jsonb().not_null(true))
					.col(ColumnDef::new("client_id").text().not_null(true))
					.col(ColumnDef::new("user_id").text())
					.col(ColumnDef::new("code_digest").text())
					.col(ColumnDef::new("expires_at").big_integer().not_null(true))
					.col(
						ColumnDef::new("revoked")
							.boolean()
							.not_null(true)
							.default(Expr::constant_false().into_simple_expr()),
					)
					.foreign_key(
						["code_digest"],
						"oauth_server_codes",
						["digest"],
						None,
						None,
					)
					.take(),
				Query::drop_table().table("oauth_server_tokens").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_tokens_client")
					.table("oauth_server_tokens")
					.col("client_id")
					.take(),
				Query::drop_index()
					.name("oauth_server_tokens_client")
					.take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_tokens_user")
					.table("oauth_server_tokens")
					.col("user_id")
					.r#where(Expr::col("user_id").is_not_null())
					.take(),
				Query::drop_index().name("oauth_server_tokens_user").take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_tokens_expiry")
					.table("oauth_server_tokens")
					.col("expires_at")
					.take(),
				Query::drop_index()
					.name("oauth_server_tokens_expiry")
					.take(),
			),
			schema_operation(
				Query::create_index()
					.name("oauth_server_tokens_code")
					.table("oauth_server_tokens")
					.col("code_digest")
					.r#where(Expr::col("code_digest").is_not_null())
					.take(),
				Query::drop_index().name("oauth_server_tokens_code").take(),
			),
		]
		.into_iter()
		.fold(
			Migration::new("0001_oauth_server", "oauth_server"),
			|migration, operation| migration.add_operation(operation),
		)
	}

	/// Access the underlying pool for host-managed migrations and operations.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}
	/// Delete expired pending requests, codes, and tokens in one transaction.
	/// Run this periodically from a host maintenance job.
	pub async fn purge_expired(&self, now: i64) -> Result<u64, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let mut deleted = 0;
		for table in [
			"oauth_server_pending",
			"oauth_server_tokens",
			"oauth_server_codes",
		] {
			let mut query = Query::delete();
			query
				.from_table(Alias::new(table))
				.and_where(Expr::col(Alias::new("expires_at").into_iden()).lte(now));
			if table == "oauth_server_codes" {
				let mut subquery = Query::select();
				subquery
					.column(Alias::new("digest"))
					.from(Alias::new("oauth_server_tokens"))
					.and_where(
						Expr::col(("oauth_server_tokens", "code_digest"))
							.eq(Expr::col(("oauth_server_codes", "digest"))),
					);
				query.and_where(Expr::not_exists(subquery));
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

fn update_flag(
	table: &'static str,
	field: &'static str,
	key: &'static str,
	value: &str,
) -> Result<crate::database_query::Prepared, String> {
	prepare(
		Query::update()
			.table(Alias::new(table))
			.value(Alias::new(field), true)
			.and_where(Expr::col(Alias::new(key).into_iden()).eq(value))
			.take(),
	)
}

async fn insert_token(
	tx: &mut Transaction<'_, Postgres>,
	token: &StoredToken,
) -> Result<(), String> {
	let user_id = match &token.principal {
		TokenPrincipal::User(id) => Some(id.as_str()),
		TokenPrincipal::Client(_) => None,
	};
	let (sql, arguments) = prepare(
		Query::insert()
			.into_table(Alias::new("oauth_server_tokens"))
			.columns([
				"digest",
				"payload",
				"client_id",
				"user_id",
				"code_digest",
				"expires_at",
				"revoked",
			])
			.values(vec![
				token.digest.clone().into(),
				json_value(token)?,
				token.client_id.clone().into(),
				Value::String(user_id.map(|id| Box::new(id.to_owned()))),
				Value::String(token.code_digest.clone().map(Box::new)),
				token.expires_at.into(),
				token.revoked.into(),
			])?
			.take(),
	)?;
	sqlx::query_with(&sql, arguments)
		.execute(&mut **tx)
		.await
		.map_err(|e| e.to_string())?;
	Ok(())
}
async fn insert_code(tx: &mut Transaction<'_, Postgres>, code: &StoredCode) -> Result<(), String> {
	let (sql, arguments) = prepare(
		Query::insert()
			.into_table(Alias::new("oauth_server_codes"))
			.columns(["digest", "payload", "client_id", "expires_at"])
			.values(vec![
				code.digest.clone().into(),
				json_value(code)?,
				code.client_id.clone().into(),
				code.expires_at.into(),
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
impl OAuthServerStore for PostgresOAuthStore {
	async fn put_client(&self, client: ClientRegistration) -> Result<(), String> {
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oauth_server_clients"))
				.columns(["client_id", "payload"])
				.values(vec![client.client_id.clone().into(), json_value(&client)?])?
				.on_conflict(OnConflict::column("client_id").update_columns(["payload"]))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn insert_client_if_absent(&self, client: ClientRegistration) -> Result<bool, String> {
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oauth_server_clients"))
				.columns(["client_id", "payload"])
				.values(vec![client.client_id.clone().into(), json_value(&client)?])?
				.on_conflict(OnConflict::column("client_id").do_nothing())
				.take(),
		)?;
		let result = sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(result.rows_affected() == 1)
	}
	async fn compare_and_swap_client(
		&self,
		expected: &ClientRegistration,
		replacement: ClientRegistration,
	) -> Result<bool, String> {
		if replacement.client_id != expected.client_id {
			return Err("client identity cannot change".to_owned());
		}
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_clients"))
				.and_where(
					Expr::col(Alias::new("client_id").into_iden()).eq(expected.client_id.as_str()),
				)
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if row.as_ref().map(|(Json(client),)| client) != Some(expected) {
			return Ok(false);
		}
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_clients"))
				.value(Alias::new("payload"), json_value(&replacement)?)
				.and_where(
					Expr::col(Alias::new("client_id").into_iden()).eq(expected.client_id.as_str()),
				)
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(true)
	}
	async fn disable_client(&self, id: &str) -> Result<Option<u64>, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_clients"))
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let Some((Json(mut client),)) = row else {
			return Ok(None);
		};
		client.enabled = false;
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_clients"))
				.value(Alias::new("payload"), json_value(&client)?)
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oauth_server_pending"))
				.and_where(json_text_path(&["request", "client_id"]).eq(id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_codes"))
				.value(Alias::new("redeemed"), true)
				.value(Alias::new("replayed"), true)
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
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
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
				.and_where(Expr::col("revoked").not())
				.take(),
		)?;
		let result = sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(Some(result.rows_affected()))
	}
	async fn client(&self, id: &str) -> Result<Option<ClientRegistration>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_clients"))
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
				.take(),
		)?;
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(client),)| client))
	}
	async fn public_client_for_origin(
		&self,
		origin: &str,
	) -> Result<Option<ClientRegistration>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_clients"))
				.and_where(json_text_path(&["kind"]).eq("Public"))
				.and_where(json_text_path(&["enabled"]).eq("true"))
				.and_where(reinhardt_query::SimpleExpr::Binary(
					Box::new(Expr::col("payload").binary(
						reinhardt_query::BinOper::PgOperator(
							reinhardt_query::types::PgBinOper::JsonGetByIndex,
						),
						Expr::val("browser_origins").into_simple_expr(),
					)),
					reinhardt_query::BinOper::PgOperator(
						reinhardt_query::types::PgBinOper::Contains,
					),
					Box::new(Expr::val(serde_json::json!([origin])).into_simple_expr()),
				))
				.limit(1)
				.take(),
		)?;
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(client),)| client))
	}
	async fn put_resource(&self, resource: ResourceRegistration) -> Result<(), String> {
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oauth_server_resources"))
				.columns(["resource_id", "payload"])
				.values(vec![
					resource.resource_id.clone().into(),
					json_value(&resource)?,
				])?
				.on_conflict(OnConflict::column("resource_id").update_columns(["payload"]))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn insert_resource_if_absent(
		&self,
		resource: ResourceRegistration,
	) -> Result<bool, String> {
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oauth_server_resources"))
				.columns(["resource_id", "payload"])
				.values(vec![
					resource.resource_id.clone().into(),
					json_value(&resource)?,
				])?
				.on_conflict(OnConflict::column("resource_id").do_nothing())
				.take(),
		)?;
		let result = sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(result.rows_affected() == 1)
	}
	async fn compare_and_swap_resource(
		&self,
		expected: &ResourceRegistration,
		replacement: ResourceRegistration,
	) -> Result<bool, String> {
		if replacement.resource_id != expected.resource_id
			|| replacement.audience != expected.audience
		{
			return Err("resource identity cannot change".to_owned());
		}
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_resources"))
				.and_where(
					Expr::col(Alias::new("resource_id").into_iden())
						.eq(expected.resource_id.as_str()),
				)
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if row.as_ref().map(|(Json(client),)| client) != Some(expected) {
			return Ok(false);
		}
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_resources"))
				.value(Alias::new("payload"), json_value(&replacement)?)
				.and_where(
					Expr::col(Alias::new("resource_id").into_iden())
						.eq(expected.resource_id.as_str()),
				)
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(true)
	}
	async fn resource(&self, id: &str) -> Result<Option<ResourceRegistration>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_resources"))
				.and_where(Expr::col(Alias::new("resource_id").into_iden()).eq(id))
				.take(),
		)?;
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(resource),)| resource))
	}
	async fn resource_for_audience(
		&self,
		audience: &str,
	) -> Result<Option<ResourceRegistration>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_resources"))
				.and_where(json_text_path(&["audience"]).eq(audience))
				.take(),
		)?;
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(resource),)| resource))
	}
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String> {
		let (sql, arguments) = prepare(
			Query::insert()
				.into_table(Alias::new("oauth_server_pending"))
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
	async fn pending(&self, id: &str) -> Result<Option<PendingRecord>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
				.take(),
		)?;
		let row: Option<(Json<PendingRecord>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(pending),)| pending))
	}
	async fn complete_pending(&self, request: AuthorizationCommit<'_>) -> Result<bool, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let committed = self
			.complete_pending_in_transaction(&mut tx, request)
			.await?;
		if committed {
			tx.commit().await.map_err(|e| e.to_string())?;
		}
		Ok(committed)
	}
	async fn complete_pending_in_transaction(
		&self,
		tx: &mut Transaction<'_, Postgres>,
		request: AuthorizationCommit<'_>,
	) -> Result<bool, String> {
		let id = &request.pending.request.id;
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id.as_str()))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<PendingRecord>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut **tx)
			.await
			.map_err(|e| e.to_string())?;
		if row.as_ref().map(|(Json(pending),)| pending) != Some(request.pending)
			|| !request.is_valid()
		{
			return Ok(false);
		}
		if let Some(code) = request.code {
			insert_code(tx, code).await?;
		}
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oauth_server_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id.as_str()))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut **tx)
			.await
			.map_err(|e| e.to_string())?;
		Ok(true)
	}
	async fn take_pending(
		&self,
		id: &str,
		session_digest: &str,
		oidc: bool,
		now: i64,
	) -> Result<Option<PendingRecord>, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (select_sql, select_arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<PendingRecord>,)> =
			sqlx::query_as_with(&select_sql, select_arguments)
				.fetch_optional(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
		let Some((Json(pending),)) = row else {
			return Ok(None);
		};
		if pending.session_digest != session_digest
			|| pending.oidc != oidc
			|| pending.expires_at <= now
		{
			return Ok(None);
		}
		let (sql, arguments) = prepare(
			Query::delete()
				.from_table(Alias::new("oauth_server_pending"))
				.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
				.returning(["payload"])
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(Some(pending))
	}
	async fn put_code(&self, code: StoredCode) -> Result<(), String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		insert_code(&mut tx, &code).await?;
		tx.commit().await.map_err(|e| e.to_string())
	}
	async fn code(&self, digest: &str) -> Result<Option<StoredCode>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.column(Alias::new("payload"))
				.from(Alias::new("oauth_server_codes"))
				.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
				.take(),
		)?;
		let row: Option<(Json<StoredCode>,)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(code),)| code))
	}
	async fn inspect_code_for_exchange(
		&self,
		request: CodeInspection<'_>,
	) -> Result<Option<StoredCode>, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.columns(["payload", "redeemed", "expires_at"])
				.from(Alias::new("oauth_server_codes"))
				.and_where(Expr::col(Alias::new("digest").into_iden()).eq(request.digest))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<StoredCode>, bool, i64)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let Some((Json(code), redeemed, expiry)) = row else {
			return Ok(None);
		};
		if !request.matches(&code) {
			return Ok(None);
		}
		if redeemed {
			let (sql, arguments) =
				update_flag("oauth_server_codes", "replayed", "digest", request.digest)?;
			sqlx::query_with(&sql, arguments)
				.execute(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
			let (sql, arguments) = update_flag(
				"oauth_server_tokens",
				"revoked",
				"code_digest",
				request.digest,
			)?;
			sqlx::query_with(&sql, arguments)
				.execute(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
			tx.commit().await.map_err(|e| e.to_string())?;
			return Ok(None);
		}
		Ok((expiry > request.now).then_some(code))
	}
	async fn redeem_code_and_store_token(
		&self,
		request: CodeRedemptionRequest<'_>,
	) -> Result<CodeRedemption, String> {
		let CodeRedemptionRequest {
			digest,
			client_id,
			redirect_uri,
			challenge,
			resource,
			expect_oidc,
			now,
			token,
		} = request;
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, arguments) = prepare(
			Query::select()
				.columns(["payload", "redeemed", "expires_at"])
				.from(Alias::new("oauth_server_codes"))
				.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
				.lock_exclusive()
				.take(),
		)?;
		let row: Option<(Json<StoredCode>, bool, i64)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let Some((Json(mut code), redeemed, expiry)) = row else {
			return Ok(CodeRedemption::Invalid);
		};
		if code.client_id != client_id
			|| code.redirect_uri != redirect_uri
			|| code.challenge != challenge
			|| resource.is_some_and(|r| r != code.audience)
			|| code.oidc != expect_oidc
		{
			return Ok(CodeRedemption::Invalid);
		}
		if redeemed {
			let (sql, arguments) = update_flag("oauth_server_codes", "replayed", "digest", digest)?;
			sqlx::query_with(&sql, arguments)
				.execute(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
			let (sql, arguments) =
				update_flag("oauth_server_tokens", "revoked", "code_digest", digest)?;
			sqlx::query_with(&sql, arguments)
				.execute(&mut *tx)
				.await
				.map_err(|e| e.to_string())?;
			tx.commit().await.map_err(|e| e.to_string())?;
			return Ok(CodeRedemption::Replay);
		}
		if expiry <= now || !token_matches_code(&token, &code) {
			return Ok(CodeRedemption::Invalid);
		}
		let (sql, arguments) = update_flag("oauth_server_codes", "redeemed", "digest", digest)?;
		sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		code.redeemed = true;
		insert_token(&mut tx, &token).await?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(CodeRedemption::Valid(code))
	}
	async fn put_token(&self, token: StoredToken) -> Result<(), String> {
		if token.code_digest.is_some() {
			return Err("code-linked tokens require atomic redemption".to_owned());
		}
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		insert_token(&mut tx, &token).await?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn token(&self, digest: &str) -> Result<Option<StoredToken>, String> {
		let (sql, arguments) = prepare(
			Query::select()
				.columns(["payload", "revoked"])
				.from(Alias::new("oauth_server_tokens"))
				.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
				.take(),
		)?;
		let row: Option<(Json<StoredToken>, bool)> = sqlx::query_as_with(&sql, arguments)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(mut token), revoked)| {
			token.revoked = revoked;
			token
		}))
	}
	async fn revoke_token(&self, digest: &str, client_id: &str) -> Result<(), String> {
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_tokens"))
				.value(Alias::new("revoked"), true)
				.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(client_id))
				.take(),
		)?;
		sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String> {
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
				.and_where(Expr::col(Alias::new("revoked").into_iden()).eq(false))
				.take(),
		)?;
		let result = sqlx::query_with(&sql, arguments)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(result.rows_affected())
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
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String> {
		let (sql, arguments) = prepare(
			Query::update()
				.table(Alias::new("oauth_server_tokens"))
				.value(Alias::new("revoked"), true)
				.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(client_id))
				.and_where(Expr::col(Alias::new("revoked").into_iden()).eq(false))
				.take(),
		)?;
		let result = sqlx::query_with(&sql, arguments)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(result.rows_affected())
	}
}
