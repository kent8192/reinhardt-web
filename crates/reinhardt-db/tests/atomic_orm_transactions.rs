//! Live-backend coverage for closure-scoped ORM transactions.
//!
//! Each fixture owns the database resource that its connection uses. PostgreSQL
//! and MySQL containers therefore remain alive through post-commit assertions,
//! and the SQLite database file stays inside a [`tempfile::TempDir`] until its
//! connection has been dropped.

#![cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]

use std::time::Duration;

use reinhardt_core::exception::Error;
use reinhardt_db::associations::ManyToManyManager;
use reinhardt_db::orm::connection::{
	BackendsConnection, DatabaseConnection, DatabaseConnectionLease,
};
use reinhardt_db::orm::custom_manager::CustomManager;
use reinhardt_db::orm::manager::Manager;
use reinhardt_db::orm::model::{FieldSelector, Model};
use reinhardt_db::orm::query::{Filter, FilterOperator, FilterValue, QuerySet};
use reinhardt_db::orm::transaction::IsolationLevel;
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use tempfile::TempDir;
use testcontainers::{
	ContainerAsync, GenericImage, ImageExt,
	core::{IntoContainerPort, WaitFor},
	runners::AsyncRunner,
};

const MAX_CONNECT_RETRIES: u32 = 7;

async fn sqlite_connection(url: &str) -> (DatabaseConnectionLease, DatabaseConnection) {
	let owner = BackendsConnection::connect_sqlite(url).await.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let handle = lease.handle();
	(lease, handle)
}

struct PostgresFixture {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_container: ContainerAsync<GenericImage>,
}

struct MySqlFixture {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_container: ContainerAsync<GenericImage>,
}

struct SqliteFixture {
	connection: DatabaseConnection,
	_lease: DatabaseConnectionLease,
	_directory: TempDir,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct AtomicArticle {
	id: Option<i64>,
	title: String,
	phase: String,
}

#[derive(Clone)]
struct AtomicArticleFields;

impl FieldSelector for AtomicArticleFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for AtomicArticle {
	type PrimaryKey = i64;
	type Fields = AtomicArticleFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"atomic_articles"
	}

