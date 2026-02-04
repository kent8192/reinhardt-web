//! User and Profile factory for examples-twitter tests.
//!
//! Provides factory functions for creating Users and Profiles in the database
//! with proper password hashing and relationships.

use chrono::Utc;
use reinhardt::{Argon2Hasher, PasswordHasher};
use rstest::*;
use sea_query::{Expr, ExprTrait, PostgresQueryBuilder, Query};
use sqlx::PgPool;
use uuid::Uuid;

use crate::apps::auth::models::User;
use crate::apps::profile::models::Profile;
use crate::test_utils::fixtures::users::TestTwitterUser;

/// Factory for creating User records in the database.
///
/// Uses SeaQuery for SQL construction and Argon2 for password hashing.
#[derive(Clone)]
pub struct UserFactory {
	hasher: Argon2Hasher,
}

impl Default for UserFactory {
	fn default() -> Self {
		Self::new()
	}
}

impl UserFactory {
	/// Create a new UserFactory with default hasher.
	pub fn new() -> Self {
		Self {
			hasher: Argon2Hasher,
		}
	}

	/// Create a user in the database from TestTwitterUser data.
	///
	/// Hashes the password and inserts the user record.
	pub async fn create_from_test_user(
		&self,
		pool: &PgPool,
		test_user: &TestTwitterUser,
	) -> Result<User, sqlx::Error> {
		let password_hash = self
			.hasher
			.hash(&test_user.password)
			.expect("Password hashing should not fail");

		let sql = Query::insert()
			.into_table(sea_query::Alias::new("auth_user"))
			.columns([
				sea_query::Alias::new("id"),
				sea_query::Alias::new("username"),
				sea_query::Alias::new("email"),
				sea_query::Alias::new("password_hash"),
				sea_query::Alias::new("is_active"),
				sea_query::Alias::new("created_at"),
				sea_query::Alias::new("bio"),
			])
			.values_panic([
				test_user.id.into(),
				test_user.username.clone().into(),
				test_user.email.clone().into(),
				password_hash.into(),
				test_user.is_active.into(),
				Utc::now().into(),
				test_user.bio.clone().into(),
			])
			.to_string(PostgresQueryBuilder);

		sqlx::query(&sql).execute(pool).await?;

		// Return the created user by fetching it
		self.find_by_id(pool, test_user.id).await
	}

	/// Create a user with custom data.
	pub async fn create(
		&self,
		pool: &PgPool,
		username: &str,
		email: &str,
		password: &str,
	) -> Result<User, sqlx::Error> {
		let test_user = TestTwitterUser::new(username)
			.with_email(email)
			.with_password(password);

		self.create_from_test_user(pool, &test_user).await
	}

	/// Create a user with default values.
	pub async fn create_default(&self, pool: &PgPool) -> Result<User, sqlx::Error> {
		self.create_from_test_user(pool, &TestTwitterUser::default())
			.await
	}

	/// Find a user by ID.
	pub async fn find_by_id(&self, pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
		let sql = Query::select()
			.columns([
				sea_query::Alias::new("id"),
				sea_query::Alias::new("username"),
				sea_query::Alias::new("email"),
				sea_query::Alias::new("password_hash"),
				sea_query::Alias::new("is_active"),
				sea_query::Alias::new("last_login"),
				sea_query::Alias::new("created_at"),
				sea_query::Alias::new("bio"),
			])
			.from(sea_query::Alias::new("auth_user"))
			.and_where(Expr::col(sea_query::Alias::new("id")).eq(Expr::val(id)))
			.to_string(PostgresQueryBuilder);

		sqlx::query_as::<_, User>(&sql).fetch_one(pool).await
	}

	/// Find a user by username.
	pub async fn find_by_username(
		&self,
		pool: &PgPool,
		username: &str,
	) -> Result<User, sqlx::Error> {
		let sql = Query::select()
			.columns([
				sea_query::Alias::new("id"),
				sea_query::Alias::new("username"),
				sea_query::Alias::new("email"),
				sea_query::Alias::new("password_hash"),
				sea_query::Alias::new("is_active"),
				sea_query::Alias::new("last_login"),
				sea_query::Alias::new("created_at"),
				sea_query::Alias::new("bio"),
			])
			.from(sea_query::Alias::new("auth_user"))
			.and_where(Expr::col(sea_query::Alias::new("username")).eq(Expr::val(username)))
			.to_string(PostgresQueryBuilder);

		sqlx::query_as::<_, User>(&sql).fetch_one(pool).await
	}
}

/// Factory for creating Profile records in the database.
pub struct ProfileFactory;

impl Default for ProfileFactory {
	fn default() -> Self {
		Self::new()
	}
}

impl ProfileFactory {
	/// Create a new ProfileFactory.
	pub fn new() -> Self {
		Self
	}

