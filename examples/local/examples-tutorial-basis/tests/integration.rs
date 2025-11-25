//! Integration tests for polls application

#[cfg(with_reinhardt)]
mod tests {
	use example_test_macros::example_test;
	use rstest::*;
	use std::sync::Arc;
	use testcontainers::ContainerAsync;
	use testcontainers_modules::postgres::Postgres;

	// Basic unit tests for model construction
	#[test]
	fn test_question_model() {
		use examples_tutorial_basis::apps::polls::models::Question;

		let question = Question::new("What's new?".to_string());
		assert_eq!(question.question_text, "What's new?");
		assert!(question.was_published_recently());
	}

	#[test]
	fn test_choice_model() {
		use examples_tutorial_basis::apps::polls::models::Choice;

		let mut choice = Choice::new(1, "Not much".to_string());
		assert_eq!(choice.choice_text, "Not much");
		assert_eq!(choice.votes, 0);

		choice.vote();
		assert_eq!(choice.votes, 1);
	}

	// Database integration tests
	#[rstest]
	#[tokio::test]
	async fn test_question_database_create(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		use examples_tutorial_basis::apps::polls::models::Question;

		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create a question
		let question_text = "What's your favorite color?";
		let result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id, question_text, pub_date"
		)
		.bind(question_text)
		.fetch_one(pool.as_ref())
		.await;