	fn new_fields() -> Self::Fields {
		AtomicArticleFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct AtomicTag {
	id: Option<i64>,
	name: String,
}

#[derive(Clone)]
struct AtomicTagFields;

impl FieldSelector for AtomicTagFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for AtomicTag {
	type PrimaryKey = i64;
	type Fields = AtomicTagFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"atomic_tags"
	}

	fn new_fields() -> Self::Fields {
		AtomicTagFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct GeneratedIdArticle {
	id: Option<i64>,
	title: String,
	is_enabled: bool,
}

#[derive(Clone)]
struct GeneratedIdArticleFields;

impl FieldSelector for GeneratedIdArticleFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for GeneratedIdArticle {
	type PrimaryKey = i64;
	type Fields = GeneratedIdArticleFields;
	type Objects = Manager<Self>;

	fn table_name() -> &'static str {
		"atomic_generated_articles"
	}

	fn new_fields() -> Self::Fields {
		GeneratedIdArticleFields
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Default)]
struct AtomicArticleManager;

impl CustomManager for AtomicArticleManager {
	type Model = AtomicArticle;

	fn new() -> Self {
		Self
	}

	fn before_save(&self, model: &mut Self::Model) -> reinhardt_core::exception::Result<()> {
		model.phase = "custom-managed".to_string();
		Ok(())
	}
}

#[derive(Clone, Copy)]
enum LifecycleBackend {
	Postgres,
	MySql,
	Sqlite,
}

#[fixture]
async fn postgres_fixture() -> PostgresFixture {
	let image = GenericImage::new("postgres", "16-alpine")
		.with_exposed_port(5432.tcp())
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_startup_timeout(Duration::from_secs(120))
		.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust");
	let container = image
		.start()
		.await
		.expect("PostgreSQL container should start");
	let port = container_port_with_retry(&container, 5432, "PostgreSQL").await;
	let url = format!("postgres://postgres@127.0.0.1:{port}/postgres?sslmode=disable");
	let (lease, connection) = connect_postgres_with_retry(&url).await;

	PostgresFixture {
		connection,
		_lease: lease,
		_container: container,
	}
}

#[fixture]
async fn mysql_fixture() -> MySqlFixture {
	let image = GenericImage::new("mysql", "8.0")
		.with_exposed_port(3306.tcp())
		.with_wait_for(WaitFor::message_on_stderr(
			"port: 3306  MySQL Community Server",
		))
		.with_startup_timeout(Duration::from_secs(120))
		.with_env_var("MYSQL_ROOT_PASSWORD", "test")
		.with_env_var("MYSQL_DATABASE", "atomic_orm_test");
	let container = image.start().await.expect("MySQL container should start");
	let port = container_port_with_retry(&container, 3306, "MySQL").await;
	let url = format!("mysql://root:test@127.0.0.1:{port}/atomic_orm_test");
	let (lease, connection) = connect_mysql_with_retry(&url).await;

	MySqlFixture {
		connection,
		_lease: lease,
		_container: container,
	}
}

#[fixture]
async fn sqlite_fixture() -> SqliteFixture {
	let directory = tempfile::Builder::new()
		.prefix("reinhardt-atomic-orm-")
		.tempdir_in("/tmp")
		.expect("SQLite temporary directory should be created under /tmp");
	let database_path = directory.path().join("atomic.sqlite");
	let url = format!("sqlite:///{}", database_path.display());
	let (lease, connection) = sqlite_connection(&url).await;

	SqliteFixture {
		connection,
		_lease: lease,
		_directory: directory,
	}
}

async fn container_port_with_retry(
	container: &ContainerAsync<GenericImage>,
	container_port: u16,
	service: &str,
) -> u16 {
	tokio::time::sleep(Duration::from_millis(500)).await;

	for attempt in 0..=MAX_CONNECT_RETRIES {
		match container.get_host_port_ipv4(container_port).await {
			Ok(port) => return port,
			Err(error) if attempt < MAX_CONNECT_RETRIES => {
				eprintln!(
					"{service} port lookup attempt {} of {} failed: {error}",
					attempt + 1,
					MAX_CONNECT_RETRIES + 1,
				);
				tokio::time::sleep(Duration::from_millis(200 * 2_u64.pow(attempt + 1))).await;
			}
			Err(error) => panic!(
				"{service} port lookup failed after {} attempts: {error}",
				MAX_CONNECT_RETRIES + 1,
			),
		}
	}

	unreachable!("the final container port lookup either returns or panics")
}

async fn connect_postgres_with_retry(url: &str) -> (DatabaseConnectionLease, DatabaseConnection) {
	tokio::time::sleep(Duration::from_millis(500)).await;

	for attempt in 0..=MAX_CONNECT_RETRIES {
		match BackendsConnection::connect_postgres(url).await {
			Ok(owner) => {
				let lease = DatabaseConnectionLease::register(owner).unwrap();
				let connection = lease.handle();
				return (lease, connection);
			}
			Err(error) if attempt < MAX_CONNECT_RETRIES => {
				eprintln!(
					"PostgreSQL connection attempt {} of {} failed: {error}",
					attempt + 1,
					MAX_CONNECT_RETRIES + 1,
				);
				tokio::time::sleep(Duration::from_millis(200 * 2_u64.pow(attempt + 1))).await;
			}
			Err(error) => panic!(
				"PostgreSQL connection failed after {} attempts: {error}",
				MAX_CONNECT_RETRIES + 1,
			),
		}
	}

	unreachable!("the final PostgreSQL connection attempt either returns or panics")
}

async fn connect_mysql_with_retry(url: &str) -> (DatabaseConnectionLease, DatabaseConnection) {
	tokio::time::sleep(Duration::from_millis(500)).await;

	for attempt in 0..=MAX_CONNECT_RETRIES {
		match BackendsConnection::connect_mysql(url).await {
			Ok(owner) => {
				let lease = DatabaseConnectionLease::register(owner).unwrap();
				let connection = lease.handle();
				return (lease, connection);
			}
			Err(error) if attempt < MAX_CONNECT_RETRIES => {
				eprintln!(
					"MySQL connection attempt {} of {} failed: {error}",
					attempt + 1,
					MAX_CONNECT_RETRIES + 1,
				);
				tokio::time::sleep(Duration::from_millis(200 * 2_u64.pow(attempt + 1))).await;
			}
			Err(error) => panic!(
				"MySQL connection failed after {} attempts: {error}",
				MAX_CONNECT_RETRIES + 1,
			),
		}
	}

	unreachable!("the final MySQL connection attempt either returns or panics")
}

async fn create_lifecycle_schema(
	connection: &DatabaseConnection,
	backend: LifecycleBackend,
) -> reinhardt_core::exception::Result<()> {
	let (articles, tags, article_tags) = match backend {
		LifecycleBackend::Postgres => (
			"CREATE TABLE atomic_articles (id BIGSERIAL PRIMARY KEY, title TEXT NOT NULL, phase TEXT NOT NULL)",
			"CREATE TABLE atomic_tags (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL)",
			"CREATE TABLE atomic_articles_tags (atomic_articles_id BIGINT NOT NULL REFERENCES atomic_articles(id), atomic_tags_id BIGINT NOT NULL REFERENCES atomic_tags(id), PRIMARY KEY (atomic_articles_id, atomic_tags_id))",
		),
		LifecycleBackend::MySql => (
			"CREATE TABLE atomic_articles (id BIGINT NOT NULL AUTO_INCREMENT, title VARCHAR(255) NOT NULL, phase VARCHAR(255) NOT NULL, PRIMARY KEY (id)) ENGINE=InnoDB",
			"CREATE TABLE atomic_tags (id BIGINT NOT NULL AUTO_INCREMENT, name VARCHAR(255) NOT NULL, PRIMARY KEY (id)) ENGINE=InnoDB",
			"CREATE TABLE atomic_articles_tags (atomic_articles_id BIGINT NOT NULL, atomic_tags_id BIGINT NOT NULL, PRIMARY KEY (atomic_articles_id, atomic_tags_id), CONSTRAINT atomic_articles_tags_article_fk FOREIGN KEY (atomic_articles_id) REFERENCES atomic_articles(id), CONSTRAINT atomic_articles_tags_tag_fk FOREIGN KEY (atomic_tags_id) REFERENCES atomic_tags(id)) ENGINE=InnoDB",
		),
		LifecycleBackend::Sqlite => (
			"CREATE TABLE atomic_articles (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, phase TEXT NOT NULL)",
			"CREATE TABLE atomic_tags (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
			"CREATE TABLE atomic_articles_tags (atomic_articles_id INTEGER NOT NULL REFERENCES atomic_articles(id), atomic_tags_id INTEGER NOT NULL REFERENCES atomic_tags(id), PRIMARY KEY (atomic_articles_id, atomic_tags_id))",
		),
	};

	connection.execute(articles, vec![]).await?;
	connection.execute(tags, vec![]).await?;
	connection.execute(article_tags, vec![]).await?;
	Ok(())
}

async fn create_mysql_generated_id_schema(
	connection: &DatabaseConnection,
) -> reinhardt_core::exception::Result<()> {
	connection
		.execute(
			"CREATE TABLE atomic_generated_articles (id BIGINT NOT NULL AUTO_INCREMENT, title VARCHAR(255) NOT NULL, is_enabled BOOLEAN NOT NULL, PRIMARY KEY (id)) ENGINE=InnoDB AUTO_INCREMENT=10",
			vec![],
		)
		.await?;
	Ok(())
}

fn article(title: &str, phase: &str) -> AtomicArticle {
	AtomicArticle {
		id: None,
		title: title.to_string(),
		phase: phase.to_string(),
	}
}

fn article_id(article: &AtomicArticle, context: &str) -> reinhardt_core::exception::Result<i64> {
	article
		.id
		.ok_or_else(|| Error::Internal(format!("{context} should have a generated primary key")))
}

fn tag_id(tag: &AtomicTag, context: &str) -> reinhardt_core::exception::Result<i64> {
	tag.id
		.ok_or_else(|| Error::Internal(format!("{context} should have a generated primary key")))
}

fn generated_id(
	article: &GeneratedIdArticle,
	context: &str,
) -> reinhardt_core::exception::Result<i64> {
	article
		.id
		.ok_or_else(|| Error::Internal(format!("{context} should have a generated primary key")))
}

async fn article_count(
	connection: &mut DatabaseConnection,
) -> reinhardt_core::exception::Result<i64> {
	Manager::<AtomicArticle>::new()
		.count_with_conn(connection)
		.await
}

async fn tag_count(connection: &mut DatabaseConnection) -> reinhardt_core::exception::Result<i64> {
	Manager::<AtomicTag>::new()
		.count_with_conn(connection)
		.await
}

async fn article_named(
	connection: &mut DatabaseConnection,
	title: &str,
) -> reinhardt_core::exception::Result<Option<AtomicArticle>> {
	QuerySet::<AtomicArticle>::new()
		.filter(Filter::new(
			"title",
			FilterOperator::Eq,
			FilterValue::String(title.to_string()),
		))
		.first_with_db(connection)
		.await
}

async fn tag_named(
	connection: &mut DatabaseConnection,
	name: &str,
) -> reinhardt_core::exception::Result<Option<AtomicTag>> {
	QuerySet::<AtomicTag>::new()
		.filter(Filter::new(
			"name",
			FilterOperator::Eq,
			FilterValue::String(name.to_string()),
		))
		.first_with_db(connection)
		.await
}

async fn verify_atomic_lifecycle(
	connection: &mut DatabaseConnection,
) -> reinhardt_core::exception::Result<()> {
	let (committed_source_id, committed_tag_id) = connection
		.atomic(async |transaction| {
			let article_manager = Manager::<AtomicArticle>::new();
			let tag_manager = Manager::<AtomicTag>::new();

			let created = article_manager
				.create_with_conn(transaction, &article("manager-created", "manager"))
				.await?;
			let source_id = article_id(&created, "manager create")?;
			let updated = article_manager
				.update_with_conn(
					transaction,
					&AtomicArticle {
						id: Some(source_id),
						title: "manager-updated".to_string(),
						phase: "manager".to_string(),
					},
				)
				.await?;
			assert_eq!(updated.title, "manager-updated");

			let deleted = article_manager
				.create_with_conn(transaction, &article("manager-deleted", "manager"))
				.await?;
			article_manager
				.delete_with_conn(transaction, article_id(&deleted, "manager delete")?)
				.await?;

			let mut persisted_model = article("model-created", "model");
			persisted_model.save_with_conn(transaction).await?;
			persisted_model.title = "model-updated".to_string();
			persisted_model.save_with_conn(transaction).await?;
			assert_eq!(persisted_model.phase, "model");

			let custom_created = AtomicArticleManager::new()
				.create_with_conn(transaction, &article("custom-created", "original"))
				.await?;
			assert_eq!(custom_created.phase, "custom-managed");

			let queried = QuerySet::<AtomicArticle>::new()
				.filter(Filter::new(
					"title",
					FilterOperator::Eq,
					FilterValue::String("manager-updated".to_string()),
				))
				.first_with_db(transaction)
				.await?;
			assert_eq!(
				queried.as_ref().map(|article| article.phase.as_str()),
				Some("manager")
			);
			assert_eq!(article_manager.count_with_conn(transaction).await?, 3);

			let tag = tag_manager
				.create_with_conn(
					transaction,
					&AtomicTag {
						id: None,
						name: "committed-tag".to_string(),
					},
				)
				.await?;
			let target_id = tag_id(&tag, "tag create")?;
			let relations = ManyToManyManager::<AtomicArticle, AtomicTag, i64>::new(
				source_id,
				"atomic_articles_tags".to_string(),
				"atomic_articles_id".to_string(),
				"atomic_tags_id".to_string(),
			);
			relations.add_with_db(transaction, target_id).await?;
			assert_eq!(relations.count_with_db(transaction).await?, 1);

			Ok::<_, Error>((source_id, target_id))
		})
		.await?;

	assert_eq!(article_count(connection).await?, 3);
	assert_eq!(tag_count(connection).await?, 1);
	let mut committed_rows = QuerySet::<AtomicArticle>::new()
		.all_with_db(connection)
		.await?;
	committed_rows.sort_by(|left, right| left.title.cmp(&right.title));
	let committed_values = committed_rows
		.into_iter()
		.map(|article| (article.title, article.phase))
		.collect::<Vec<_>>();
	assert_eq!(
		committed_values,
		vec![
			("custom-created".to_string(), "custom-managed".to_string()),
			("manager-updated".to_string(), "manager".to_string()),
			("model-updated".to_string(), "model".to_string()),
		]
	);
	let committed_relations = ManyToManyManager::<AtomicArticle, AtomicTag, i64>::new(
		committed_source_id,
		"atomic_articles_tags".to_string(),
		"atomic_articles_id".to_string(),
		"atomic_tags_id".to_string(),
	);
	assert_eq!(committed_relations.count_with_db(connection).await?, 1);
	assert_eq!(
		committed_relations
			.contains_with_db(connection, committed_tag_id)
			.await?,
		true
	);

	let outer_error = connection
		.atomic(async |transaction| {
			let article_manager = Manager::<AtomicArticle>::new();
			let tag_manager = Manager::<AtomicTag>::new();
			let rolled_back_article = article_manager
				.create_with_conn(transaction, &article("outer-rollback", "rollback"))
				.await?;
			let rolled_back_tag = tag_manager
				.create_with_conn(
					transaction,
					&AtomicTag {
						id: None,
						name: "rolled-back-tag".to_string(),
					},
				)
				.await?;
			let relations = ManyToManyManager::<AtomicArticle, AtomicTag, i64>::new(
				article_id(&rolled_back_article, "outer rollback article")?,
				"atomic_articles_tags".to_string(),
				"atomic_articles_id".to_string(),
				"atomic_tags_id".to_string(),
			);
			relations
				.add_with_db(transaction, tag_id(&rolled_back_tag, "outer rollback tag")?)
				.await?;

			Err::<(), Error>(Error::Validation("outer rollback".to_string()))
		})
		.await
		.expect_err("an outer callback error should roll back its transaction");
	assert_eq!(outer_error.to_string(), "Validation error: outer rollback");
	assert_eq!(article_count(connection).await?, 3);
	assert_eq!(tag_count(connection).await?, 1);
	assert_eq!(committed_relations.count_with_db(connection).await?, 1);
	assert_eq!(article_named(connection, "outer-rollback").await?, None);
	assert_eq!(tag_named(connection, "rolled-back-tag").await?, None);

	connection
		.atomic(async |outer| {
			let manager = Manager::<AtomicArticle>::new();
			manager
				.create_with_conn(outer, &article("nested-success-outer", "nested"))
				.await?;
			let nested = outer
				.atomic(async |inner| {
					Manager::<AtomicArticle>::new()
						.create_with_conn(inner, &article("nested-success-inner", "nested"))
						.await
				})
				.await?;
			assert_eq!(nested.title, "nested-success-inner");
			Ok::<_, Error>(())
		})
		.await?;
	assert_eq!(article_count(connection).await?, 5);
	assert_eq!(
		article_named(connection, "nested-success-outer")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("nested-success-outer".to_string(), "nested".to_string()))
	);
	assert_eq!(
		article_named(connection, "nested-success-inner")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("nested-success-inner".to_string(), "nested".to_string()))
	);

