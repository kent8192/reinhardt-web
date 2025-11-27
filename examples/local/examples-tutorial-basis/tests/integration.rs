//! Integration tests for polls application
//!
//! These tests use the `postgres_with_migrations_from` fixture with
//! `PollsMigrations` provider to run database integration tests.

#[cfg(with_reinhardt)]
mod tests {
	use examples_tutorial_basis::apps::polls::migrations::PollsMigrations;
	use reinhardt::test::fixtures::postgres_with_migrations_from;
	use rstest::*;

	// Database integration tests using PollsMigrations provider
	#[rstest]
	#[tokio::test]
	async fn test_question_database_create() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create a question
		let question_text = "What's your favorite color?";
		let result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id, question_text, pub_date",
				vec![question_text.into()],
			)
			.await;

		assert!(result.is_ok());
		let row = result.unwrap();
		let retrieved_text: String = row.get::<String>("question_text").unwrap();
		assert_eq!(retrieved_text, question_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_database_read() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Insert test data
		let question_text = "Test question for reading";
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec![question_text.into()],
			)
			.await;

		assert!(insert_result.is_ok());
		let id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Read the question back
		let read_result = db
			.fetch_one(
				"SELECT id, question_text, pub_date FROM polls_question WHERE id = $1",
				vec![id.into()],
			)
			.await;

		assert!(read_result.is_ok());
		let row = read_result.unwrap();
		let retrieved_text: String = row.get::<String>("question_text").unwrap();
		assert_eq!(retrieved_text, question_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_database_update() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Insert initial data
		let original_text = "Original question text";
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec![original_text.into()],
			)
			.await;

		assert!(insert_result.is_ok());
		let id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Update the question
		let updated_text = "Updated question text";
		let update_result = db
			.execute(
				"UPDATE polls_question SET question_text = $1 WHERE id = $2",
				vec![updated_text.into(), id.into()],
			)
			.await;

		assert!(update_result.is_ok());

		// Verify update
		let verify_result = db
			.fetch_one(
				"SELECT question_text FROM polls_question WHERE id = $1",
				vec![id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		let retrieved_text: String = verify_result
			.unwrap()
			.get::<String>("question_text")
			.unwrap();
		assert_eq!(retrieved_text, updated_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_database_delete() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Insert test data
		let question_text = "Question to be deleted";
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec![question_text.into()],
			)
			.await;

		assert!(insert_result.is_ok());
		let id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Delete the question
		let delete_result = db
			.execute("DELETE FROM polls_question WHERE id = $1", vec![id.into()])
			.await;

		assert!(delete_result.is_ok());

		// Verify deletion
		let verify_result = db
			.fetch_optional(
				"SELECT id FROM polls_question WHERE id = $1",
				vec![id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		assert!(verify_result.unwrap().is_none());
	}

	// Debug test to expose actual database errors
	#[rstest]
	#[tokio::test]
	async fn test_debug_migrations() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Check if table exists
		let table_check = db
			.fetch_one(
				"SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'polls_question')",
				vec![],
			)
			.await;

		match &table_check {
			Ok(row) => eprintln!("✅ Table check succeeded: {:?}", row),
			Err(e) => eprintln!("❌ Table check failed: {:?}\nDetails: {}", e, e),
		}

		// Try simple insert
		match db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Test".into()],
			)
			.await
		{
			Ok(row) => eprintln!("✅ Insert succeeded: {:?}", row),
			Err(e) => eprintln!("❌ Insert failed: {:?}\nDetails: {}", e, e),
		}

		// This test will fail intentionally to display the error details
		assert!(false, "Debug test - check stderr output above");
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_create() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// First create a question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Test question for choice".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Create a choice
		let choice_text = "Test choice";
		let choice_result = db
			.fetch_one(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id, choice_text, votes",
				vec![question_id.into(), choice_text.into(), 0i32.into()],
			)
			.await;

		assert!(choice_result.is_ok());
		let row = choice_result.unwrap();
		let retrieved_text: String = row.get::<String>("choice_text").unwrap();
		let votes: i32 = row.get::<i32>("votes").unwrap();
		assert_eq!(retrieved_text, choice_text);
		assert_eq!(votes, 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_read() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Question for choice read test".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Insert choice
		let choice_text = "Choice to be read";
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
				vec![question_id.into(), choice_text.into(), 0i32.into()],
			)
			.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Read the choice back
		let read_result = db
			.fetch_one(
				"SELECT id, question_id, choice_text, votes FROM polls_choice WHERE id = $1",
				vec![choice_id.into()],
			)
			.await;

		assert!(read_result.is_ok());
		let row = read_result.unwrap();
		let retrieved_text: String = row.get::<String>("choice_text").unwrap();
		let retrieved_question_id: i64 = row.get::<i64>("question_id").unwrap();
		assert_eq!(retrieved_text, choice_text);
		assert_eq!(retrieved_question_id, question_id);
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_update() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Question for choice update test".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Insert choice
		let original_text = "Original choice text";
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
				vec![question_id.into(), original_text.into(), 0i32.into()],
			)
			.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Update the choice
		let updated_text = "Updated choice text";
		let update_result = db
			.execute(
				"UPDATE polls_choice SET choice_text = $1 WHERE id = $2",
				vec![updated_text.into(), choice_id.into()],
			)
			.await;

		assert!(update_result.is_ok());

		// Verify update
		let verify_result = db
			.fetch_one(
				"SELECT choice_text FROM polls_choice WHERE id = $1",
				vec![choice_id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		let retrieved_text: String = verify_result.unwrap().get::<String>("choice_text").unwrap();
		assert_eq!(retrieved_text, updated_text);
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_database_delete() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Question for choice delete test".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Insert choice
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
				vec![
					question_id.into(),
					"Choice to be deleted".into(),
					0i32.into(),
				],
			)
			.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Delete the choice
		let delete_result = db
			.execute(
				"DELETE FROM polls_choice WHERE id = $1",
				vec![choice_id.into()],
			)
			.await;

		assert!(delete_result.is_ok());

		// Verify deletion
		let verify_result = db
			.fetch_optional(
				"SELECT id FROM polls_choice WHERE id = $1",
				vec![choice_id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		assert!(verify_result.unwrap().is_none());
	}

	#[rstest]
	#[tokio::test]
	async fn test_choice_vote_increment() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Question for vote test".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Insert choice with 0 votes
		let insert_result = db
			.fetch_one(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3) RETURNING id",
				vec![question_id.into(), "Choice to vote for".into(), 0i32.into()],
			)
			.await;

		assert!(insert_result.is_ok());
		let choice_id: i64 = insert_result.unwrap().get::<i64>("id").unwrap();

		// Increment votes
		let update_result = db
			.execute(
				"UPDATE polls_choice SET votes = votes + 1 WHERE id = $1",
				vec![choice_id.into()],
			)
			.await;

		assert!(update_result.is_ok());

		// Verify vote count
		let verify_result = db
			.fetch_one(
				"SELECT votes FROM polls_choice WHERE id = $1",
				vec![choice_id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		let votes: i32 = verify_result.unwrap().get::<i32>("votes").unwrap();
		assert_eq!(votes, 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_choice_foreign_key() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Question for FK test".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Insert multiple choices for the question
		let choice1_result = db
			.execute(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
				vec![question_id.into(), "Choice 1".into(), 0i32.into()],
			)
			.await;

		assert!(choice1_result.is_ok());

		let choice2_result = db
			.execute(
				"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
				vec![question_id.into(), "Choice 2".into(), 0i32.into()],
			)
			.await;

		assert!(choice2_result.is_ok());

		// Verify all choices belong to the question
		let verify_result = db
			.fetch_one(
				"SELECT COUNT(*) as count FROM polls_choice WHERE question_id = $1",
				vec![question_id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		let count: i64 = verify_result.unwrap().get::<i64>("count").unwrap();
		assert_eq!(count, 2);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_cascade_delete() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Create question
		let question_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id",
				vec!["Question for cascade delete test".into()],
			)
			.await;

		assert!(question_result.is_ok());
		let question_id: i64 = question_result.unwrap().get::<i64>("id").unwrap();

		// Insert choices
		db.execute(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
			vec![question_id.into(), "Choice 1".into(), 0i32.into()],
		)
		.await
		.unwrap();

		db.execute(
			"INSERT INTO polls_choice (question_id, choice_text, votes) VALUES ($1, $2, $3)",
			vec![question_id.into(), "Choice 2".into(), 0i32.into()],
		)
		.await
		.unwrap();

		// Delete the question (should cascade to choices)
		let delete_result = db
			.execute(
				"DELETE FROM polls_question WHERE id = $1",
				vec![question_id.into()],
			)
			.await;

		assert!(delete_result.is_ok());

		// Verify choices were also deleted
		let verify_result = db
			.fetch_one(
				"SELECT COUNT(*) as count FROM polls_choice WHERE question_id = $1",
				vec![question_id.into()],
			)
			.await;

		assert!(verify_result.is_ok());
		let count: i64 = verify_result.unwrap().get::<i64>("count").unwrap();
		assert_eq!(count, 0);
	}

	#[rstest]
	#[tokio::test]
	async fn test_question_recent_pub_date() {
		let (_container, db) = postgres_with_migrations_from::<PollsMigrations>().await;

		// Insert a recent question (published now)
		let recent_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW()) RETURNING id, pub_date",
				vec!["Recent question".into()],
			)
			.await;

		assert!(recent_result.is_ok());
		let recent_row = recent_result.unwrap();
		let recent_pub_date: chrono::DateTime<chrono::Utc> = recent_row
			.get::<chrono::DateTime<chrono::Utc>>("pub_date")
			.unwrap();

		// Verify it's recent (within last minute)
		let now = chrono::Utc::now();
		let diff = now - recent_pub_date;
		assert!(diff.num_seconds() < 60);

		// Insert an old question (published 2 days ago)
		let old_result = db
			.fetch_one(
				"INSERT INTO polls_question (question_text, pub_date) VALUES ($1, NOW() - INTERVAL '2 days') RETURNING id, pub_date",
				vec!["Old question".into()],
			)
			.await;

		assert!(old_result.is_ok());
		let old_row = old_result.unwrap();
		let old_pub_date: chrono::DateTime<chrono::Utc> = old_row
			.get::<chrono::DateTime<chrono::Utc>>("pub_date")
			.unwrap();

		// Verify it's old (more than 1 day ago)
		let old_diff = now - old_pub_date;
		assert!(old_diff.num_days() >= 1);
	}
}