	/// Create a profile for the given user.
	pub async fn create_for_user(
		&self,
		pool: &PgPool,
		user_id: Uuid,
		bio: &str,
		avatar_url: &str,
	) -> Result<Profile, sqlx::Error> {
		let id = Uuid::new_v4();
		let now = Utc::now();

		let sql = Query::insert()
			.into_table(sea_query::Alias::new("profile_profile"))
			.columns([
				sea_query::Alias::new("id"),
				sea_query::Alias::new("user_id"),
				sea_query::Alias::new("bio"),
				sea_query::Alias::new("avatar_url"),
				sea_query::Alias::new("location"),
				sea_query::Alias::new("website"),
				sea_query::Alias::new("created_at"),
				sea_query::Alias::new("updated_at"),
			])
			.values_panic([
				id.into(),
				user_id.into(),
				bio.into(),
				avatar_url.into(),
				Option::<String>::None.into(),
				Option::<String>::None.into(),
				now.into(),
				now.into(),
			])
			.to_string(PostgresQueryBuilder);

		sqlx::query(&sql).execute(pool).await?;

		self.find_by_id(pool, id).await
	}

	/// Create a profile with default values.
	pub async fn create_default_for_user(
		&self,
		pool: &PgPool,
		user_id: Uuid,
	) -> Result<Profile, sqlx::Error> {
		self.create_for_user(pool, user_id, "", "").await
	}

	/// Find a profile by ID.
	pub async fn find_by_id(&self, pool: &PgPool, id: Uuid) -> Result<Profile, sqlx::Error> {
		let sql = Query::select()
			.columns([
				sea_query::Alias::new("id"),
				sea_query::Alias::new("user_id"),
				sea_query::Alias::new("bio"),
				sea_query::Alias::new("avatar_url"),
				sea_query::Alias::new("location"),
				sea_query::Alias::new("website"),
				sea_query::Alias::new("created_at"),
				sea_query::Alias::new("updated_at"),
			])
			.from(sea_query::Alias::new("profile_profile"))
			.and_where(Expr::col(sea_query::Alias::new("id")).eq(Expr::val(id)))
			.to_string(PostgresQueryBuilder);

		sqlx::query_as::<_, Profile>(&sql).fetch_one(pool).await
	}

	/// Find a profile by user ID.
	pub async fn find_by_user_id(
		&self,
		pool: &PgPool,
		user_id: Uuid,
	) -> Result<Profile, sqlx::Error> {
		let sql = Query::select()
			.columns([
				sea_query::Alias::new("id"),
				sea_query::Alias::new("user_id"),
				sea_query::Alias::new("bio"),
				sea_query::Alias::new("avatar_url"),
				sea_query::Alias::new("location"),
				sea_query::Alias::new("website"),
				sea_query::Alias::new("created_at"),
				sea_query::Alias::new("updated_at"),
			])
			.from(sea_query::Alias::new("profile_profile"))
			.and_where(Expr::col(sea_query::Alias::new("user_id")).eq(Expr::val(user_id)))
			.to_string(PostgresQueryBuilder);

		sqlx::query_as::<_, Profile>(&sql).fetch_one(pool).await
	}
}

/// rstest fixture providing a UserFactory.
#[fixture]
pub fn user_factory() -> UserFactory {
	UserFactory::new()
}

/// rstest fixture providing a ProfileFactory.
#[fixture]
pub fn profile_factory() -> ProfileFactory {
	ProfileFactory::new()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_utils::fixtures::database::twitter_db_pool;

	#[rstest]
	#[tokio::test]
	async fn test_user_factory_create(
		#[future] twitter_db_pool: (PgPool, String),
		user_factory: UserFactory,
	) {
		let (pool, _url) = twitter_db_pool.await;

		let test_user = TestTwitterUser::new("factoryuser");
		let user = user_factory
			.create_from_test_user(&pool, &test_user)
			.await
			.expect("User creation should succeed");

		assert_eq!(user.username(), "factoryuser");
		assert_eq!(user.email(), "factoryuser@example.com");
		assert!(user.is_active());
	}

	#[rstest]
	#[tokio::test]
	async fn test_profile_factory_create(
		#[future] twitter_db_pool: (PgPool, String),
		user_factory: UserFactory,
		profile_factory: ProfileFactory,
	) {
		let (pool, _url) = twitter_db_pool.await;

		let test_user = TestTwitterUser::new("profileuser");
		let user = user_factory
			.create_from_test_user(&pool, &test_user)
			.await
			.expect("User creation should succeed");

		let profile = profile_factory
			.create_for_user(
				&pool,
				user.id(),
				"Test bio",
				"https://example.com/avatar.png",
			)
			.await
			.expect("Profile creation should succeed");

		assert_eq!(profile.user_id(), user.id());
		assert_eq!(profile.bio(), "Test bio");
		assert_eq!(profile.avatar_url(), "https://example.com/avatar.png");
	}
}