	connection
		.atomic(async |outer| {
			let manager = Manager::<AtomicArticle>::new();
			manager
				.create_with_conn(outer, &article("nested-caught-outer", "nested"))
				.await?;
			let nested_error = outer
				.atomic(async |inner| {
					Manager::<AtomicArticle>::new()
						.create_with_conn(inner, &article("nested-caught-inner", "nested"))
						.await?;
					Err::<(), Error>(Error::Validation("nested caught rollback".to_string()))
				})
				.await
				.expect_err("a nested callback error should roll back its savepoint");
			assert_eq!(
				nested_error.to_string(),
				"Validation error: nested caught rollback"
			);
			manager
				.create_with_conn(outer, &article("nested-caught-after", "nested"))
				.await?;
			Ok::<_, Error>(())
		})
		.await?;
	assert_eq!(article_count(connection).await?, 7);
	assert_eq!(
		article_named(connection, "nested-caught-outer")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("nested-caught-outer".to_string(), "nested".to_string()))
	);
	assert_eq!(
		article_named(connection, "nested-caught-inner").await?,
		None
	);
	assert_eq!(
		article_named(connection, "nested-caught-after")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("nested-caught-after".to_string(), "nested".to_string()))
	);

	let propagated_error = connection
		.atomic(async |outer| {
			let manager = Manager::<AtomicArticle>::new();
			manager
				.create_with_conn(outer, &article("nested-propagated-outer", "nested"))
				.await?;
			outer
				.atomic(async |inner| {
					Manager::<AtomicArticle>::new()
						.create_with_conn(inner, &article("nested-propagated-inner", "nested"))
						.await?;
					Err::<(), Error>(Error::Validation("nested propagated rollback".to_string()))
				})
				.await?;
			Ok::<_, Error>(())
		})
		.await
		.expect_err("a propagated nested error should roll back the outer transaction");
	assert_eq!(
		propagated_error.to_string(),
		"Validation error: nested propagated rollback"
	);
	assert_eq!(article_count(connection).await?, 7);
	assert_eq!(
		article_named(connection, "nested-propagated-outer").await?,
		None
	);
	assert_eq!(
		article_named(connection, "nested-propagated-inner").await?,
		None
	);

	Ok(())
}