		assert!(result.is_ok());
		let row = result.unwrap();
		let retrieved_text: String = row.get("question_text");
		assert_eq!(retrieved_text, question_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_database_read(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		use examples_tutorial_basis::apps::polls::models::Question;

		let (_container, pool, _url) = postgres_with_migrations.await;

		// Insert test data
		let question_text = "Test question for reading";
		let insert_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind(question_text)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let id: i64 = insert_result.unwrap().get("id");

		// Read the question back
		let read_result =
			sqlx::query("SELECT id, question_text, pub_date FROM polls_question WHERE id = $1")
				.bind(id)
				.fetch_one(pool.as_ref())
				.await;

		assert!(read_result.is_ok());
		let row = read_result.unwrap();
		let retrieved_text: String = row.get("question_text");
		assert_eq!(retrieved_text, question_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_database_update(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Insert initial data
		let original_text = "Original question text";
		let insert_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind(original_text)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let id: i64 = insert_result.unwrap().get("id");

		// Update the question
		let updated_text = "Updated question text";
		let update_result =
			sqlx::query("UPDATE polls_question SET question_text = $1 WHERE id = $2")
				.bind(updated_text)
				.bind(id)
				.execute(pool.as_ref())
				.await;

		assert!(update_result.is_ok());

		// Verify update
		let verify_result = sqlx::query("SELECT question_text FROM polls_question WHERE id = $1")
			.bind(id)
			.fetch_one(pool.as_ref())
			.await;

		assert!(verify_result.is_ok());
		let retrieved_text: String = verify_result.unwrap().get("question_text");
		assert_eq!(retrieved_text, updated_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_database_delete(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Insert test data
		let question_text = "Question to be deleted";
		let insert_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind(question_text)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let id: i64 = insert_result.unwrap().get("id");

		// Delete the question
		let delete_result = sqlx::query("DELETE FROM polls_question WHERE id = $1")
			.bind(id)
			.execute(pool.as_ref())
			.await;

		assert!(delete_result.is_ok());

		// Verify deletion
		let verify_result = sqlx::query("SELECT id FROM polls_question WHERE id = $1")
			.bind(id)
			.fetch_optional(pool.as_ref())
			.await;

		assert!(verify_result.is_ok());
		assert!(verify_result.unwrap().is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_create(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// First create a question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Test question for choice")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Create a choice
		let choice_text = "Test choice";
		let choice_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id, choice_text, votes"
		)
		.bind(question_id)
		.bind(choice_text)
		.bind(0)
		.fetch_one(pool.as_ref())
		.await;

		assert!(choice_result.is_ok());
		let row = choice_result.unwrap();
		let retrieved_text: String = row.get("choice_text");
		let votes: i32 = row.get("votes");
		assert_eq!(retrieved_text, choice_text);
		assert_eq!(votes, 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_read(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Question for choice read test")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Insert choice
		let choice_text = "Choice to be read";
		let insert_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
		)
		.bind(question_id)
		.bind(choice_text)
		.bind(0)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get("id");

		// Read the choice back
		let read_result = sqlx::query(
			"SELECT id, question_id, choice_text, votes FROM polls_choice WHERE id = $1",
		)
		.bind(choice_id)
		.fetch_one(pool.as_ref())
		.await;

		assert!(read_result.is_ok());
		let row = read_result.unwrap();
		let retrieved_text: String = row.get("choice_text");
		let retrieved_question_id: i64 = row.get("question_id");
		assert_eq!(retrieved_text, choice_text);
		assert_eq!(retrieved_question_id, question_id);
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_update(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Question for choice update test")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Insert choice
		let original_text = "Original choice text";
		let insert_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
		)
		.bind(question_id)
		.bind(original_text)
		.bind(0)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get("id");

		// Update the choice
		let updated_text = "Updated choice text";
		let update_result = sqlx::query("UPDATE polls_choice SET choice_text = $1 WHERE id = $2")
			.bind(updated_text)
			.bind(choice_id)
			.execute(pool.as_ref())
			.await;

		assert!(update_result.is_ok());

		// Verify update
		let verify_result = sqlx::query("SELECT choice_text FROM polls_choice WHERE id = $1")
			.bind(choice_id)
			.fetch_one(pool.as_ref())
			.await;

		assert!(verify_result.is_ok());
		let retrieved_text: String = verify_result.unwrap().get("choice_text");
		assert_eq!(retrieved_text, updated_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_delete(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Question for choice delete test")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Insert choice
		let insert_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
		)
		.bind(question_id)
		.bind("Choice to be deleted")
		.bind(0)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get("id");

		// Delete the choice
		let delete_result = sqlx::query("DELETE FROM polls_choice WHERE id = $1")
			.bind(choice_id)
			.execute(pool.as_ref())
			.await;

		assert!(delete_result.is_ok());

		// Verify deletion
		let verify_result = sqlx::query("SELECT id FROM polls_choice WHERE id = $1")
			.bind(choice_id)
			.fetch_optional(pool.as_ref())
			.await;

		assert!(verify_result.is_ok());
		assert!(verify_result.unwrap().is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_vote_increment(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Question for vote test")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Insert choice with 0 votes
		let insert_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
		)
		.bind(question_id)
		.bind("Choice to vote for")
		.bind(0)
		.fetch_one(pool.as_ref())
		.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get("id");

		// Increment votes
		let update_result = sqlx::query("UPDATE polls_choice SET votes = votes + 1 WHERE id = $1")
			.bind(choice_id)
			.execute(pool.as_ref())
			.await;

		assert!(update_result.is_ok());

		// Verify vote count
		let verify_result = sqlx::query("SELECT votes FROM polls_choice WHERE id = $1")
			.bind(choice_id)
			.fetch_one(pool.as_ref())
			.await;

		assert!(verify_result.is_ok());
		let votes: i32 = verify_result.unwrap().get("votes");
		assert_eq!(votes, 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_choice_foreign_key(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Question for FK test")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Insert multiple choices for the question
		let choice1_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
		)
		.bind(question_id)
		.bind("Choice 1")
		.bind(0)
		.execute(pool.as_ref())
		.await;

		assert!(choice1_result.is_ok());

		let choice2_result = sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
		)
		.bind(question_id)
		.bind("Choice 2")
		.bind(0)
		.execute(pool.as_ref())
		.await;

		assert!(choice2_result.is_ok());

		// Verify all choices belong to the question
		let verify_result =
			sqlx::query("SELECT COUNT(*) as count FROM polls_choice WHERE question_id = $1")
				.bind(question_id)
				.fetch_one(pool.as_ref())
				.await;

		assert!(verify_result.is_ok());
		let count: i64 = verify_result.unwrap().get("count");
		assert_eq!(count, 2);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_cascade_delete(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Create question
		let question_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
		)
		.bind("Question for cascade delete test")
		.fetch_one(pool.as_ref())
		.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get("id");

		// Insert choices
		sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
		)
		.bind(question_id)
		.bind("Choice 1")
		.bind(0)
		.execute(pool.as_ref())
		.await
		.unwrap();

		sqlx::query(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
		)
		.bind(question_id)
		.bind("Choice 2")
		.bind(0)
		.execute(pool.as_ref())
		.await
		.unwrap();

		// Delete the question (should cascade to choices)
		let delete_result = sqlx::query("DELETE FROM polls_question WHERE id = $1")
			.bind(question_id)
			.execute(pool.as_ref())
			.await;

		assert!(delete_result.is_ok());

		// Verify choices were also deleted
		let verify_result =
			sqlx::query("SELECT COUNT(*) as count FROM polls_choice WHERE question_id = $1")
				.bind(question_id)
				.fetch_one(pool.as_ref())
				.await;

		assert!(verify_result.is_ok());
		let count: i64 = verify_result.unwrap().get("count");
		assert_eq!(count, 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_recent_pub_date(
		#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<sqlx::PgPool>, String),
	) {
		let (_container, pool, _url) = postgres_with_migrations.await;

		// Insert a recent question (published now)
		let recent_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id, pub_date",
		)
		.bind("Recent question")
		.fetch_one(pool.as_ref())
		.await;

		assert!(recent_result.is_ok());
		let recent_row = recent_result.unwrap();
		let recent_pub_date: chrono::DateTime<chrono::Utc> = recent_row.get("pub_date");

		// Verify it's recent (within last minute)
		let now = chrono::Utc::now();
		let diff = now - recent_pub_date;
		assert!(diff.num_seconds() < 60);

		// Insert an old question (published 2 days ago)
		let old_result = sqlx::query(
			"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW() - INTERVAL '2 days') RETURNING id, pub_date"
		)
		.bind("Old question")
		.fetch_one(pool.as_ref())
		.await;

		assert!(old_result.is_ok());
		let old_row = old_result.unwrap();
		let old_pub_date: chrono::DateTime<chrono::Utc> = old_row.get("pub_date");

		// Verify it's old (more than 1 day ago)
		let old_diff = now - old_pub_date;
		assert!(old_diff.num_days() >= 1);
	}
}
