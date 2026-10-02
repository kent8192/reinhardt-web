//! Execute ORM-generated relationship plans against SQLite and count backend calls.
//! Load-option and LoadContext state contracts are owned by orm::loading unit tests.

use async_trait::async_trait;
use reinhardt_core::exception::Result;
use reinhardt_db::orm::connection::{
	BackendsConnection, DatabaseBackend, DatabaseConnection, DatabaseConnectionLease, OrmExecutor,
	QueryResult, QueryValue, Row,
};
use reinhardt_db::orm::execution::{QueryExecution, SelectExecution};
use reinhardt_db::orm::{
	Filter, FilterOperator, FilterValue, NPlusOneConfig, NPlusOneScope, QuerySet,
};
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Author {
	id: Option<i64>,
	name: String,
}
reinhardt_test::impl_test_model!(Author, i64, "authors");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Book {
	id: Option<i64>,
	author_id: Option<i64>,
	title: String,
}
reinhardt_test::impl_test_model!(Book, i64, "books");

/// Counts calls made by production ORM code, while executing every call on SQLite.
struct CountingExecutor {
	connection: DatabaseConnection,
	calls: usize,
}

#[async_trait]
impl OrmExecutor for CountingExecutor {
	fn backend(&self) -> DatabaseBackend {
		self.connection.backend()
	}
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		self.calls += 1;
		OrmExecutor::execute(&mut self.connection, sql, params).await
	}
	async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		self.calls += 1;
		OrmExecutor::fetch_one(&mut self.connection, sql, params).await
	}
	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		self.calls += 1;
		OrmExecutor::fetch_all(&mut self.connection, sql, params).await
	}
	async fn fetch_optional(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		self.calls += 1;
		OrmExecutor::fetch_optional(&mut self.connection, sql, params).await
	}
}

#[fixture]
async fn sqlite_fixture() -> (DatabaseConnectionLease, CountingExecutor) {
	let owner = BackendsConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	let lease = DatabaseConnectionLease::register(owner).unwrap();
	let connection = lease.handle();
	for sql in [
		"CREATE TABLE authors (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
		"CREATE TABLE books (id INTEGER PRIMARY KEY, author_id INTEGER, title TEXT NOT NULL)",
		"INSERT INTO authors VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Carol')",
		"INSERT INTO books VALUES (10, 1, 'A'), (11, 1, 'B'), (12, 2, 'C'), (13, NULL, 'Orphan')",
	] {
		connection.execute(sql, vec![]).await.unwrap();
	}
	(
		lease,
		CountingExecutor {
			connection,
			calls: 0,
		},
	)
}

#[rstest]
#[tokio::test]
async fn test_lazy_queries_report_n_plus_one(
	#[future] sqlite_fixture: (DatabaseConnectionLease, CountingExecutor),
) {
	// Arrange
	let (_lease, mut executor) = sqlite_fixture.await;
	let config = NPlusOneConfig {
		threshold: 3,
		min_distinct_params: 3,
		..Default::default()
	};
	// Act
	let (mut titles, report) = NPlusOneScope::warn("lazy authors to books", config)
		.run_with_report(async {
			let authors = QuerySet::<Author>::new()
				.all_with_db(&mut executor)
				.await
				.unwrap();
			let mut titles = Vec::new();
			for author in authors {
				let books = QuerySet::<Book>::new()
					.filter(Filter::new(
						"author_id",
						FilterOperator::Eq,
						FilterValue::Integer(author.id.unwrap()),
					))
					.all_with_db(&mut executor)
					.await
					.unwrap();
				titles.extend(books.into_iter().map(|book| book.title));
			}
			titles
		})
		.await;
	titles.sort();
	// Assert
	assert_eq!(titles, ["A", "B", "C"]);
	assert_eq!(executor.calls, 4);
	assert_eq!(report.total_recorded_queries, 4);
	assert_eq!(report.findings.len(), 1);
	assert_eq!(report.findings[0].execution_count, 3);
	assert_eq!(report.findings[0].distinct_bind_signature_count, 3);
}

#[rstest]
#[tokio::test]
async fn test_select_related_executes_one_join_and_preserves_missing_parent(
	#[future] sqlite_fixture: (DatabaseConnectionLease, CountingExecutor),
) {
	// Arrange
	let (_lease, mut executor) = sqlite_fixture.await;
	// Act
	let rows = QuerySet::<Book>::new()
		.select_related(["author"])
		.rows_with_db(&mut executor)
		.await
		.unwrap();
	let mut pairs: Vec<_> = rows
		.into_iter()
		.map(|row| {
			(
				row.data["title"].as_str().unwrap().to_owned(),
				row.data["name"].as_str().map(str::to_owned),
			)
		})
		.collect();
	pairs.sort();
	// Assert
	assert_eq!(executor.calls, 1);
	assert_eq!(
		pairs,
		vec![
			("A".into(), Some("Alice".into())),
			("B".into(), Some("Alice".into())),
			("C".into(), Some("Bob".into())),
			("Orphan".into(), None),
		]
	);
}

#[rstest]
#[case::parents_with_and_without_children(0, 2, vec!["A", "B", "C"])]
#[case::empty_parents(99, 1, vec![])]
#[tokio::test]
async fn test_prefetch_plan_executes_batched_queries(
	#[future] sqlite_fixture: (DatabaseConnectionLease, CountingExecutor),
	#[case] minimum_id: i64,
	#[case] expected_calls: usize,
	#[case] expected_titles: Vec<&str>,
) {
	// Arrange
	let (_lease, mut executor) = sqlite_fixture.await;
	let queryset = QuerySet::<Author>::new()
		.prefetch_related(["book"])
		.filter(Filter::new(
			"id",
			FilterOperator::Gt,
			FilterValue::Integer(minimum_id),
		));
	// Act: execute the public prefetch plan through the real ORM executor.
	let authors = queryset.all_with_db(&mut executor).await.unwrap();
	let ids: Vec<_> = authors.iter().map(|author| author.id.unwrap()).collect();
	let mut titles = Vec::new();
	for (relation, statement) in queryset.prefetch_related_queries(&ids) {
		assert_eq!(relation, "book");
		let books = SelectExecution::<Book>::new(statement)
			.all_async(&mut executor)
			.await
			.unwrap();
		titles.extend(books.into_iter().map(|book| book.title));
	}
	titles.sort();
	// Assert: includes both children of Alice, excludes the unrelated orphan.
	assert_eq!(titles, expected_titles);
	assert_eq!(executor.calls, expected_calls);
}