async fn verify_mysql_isolation_atomic_savepoints(
	connection: &mut DatabaseConnection,
) -> reinhardt_core::exception::Result<()> {
	connection
		.atomic_with_isolation(IsolationLevel::ReadCommitted, async |outer| {
			let outer_article = Manager::<AtomicArticle>::new()
				.create_with_conn(outer, &article("isolation-outer", "isolation"))
				.await?;
			assert_eq!(outer_article.title, "isolation-outer");

			let nested_article = outer
				.atomic(async |inner| {
					Manager::<AtomicArticle>::new()
						.create_with_conn(inner, &article("isolation-nested", "isolation"))
						.await
				})
				.await?;
			assert_eq!(nested_article.title, "isolation-nested");

			let caught_error = outer
				.atomic(async |inner| {
					Manager::<AtomicArticle>::new()
						.create_with_conn(inner, &article("isolation-caught-inner", "isolation"))
						.await?;
					Err::<(), Error>(Error::Validation("isolation nested rollback".to_string()))
				})
				.await
				.expect_err("a nested isolation error should roll back its savepoint");
			assert_eq!(
				caught_error.to_string(),
				"Validation error: isolation nested rollback"
			);

			let after_article = Manager::<AtomicArticle>::new()
				.create_with_conn(outer, &article("isolation-after", "isolation"))
				.await?;
			assert_eq!(after_article.title, "isolation-after");
			Ok::<_, Error>(())
		})
		.await?;

	assert_eq!(article_count(connection).await?, 10);
	assert_eq!(
		article_named(connection, "isolation-outer")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("isolation-outer".to_string(), "isolation".to_string()))
	);
	assert_eq!(
		article_named(connection, "isolation-nested")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("isolation-nested".to_string(), "isolation".to_string()))
	);
	assert_eq!(
		article_named(connection, "isolation-caught-inner").await?,
		None
	);
	assert_eq!(
		article_named(connection, "isolation-after")
			.await?
			.map(|article| (article.title, article.phase)),
		Some(("isolation-after".to_string(), "isolation".to_string()))
	);

	Ok(())
}

