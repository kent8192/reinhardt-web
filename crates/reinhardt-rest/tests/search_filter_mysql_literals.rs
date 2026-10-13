//! MySQL regressions for search filters that return SQL with embedded search text.
//!
//! The filters cannot bind parameters, so their literals must keep the same
//! boundary with and without `NO_BACKSLASH_ESCAPES`, and the added search
//! condition must not widen the caller's existing WHERE condition.
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use reinhardt_rest::filters::{
	FilterBackend, FilterError, RelevanceScorer, ScoringAlgorithm, SynonymDictionary,
	SynonymExpander,
};
use rstest::{fixture, rstest};
use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;
use std::collections::HashMap;
use std::time::Duration;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

const TENANT_ONE_ARTICLES: &str = "SELECT id FROM articles WHERE tenant_id = 1";

#[fixture]
async fn mysql_pool() -> (ContainerAsync<Mysql>, MySqlPool) {
	// First-time initialization can exceed the default minute on loaded Docker hosts.
	let container = Mysql::default()
		.with_tag("8.0")
		.with_startup_timeout(Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let host = container.get_host().await.unwrap();
	// A single connection keeps the session sql_mode for every query.
	let pool = MySqlPoolOptions::new()
		.max_connections(1)
		.connect(&format!("mysql://root@{host}:{port}/test"))
		.await
		.unwrap();
	sqlx::query(
		"CREATE TABLE articles (id BIGINT PRIMARY KEY, tenant_id BIGINT NOT NULL, content VARCHAR(255) NOT NULL)",
	)
	.execute(&pool)
	.await
	.unwrap();
	for (id, tenant_id, content) in [
		(1_i64, 1_i64, "it's fast"),
		(2, 1, "quick and steady"),
		(3, 2, "fast private draft"),
		(4, 2, "it's private"),
	] {
		sqlx::query("INSERT INTO articles (id, tenant_id, content) VALUES (?, ?, ?)")
			.bind(id)
			.bind(tenant_id)
			.bind(content)
			.execute(&pool)
			.await
			.unwrap();
	}
	(container, pool)
}

fn search(text: &str) -> HashMap<String, String> {
	HashMap::from([("q".to_string(), text.to_string())])
}

async fn sorted_ids(pool: &MySqlPool, sql: &str) -> Vec<i64> {
	let mut ids: Vec<i64> = sqlx::query_scalar(sql).fetch_all(pool).await.unwrap();
	ids.sort_unstable();
	ids
}

#[rstest]
#[tokio::test]
async fn search_filters_keep_literal_and_scope_boundaries_on_mysql(
	#[future] mysql_pool: (ContainerAsync<Mysql>, MySqlPool),
) {
	// Arrange
	let (_container, pool) = mysql_pool.await;
	let mut dictionary = SynonymDictionary::new();
	dictionary.add_synonym("fast", "quick");
	let synonyms = SynonymExpander::new().with_dictionary(dictionary);
	let scorer = RelevanceScorer::new()
		.with_algorithm(ScoringAlgorithm::TfIdf)
		.with_boost_field("content", 1.0);

	for mode in ["", "NO_BACKSLASH_ESCAPES"] {
		sqlx::query("SET SESSION sql_mode = ?")
			.bind(mode)
			.execute(&pool)
			.await
			.unwrap();

		// Act
		let expanded = synonyms
			.filter_queryset(&search("fast"), TENANT_ONE_ARTICLES.to_string())
			.await
			.unwrap();
		let quoted = synonyms
			.filter_queryset(&search("it's"), TENANT_ONE_ARTICLES.to_string())
			.await
			.unwrap();
		let scored = scorer
			.filter_queryset(&search("' OR 1=1 -- "), TENANT_ONE_ARTICLES.to_string())
			.await
			.unwrap();
		let rejected = [
			synonyms
				.filter_queryset(&search("x\\' OR 1=1 -- "), TENANT_ONE_ARTICLES.to_string())
				.await,
			scorer
				.filter_queryset(&search("x\\' OR 1=1 -- "), TENANT_ONE_ARTICLES.to_string())
				.await,
		];

		// Assert
		// Synonym alternatives only narrow the tenant condition; row 3 matches
		// "fast" in another tenant.
		assert_eq!(
			sorted_ids(&pool, &expanded).await,
			vec![1, 2],
			"mode {mode:?}"
		);
		assert_eq!(sorted_ids(&pool, &quoted).await, vec![1], "mode {mode:?}");
		assert_eq!(
			sorted_ids(&pool, &scored).await,
			vec![1, 2],
			"mode {mode:?}"
		);
		for result in rejected {
			assert!(
				matches!(result, Err(FilterError::InvalidParameter(_))),
				"mode {mode:?}: expected InvalidParameter, got {result:?}"
			);
		}
	}
}
