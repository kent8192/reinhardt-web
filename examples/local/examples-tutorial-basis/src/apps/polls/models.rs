use chrono::{DateTime, Utc};
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

/// Question model representing a poll question
#[derive(Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
#[model(app_label = "polls", table_name = "choices")]
pub struct Choice {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(foreign_key = Question)]
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