async fn verify_mysql_generated_id_reloads(
	connection: &mut DatabaseConnection,
) -> reinhardt_core::exception::Result<()> {
	let manager = Manager::<GeneratedIdArticle>::new();
	let explicit = manager
		.create_with_conn(
			connection,
			&GeneratedIdArticle {
				id: Some(5),
				title: "explicit-five".to_string(),
				is_enabled: false,
			},
		)
		.await?;
	assert_eq!(explicit.id, Some(5));
	assert_eq!(explicit.title, "explicit-five");
	assert_eq!(explicit.is_enabled, false);

	let direct = manager
		.create_with_conn(
			connection,
			&GeneratedIdArticle {
				id: None,
				title: "direct-generated".to_string(),
				is_enabled: true,
			},
		)
		.await?;
	assert_eq!(direct.id, Some(10));
	assert_eq!(direct.title, "direct-generated");
	assert_eq!(direct.is_enabled, true);
	let direct_reloaded = Manager::<GeneratedIdArticle>::new()
		.get(10)
		.get_with_db(connection)
		.await?;
	assert_eq!(direct_reloaded, direct);

	let nested = connection
		.atomic(async |outer| {
			outer
				.atomic(async |inner| {
					Manager::<GeneratedIdArticle>::new()
						.create_with_conn(
							inner,
							&GeneratedIdArticle {
								id: None,
								title: "nested-generated".to_string(),
								is_enabled: false,
							},
						)
						.await
				})
				.await
		})
		.await?;
	assert_eq!(nested.id, Some(11));
	assert_eq!(nested.title, "nested-generated");
	assert_eq!(nested.is_enabled, false);
	let nested_reloaded = Manager::<GeneratedIdArticle>::new()
		.get(generated_id(&nested, "nested generated id")?)
		.get_with_db(connection)
		.await?;
	assert_eq!(nested_reloaded, nested);

	Ok(())
}

