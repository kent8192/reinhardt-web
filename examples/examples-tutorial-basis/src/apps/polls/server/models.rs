use chrono::{DateTime, Utc};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

use crate::apps::users::server::models::User;

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

	// Author of the question. Only the author can edit or delete it
	// (enforced server-side in `crate::apps::polls::server_fn`).
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

	// ⚠️ IMPORTANT: related_name is REQUIRED for #[rel(foreign_key)]
	#[rel(foreign_key, related_name = "choices")]
	pub question: ForeignKeyField<Question>,

	#[field(max_length = 200)]
	pub choice_text: String,

	#[field(default = 0)]
	pub votes: i32,
}

#[cfg(native)]
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

#[cfg(all(test, native))]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_question_build_typestate() {
		// `Question::build()` surfaces each required field as a named setter,
		// which keeps tutorial call sites stable as the `Question` schema grows.
		// The same typestate constructor (introduced for `#[model]` in issue
		// #4400 and extended to FK fields in #4413) is also used by
		// `Choice::build()` in the integration / wasm tests, where the model
		// has a live database backing it. Persisting `vote()` therefore
		// belongs in those tests, not in this synchronous unit test.
		let question = Question::build()
			.question_text("What's your favorite color?")
			.author(1_i64)
			.finish();
		assert_eq!(question.question_text(), "What's your favorite color?");
		// `pub_date` is `auto_now_add`, so `finish()` populates it just like
		// `new()` would.
		assert!(question.was_published_recently());
	}
}
