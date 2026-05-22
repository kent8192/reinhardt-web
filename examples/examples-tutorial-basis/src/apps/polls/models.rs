use crate::apps::users::models::User;
use chrono::{DateTime, Utc};
use reinhardt::core::serde::{Deserialize, Serialize};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
/// Question model representing a poll question
#[model(app_label = "polls", table_name = "questions")]
#[derive(Serialize, Deserialize)]
pub struct Question {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 200)]
	pub question_text: String,
	#[field(auto_now_add = true)]
	pub pub_date: DateTime<Utc>,
	#[rel(foreign_key, related_name = "questions")]
	pub author: ForeignKeyField<User>,
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
#[model(app_label = "polls", table_name = "choices")]
#[derive(Serialize, Deserialize)]
pub struct Choice {
	#[field(primary_key = true)]
	pub id: i64,
	#[rel(foreign_key, related_name = "choices")]
	pub question: ForeignKeyField<Question>,
	#[field(max_length = 200)]
	pub choice_text: String,
	#[field(default = 0)]
	pub votes: i32,
}
impl Choice {
	/// Increment the vote count and persist it.
	///
	/// Uses Django-style `Model::save()` (see
	/// `crates/reinhardt-db/src/orm/model.rs` `Model::save`) so the row is
	/// updated in place; `before_update` / `after_update` signals fire as part
	/// of the standard model lifecycle. Call sites can therefore drop a
	/// separate `manager.update(&choice).await?` and treat `vote()` as the
	/// canonical "increment + flush" operation.
	pub async fn vote(&mut self) -> reinhardt::core::exception::Result<()> {
		self.votes += 1;
		self.save().await
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	#[rstest]
	fn test_question_build_typestate() {
		let question = Question::build()
			.question_text("What's your favorite color?")
			.author(1_i64)
			.finish();
		assert_eq!(question.question_text(), "What's your favorite color?");
		assert!(question.was_published_recently());
	}
}