#[rstest]
#[tokio::test]
#[serial(orm_atomic_transactions)]
async fn postgres_atomic_orm_lifecycle_commits_and_rolls_back(
	#[future] postgres_fixture: PostgresFixture,
) {
	let mut fixture = postgres_fixture.await;
	create_lifecycle_schema(&fixture.connection, LifecycleBackend::Postgres)
		.await
		.expect("PostgreSQL schema should be created before atomic callbacks");
	verify_atomic_lifecycle(&mut fixture.connection)
		.await
		.expect("PostgreSQL atomic ORM lifecycle should preserve the expected rows");
}

#[rstest]
#[tokio::test]
#[serial(orm_atomic_transactions)]
async fn sqlite_atomic_orm_lifecycle_commits_and_rolls_back(
	#[future] sqlite_fixture: SqliteFixture,
) {
	let mut fixture = sqlite_fixture.await;
	create_lifecycle_schema(&fixture.connection, LifecycleBackend::Sqlite)
		.await
		.expect("SQLite schema should be created before atomic callbacks");
	verify_atomic_lifecycle(&mut fixture.connection)
		.await
		.expect("SQLite atomic ORM lifecycle should preserve the expected rows");
}

#[rstest]
#[tokio::test]
#[serial(orm_atomic_transactions)]
async fn mysql_atomic_orm_lifecycle_and_generated_ids_are_connection_affine(
	#[future] mysql_fixture: MySqlFixture,
) {
	let mut fixture = mysql_fixture.await;
	create_lifecycle_schema(&fixture.connection, LifecycleBackend::MySql)
		.await
		.expect("MySQL lifecycle schema should be created before atomic callbacks");
	create_mysql_generated_id_schema(&fixture.connection)
		.await
		.expect("MySQL generated-ID schema should be created before atomic callbacks");
	verify_atomic_lifecycle(&mut fixture.connection)
		.await
		.expect("MySQL atomic ORM lifecycle should preserve the expected rows");
	verify_mysql_isolation_atomic_savepoints(&mut fixture.connection)
		.await
		.expect("MySQL isolation transactions should preserve nested savepoint atomicity");
	verify_mysql_generated_id_reloads(&mut fixture.connection)
		.await
		.expect("MySQL generated IDs should reload through their caller-owned executors");
}
