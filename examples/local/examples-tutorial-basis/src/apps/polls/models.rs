use chrono::{DateTime, Utc};
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

/// Question model representing a poll question
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "polls", table_name = "questions")]
pub struct Question {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 200)]
	pub question_text: String,

	#[field(auto_now_add = true)]
	pub pub_date: DateTime<Utc>,
}

impl Question {
	/// Check if the question was published recently (within last day)
	pub fn was_published_recently(&self) -> bool {
		let now = Utc::now();
		let one_day_ago = now - chrono::Duration::days(1);
		self.pub_date >= one_day_ago && self.pub_date <= now
	}
}

/// Choice model representing an answer option for a question
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[model(app_label = "polls", table_name = "choices")]
pub struct Choice {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(foreign_key = "Question")]
	pub question_id: i64,

	#[field(max_length = 200)]
	pub choice_text: String,

	#[field(default = 0)]
	pub votes: i32,
}

impl Choice {
	/// Increment the vote count
	pub fn vote(&mut self) {
		self.votes += 1;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Duration;

	// TODO: Add database integration tests using TestContainers
	// Example:
	// use reinhardt::db::orm::Manager;
	// use reinhardt_test::fixtures::postgres_container;
	// use rstest::*;
	//
	// #[rstest]
	// #[tokio::test]
	// async fn test_question_crud_operations(
	//     #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>)
	// ) {
	//     let (_container, db) = postgres_container.await;
	//     let manager = Manager::<Question>::new();
	//
	//     // Test create
	//     let question = Question {
	//         id: 0,
	//         question_text: "What's your favorite color?".to_string(),
	//         pub_date: Utc::now(),
	//     };
	//     let created = manager.create(question).await.unwrap();
	//
	//     // Test was_published_recently
	//     assert!(created.was_published_recently());
	//
	//     // Test with old date
	//     let mut old_question = created.clone();
	//     old_question.pub_date = Utc::now() - Duration::days(2);
	//     assert!(!old_question.was_published_recently());
	// }

	#[test]
	fn test_choice_vote() {
		let mut choice = Choice {
			id: 1,
			question_id: 1,
			choice_text: "Choice 1".to_string(),
			votes: 0,
		};
		assert_eq!(choice.votes, 0);

		choice.vote();
		assert_eq!(choice.votes, 1);

		choice.vote();
		assert_eq!(choice.votes, 2);
	}
}
