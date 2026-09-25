//! PostgreSQL state store. The migration is registered with Reinhardt's migration engine.

use super::protocol::TokenPrincipal;
use super::store::{
	AuthorizationCommit, ClientRegistration, CodeInspection, CodeRedemption, CodeRedemptionRequest,
	OAuthServerStore, PendingRecord, ResourceRegistration, StoredCode, StoredToken,
	token_matches_code,
};
use async_trait::async_trait;
use reinhardt_db::migrations::{Migration, Operation};
use reinhardt_query::prelude::{
	Alias, Expr, ExprTrait, IntoIden, OnConflict, PostgresQueryBuilder, Query,
	QueryStatementBuilder, Value,
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
		// Reinhardt's PostgreSQL schema editor prepares each RunSQL operation.
		// Keep one statement per operation; prepared statements reject SQL batches.
		const REVERSE: [&str; 12] = [
			"DROP TABLE oauth_server_clients",
			"DROP TABLE oauth_server_resources",
			"DROP INDEX oauth_server_resource_audience",
			"DROP TABLE oauth_server_pending",
			"DROP INDEX oauth_server_pending_expiry",
			"DROP TABLE oauth_server_codes",
			"DROP INDEX oauth_server_codes_expiry",
			"DROP TABLE oauth_server_tokens",
			"DROP INDEX oauth_server_tokens_client",
			"DROP INDEX oauth_server_tokens_user",
			"DROP INDEX oauth_server_tokens_expiry",
			"DROP INDEX oauth_server_tokens_code",
		];
		let statements: Vec<_> = include_str!("../../migrations/0001_oauth_server.sql")
			.split(';')
			.map(str::trim)
			.filter(|sql| !sql.is_empty())
			.collect();
		assert_eq!(
			statements.len(),
			REVERSE.len(),
			"OAuth migration and reverse statements must match"
		);
		statements.into_iter().zip(REVERSE).fold(
			Migration::new("0001_oauth_server", "oauth_server"),
			|migration, (sql, reverse_sql)| {
				migration.add_operation(Operation::RunSQL {
					sql: sql.to_owned(),
					reverse_sql: Some(reverse_sql.to_owned()),
				})
			},
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
			let (sql, _) = query.build(PostgresQueryBuilder);
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

fn update_flag(table: &'static str, field: &'static str, key: &'static str, value: &str) -> String {
	Query::update()
		.table(Alias::new(table))
		.value_expr(Alias::new(field), Expr::cust("TRUE"))
		.and_where(Expr::col(Alias::new(key).into_iden()).eq(value))
		.build(PostgresQueryBuilder)
		.0
}

async fn insert_token(
	tx: &mut Transaction<'_, Postgres>,
	token: &StoredToken,
) -> Result<(), String> {
	let user_id = match &token.principal {
		TokenPrincipal::User(id) => Some(id.as_str()),
		TokenPrincipal::Client(_) => None,
	};
	let (sql, _) = Query::insert()
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
		.build(PostgresQueryBuilder);
	let mut query = sqlx::query(&sql)
		.bind(&token.digest)
		.bind(Json(token))
		.bind(&token.client_id);
	if let Some(user_id) = user_id {
		query = query.bind(user_id);
	}
	if let Some(code_digest) = &token.code_digest {
		query = query.bind(code_digest);
	}
	query
		.bind(token.expires_at)
		.bind(token.revoked)
		.execute(&mut **tx)
		.await
		.map_err(|e| e.to_string())?;
	Ok(())
}
async fn insert_code(tx: &mut Transaction<'_, Postgres>, code: &StoredCode) -> Result<(), String> {
	let (sql, _) = Query::insert()
		.into_table(Alias::new("oauth_server_codes"))
		.columns(["digest", "payload", "client_id", "expires_at"])
		.values(vec![
			code.digest.clone().into(),
			json_value(code)?,
			code.client_id.clone().into(),
			code.expires_at.into(),
		])?
		.build(PostgresQueryBuilder);
	sqlx::query(&sql)
		.bind(&code.digest)
		.bind(Json(code))
		.bind(&code.client_id)
		.bind(code.expires_at)
		.execute(&mut **tx)
		.await
		.map_err(|e| e.to_string())?;
	Ok(())
}

#[async_trait]
impl OAuthServerStore for PostgresOAuthStore {
	async fn put_client(&self, client: ClientRegistration) -> Result<(), String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oauth_server_clients"))
			.columns(["client_id", "payload"])
			.values(vec![client.client_id.clone().into(), json_value(&client)?])?
			.on_conflict(OnConflict::column("client_id").update_columns(["payload"]))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(&client.client_id)
			.bind(Json(&client))
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn insert_client_if_absent(&self, client: ClientRegistration) -> Result<bool, String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oauth_server_clients"))
			.columns(["client_id", "payload"])
			.values(vec![client.client_id.clone().into(), json_value(&client)?])?
			.on_conflict(OnConflict::column("client_id").do_nothing())
			.build(PostgresQueryBuilder);
		let result = sqlx::query(&sql)
			.bind(&client.client_id)
			.bind(Json(&client))
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
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_clients"))
			.and_where(
				Expr::col(Alias::new("client_id").into_iden()).eq(expected.client_id.as_str()),
			)
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as(&sql)
			.bind(&expected.client_id)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if row.as_ref().map(|(Json(client),)| client) != Some(expected) {
			return Ok(false);
		}
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_clients"))
			.value(Alias::new("payload"), json_value(&replacement)?)
			.and_where(
				Expr::col(Alias::new("client_id").into_iden()).eq(expected.client_id.as_str()),
			)
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(Json(&replacement))
			.bind(&expected.client_id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(true)
	}
	async fn disable_client(&self, id: &str) -> Result<Option<u64>, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_clients"))
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as(&sql)
			.bind(id)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let Some((Json(mut client),)) = row else {
			return Ok(None);
		};
		client.enabled = false;
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_clients"))
			.value(Alias::new("payload"), json_value(&client)?)
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(Json(&client))
			.bind(id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oauth_server_pending"))
			.and_where(Expr::cust_with_values(
				"payload->'request'->>'client_id' = ?",
				[id],
			))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_codes"))
			.value_expr(Alias::new("redeemed"), Expr::cust("TRUE"))
			.value_expr(Alias::new("replayed"), Expr::cust("TRUE"))
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_tokens"))
			.value_expr(Alias::new("revoked"), Expr::cust("TRUE"))
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
			.and_where(Expr::cust("NOT revoked"))
			.build(PostgresQueryBuilder);
		let result = sqlx::query(&sql)
			.bind(id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(Some(result.rows_affected()))
	}
	async fn client(&self, id: &str) -> Result<Option<ClientRegistration>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_clients"))
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(id))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as(&sql)
			.bind(id)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(client),)| client))
	}
	async fn public_client_for_origin(
		&self,
		origin: &str,
	) -> Result<Option<ClientRegistration>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_clients"))
			.and_where(Expr::cust("payload->>'kind' = 'Public'"))
			.and_where(Expr::cust("payload->>'enabled' = 'true'"))
			.and_where(Expr::cust_with_values(
				"payload->'browser_origins' @> ?",
				[origin],
			))
			.limit(1)
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as(&sql)
			.bind(Json(vec![origin]))
			.bind(1_i64)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(client),)| client))
	}
	async fn put_resource(&self, resource: ResourceRegistration) -> Result<(), String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oauth_server_resources"))
			.columns(["resource_id", "payload"])
			.values(vec![
				resource.resource_id.clone().into(),
				json_value(&resource)?,
			])?
			.on_conflict(OnConflict::column("resource_id").update_columns(["payload"]))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(&resource.resource_id)
			.bind(Json(&resource))
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn insert_resource_if_absent(
		&self,
		resource: ResourceRegistration,
	) -> Result<bool, String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oauth_server_resources"))
			.columns(["resource_id", "payload"])
			.values(vec![
				resource.resource_id.clone().into(),
				json_value(&resource)?,
			])?
			.on_conflict(OnConflict::column("resource_id").do_nothing())
			.build(PostgresQueryBuilder);
		let result = sqlx::query(&sql)
			.bind(&resource.resource_id)
			.bind(Json(&resource))
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
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_resources"))
			.and_where(
				Expr::col(Alias::new("resource_id").into_iden()).eq(expected.resource_id.as_str()),
			)
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as(&sql)
			.bind(&expected.resource_id)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		if row.as_ref().map(|(Json(client),)| client) != Some(expected) {
			return Ok(false);
		}
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_resources"))
			.value(Alias::new("payload"), json_value(&replacement)?)
			.and_where(
				Expr::col(Alias::new("resource_id").into_iden()).eq(expected.resource_id.as_str()),
			)
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(Json(&replacement))
			.bind(&expected.resource_id)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(true)
	}
	async fn resource(&self, id: &str) -> Result<Option<ResourceRegistration>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_resources"))
			.and_where(Expr::col(Alias::new("resource_id").into_iden()).eq(id))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as(&sql)
			.bind(id)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(resource),)| resource))
	}
	async fn resource_for_audience(
		&self,
		audience: &str,
	) -> Result<Option<ResourceRegistration>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_resources"))
			.and_where(Expr::cust_with_values(
				"payload->>'audience' = ?",
				[audience],
			))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as(&sql)
			.bind(audience)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(resource),)| resource))
	}
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String> {
		let (sql, _) = Query::insert()
			.into_table(Alias::new("oauth_server_pending"))
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
	async fn pending(&self, id: &str) -> Result<Option<PendingRecord>, String> {
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<PendingRecord>,)> = sqlx::query_as(&sql)
			.bind(id)
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
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id.as_str()))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<PendingRecord>,)> = sqlx::query_as(&sql)
			.bind(id)
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
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oauth_server_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id.as_str()))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(id)
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
		let (select_sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<PendingRecord>,)> = sqlx::query_as(&select_sql)
			.bind(id)
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
		let (sql, _) = Query::delete()
			.from_table(Alias::new("oauth_server_pending"))
			.and_where(Expr::col(Alias::new("id").into_iden()).eq(id))
			.returning(["payload"])
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(id)
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
		let (sql, _) = Query::select()
			.column(Alias::new("payload"))
			.from(Alias::new("oauth_server_codes"))
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<StoredCode>,)> = sqlx::query_as(&sql)
			.bind(digest)
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
		let (sql, _) = Query::select()
			.columns(["payload", "redeemed", "expires_at"])
			.from(Alias::new("oauth_server_codes"))
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(request.digest))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<StoredCode>, bool, i64)> = sqlx::query_as(&sql)
			.bind(request.digest)
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
			sqlx::query(&update_flag(
				"oauth_server_codes",
				"replayed",
				"digest",
				request.digest,
			))
			.bind(request.digest)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
			sqlx::query(&update_flag(
				"oauth_server_tokens",
				"revoked",
				"code_digest",
				request.digest,
			))
			.bind(request.digest)
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
		let (sql, _) = Query::select()
			.columns(["payload", "redeemed", "expires_at"])
			.from(Alias::new("oauth_server_codes"))
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
			.lock_exclusive()
			.build(PostgresQueryBuilder);
		let row: Option<(Json<StoredCode>, bool, i64)> = sqlx::query_as(&sql)
			.bind(digest)
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
			sqlx::query(&update_flag(
				"oauth_server_codes",
				"replayed",
				"digest",
				digest,
			))
			.bind(digest)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
			sqlx::query(&update_flag(
				"oauth_server_tokens",
				"revoked",
				"code_digest",
				digest,
			))
			.bind(digest)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
			tx.commit().await.map_err(|e| e.to_string())?;
			return Ok(CodeRedemption::Replay);
		}
		if expiry <= now || !token_matches_code(&token, &code) {
			return Ok(CodeRedemption::Invalid);
		}
		sqlx::query(&update_flag(
			"oauth_server_codes",
			"redeemed",
			"digest",
			digest,
		))
		.bind(digest)
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
		let (sql, _) = Query::select()
			.columns(["payload", "revoked"])
			.from(Alias::new("oauth_server_tokens"))
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
			.build(PostgresQueryBuilder);
		let row: Option<(Json<StoredToken>, bool)> = sqlx::query_as(&sql)
			.bind(digest)
			.fetch_optional(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(mut token), revoked)| {
			token.revoked = revoked;
			token
		}))
	}
	async fn revoke_token(&self, digest: &str, client_id: &str) -> Result<(), String> {
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_tokens"))
			.value(Alias::new("revoked"), true)
			.and_where(Expr::col(Alias::new("digest").into_iden()).eq(digest))
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(client_id))
			.build(PostgresQueryBuilder);
		sqlx::query(&sql)
			.bind(true)
			.bind(digest)
			.bind(client_id)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String> {
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
			.value(Alias::new("revoked"), true)
			.and_where(Expr::col(Alias::new("user_id").into_iden()).eq(user_id))
			.and_where(Expr::col(Alias::new("revoked").into_iden()).eq(false))
			.build(PostgresQueryBuilder);
		let result = sqlx::query(&sql)
			.bind(true)
			.bind(user_id)
			.bind(false)
			.execute(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(result.rows_affected())
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
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String> {
		let (sql, _) = Query::update()
			.table(Alias::new("oauth_server_tokens"))
			.value(Alias::new("revoked"), true)
			.and_where(Expr::col(Alias::new("client_id").into_iden()).eq(client_id))
			.and_where(Expr::col(Alias::new("revoked").into_iden()).eq(false))
			.build(PostgresQueryBuilder);
		let result = sqlx::query(&sql)
			.bind(true)
			.bind(client_id)
			.bind(false)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(result.rows_affected())
	}
}
