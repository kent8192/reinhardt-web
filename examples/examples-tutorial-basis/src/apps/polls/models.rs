use chrono::{DateTime, Utc};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

use crate::apps::users::models::User;

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
