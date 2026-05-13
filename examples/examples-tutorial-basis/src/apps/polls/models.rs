use chrono::{DateTime, Utc};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

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

	// ⚠️ IMPORTANT: related_name is REQUIRED for #[rel(foreign_key)]
	#[rel(foreign_key, related_name = "choices")]
	pub question: ForeignKeyField<Question>,

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
	use rstest::rstest;

	#[rstest]
	fn test_choice_vote() {
		// Positional `new()` constructor — concise but field-order-sensitive.
		// Adding a new required field to `Choice` would force every call site
		// like this one to be rewritten in the same commit.
		let mut choice = Choice::new(
			"Choice 1".to_string(), // choice_text
			0,                      // votes
			1,                      // question_id (ForeignKeyField is last)
		);
		assert_eq!(choice.votes(), 0);

		choice.vote();
		assert_eq!(choice.votes(), 1);

		choice.vote();
		assert_eq!(choice.votes(), 2);
	}

	#[rstest]
	fn test_choice_vote_via_typestate_builder() {
		// Typestate `build()` constructor (added in issue #4400).
		//
		// Every required field is set by name through a dedicated setter, so
		// adding a new required field becomes a non-breaking change for this
		// call site — the new field shows up as an additional setter rather
		// than a new positional parameter that breaks every caller in
		// lock-step. Omitting any required setter is a compile-time error
		// thanks to the per-field typestate (no `.finish()` until every slot
		// is `Set`).
		let mut choice = Choice::build()
			.choice_text("Choice 1")
			.votes(0)
			.question(1_i64) // FK accepts `IntoPrimaryKey` — either `&Question` or raw PK.
			.finish();
		assert_eq!(choice.votes(), 0);

		choice.vote();
		assert_eq!(choice.votes(), 1);
	}

	#[rstest]
	fn test_question_build_typestate() {
		// `Question::build()` mirrors `Question::new(question_text)` but
		// surfaces each required field as a named setter, which keeps tutorial
		// call sites stable as the `Question` schema grows.
		let question = Question::build()
			.question_text("What's your favorite color?")
			.finish();
		assert_eq!(question.question_text(), "What's your favorite color?");
		// `pub_date` is `auto_now_add`, so `finish()` populates it just like
		// `new()` would.
		assert!(question.was_published_recently());
	}
}
