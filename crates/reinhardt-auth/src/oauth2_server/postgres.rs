//! PostgreSQL state store. The migration is registered with Reinhardt's migration engine.

use super::protocol::TokenPrincipal;
use super::store::{
	ClientRegistration, CodeRedemption, OAuthServerStore, PendingRecord, ResourceRegistration,
	StoredCode, StoredToken,
};
use async_trait::async_trait;
use reinhardt_db::migrations::{Migration, Operation};
use reinhardt_query::prelude::{
	Alias, Expr, ExprTrait, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder,
};
use sqlx::{PgPool, types::Json};

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
}
#[async_trait]
impl OAuthServerStore for PostgresOAuthStore {
	async fn put_client(&self, client: ClientRegistration) -> Result<(), String> {
		sqlx::query("INSERT INTO oauth_server_clients (client_id,payload) VALUES ($1,$2) ON CONFLICT (client_id) DO UPDATE SET payload=excluded.payload")
            .bind(&client.client_id).bind(Json(&client)).execute(&self.pool).await.map_err(|e| e.to_string())?;
		Ok(())
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
		let row: Option<(Json<ClientRegistration>,)> = sqlx::query_as("SELECT payload FROM oauth_server_clients WHERE payload->>'kind'='Public' AND payload->>'enabled'='true' AND payload->'browser_origins' ? $1 LIMIT 1").bind(origin).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(client),)| client))
	}
	async fn put_resource(&self, resource: ResourceRegistration) -> Result<(), String> {
		sqlx::query("INSERT INTO oauth_server_resources (resource_id,payload) VALUES ($1,$2) ON CONFLICT (resource_id) DO UPDATE SET payload=excluded.payload")
            .bind(&resource.resource_id).bind(Json(&resource)).execute(&self.pool).await.map_err(|e| e.to_string())?;
		Ok(())
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
		let row: Option<(Json<ResourceRegistration>,)> = sqlx::query_as(
			"SELECT payload FROM oauth_server_resources WHERE payload->>'audience'=$1",
		)
		.bind(audience)
		.fetch_optional(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(resource),)| resource))
	}
	async fn put_pending(&self, id: &str, pending: PendingRecord) -> Result<(), String> {
		sqlx::query("INSERT INTO oauth_server_pending (id,payload,expires_at) VALUES ($1,$2,$3)")
			.bind(id)
			.bind(Json(&pending))
			.bind(pending.expires_at)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn take_pending(&self, id: &str) -> Result<Option<PendingRecord>, String> {
		let row: Option<(Json<PendingRecord>,)> =
			sqlx::query_as("DELETE FROM oauth_server_pending WHERE id=$1 RETURNING payload")
				.bind(id)
				.fetch_optional(&self.pool)
				.await
				.map_err(|e| e.to_string())?;
		Ok(row.map(|(Json(pending),)| pending))
	}
	async fn put_code(&self, code: StoredCode) -> Result<(), String> {
		sqlx::query(
			"INSERT INTO oauth_server_codes (digest,payload,client_id,expires_at) VALUES ($1,$2,$3,$4)",
		)
		.bind(&code.digest)
		.bind(Json(&code))
		.bind(&code.client_id)
		.bind(code.expires_at)
		.execute(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn redeem_code(&self, digest: &str, now: i64) -> Result<CodeRedemption, String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		let row: Option<(Json<StoredCode>, bool, bool, i64)> = sqlx::query_as(
			"SELECT payload,redeemed,replayed,expires_at FROM oauth_server_codes WHERE digest=$1 FOR UPDATE",
		)
		.bind(digest)
		.fetch_optional(&mut *tx)
		.await
		.map_err(|e| e.to_string())?;
		let result = match row {
			None => CodeRedemption::Invalid,
			Some((_, true, _, _)) => {
				sqlx::query("UPDATE oauth_server_codes SET replayed=TRUE WHERE digest=$1")
					.bind(digest)
					.execute(&mut *tx)
					.await
					.map_err(|e| e.to_string())?;
				sqlx::query("UPDATE oauth_server_tokens SET revoked=TRUE WHERE code_digest=$1")
					.bind(digest)
					.execute(&mut *tx)
					.await
					.map_err(|e| e.to_string())?;
				CodeRedemption::Replay
			}
			Some((_, false, _, expiry)) if expiry <= now => CodeRedemption::Invalid,
			Some((Json(mut code), false, _, _)) => {
				sqlx::query("UPDATE oauth_server_codes SET redeemed=TRUE WHERE digest=$1")
					.bind(digest)
					.execute(&mut *tx)
					.await
					.map_err(|e| e.to_string())?;
				code.redeemed = true;
				CodeRedemption::Valid(code)
			}
		};
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(result)
	}
	async fn put_token(&self, mut token: StoredToken) -> Result<(), String> {
		let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
		if let Some(code_digest) = &token.code_digest {
			let row: Option<(bool,)> = sqlx::query_as(
				"SELECT replayed FROM oauth_server_codes WHERE digest=$1 FOR UPDATE",
			)
			.bind(code_digest)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| e.to_string())?;
			if row.is_none_or(|(replayed,)| replayed) {
				token.revoked = true;
			}
		}
		let user_id = match &token.principal {
			TokenPrincipal::User(id) => Some(id.as_str()),
			TokenPrincipal::Client(_) => None,
		};
		sqlx::query("INSERT INTO oauth_server_tokens (digest,payload,client_id,user_id,code_digest,expires_at,revoked) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(&token.digest).bind(Json(&token)).bind(&token.client_id).bind(user_id).bind(&token.code_digest).bind(token.expires_at).bind(token.revoked)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
		tx.commit().await.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn token(&self, digest: &str) -> Result<Option<StoredToken>, String> {
		let row: Option<(Json<StoredToken>, bool)> =
			sqlx::query_as("SELECT payload,revoked FROM oauth_server_tokens WHERE digest=$1")
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
		sqlx::query("UPDATE oauth_server_tokens SET revoked=TRUE WHERE digest=$1 AND client_id=$2")
			.bind(digest)
			.bind(client_id)
			.execute(&self.pool)
			.await
			.map_err(|e| e.to_string())?;
		Ok(())
	}
	async fn revoke_user(&self, user_id: &str) -> Result<u64, String> {
		let result = sqlx::query(
			"UPDATE oauth_server_tokens SET revoked=TRUE WHERE user_id=$1 AND revoked=FALSE",
		)
		.bind(user_id)
		.execute(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(result.rows_affected())
	}
	async fn revoke_client(&self, client_id: &str) -> Result<u64, String> {
		let result = sqlx::query(
			"UPDATE oauth_server_tokens SET revoked=TRUE WHERE client_id=$1 AND revoked=FALSE",
		)
		.bind(client_id)
		.execute(&self.pool)
		.await
		.map_err(|e| e.to_string())?;
		Ok(result.rows_affected())
	}
}
