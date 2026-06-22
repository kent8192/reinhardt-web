use reinhardt::Validate;
use serde::{Deserialize, Serialize};

pub use crate::shared::types::{ChoiceInfo, QuestionInfo};

/// Serializer for creating/updating questions
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct QuestionSerializer {
	#[validate(length(
		min = 1,
		max = 200,
		message = "Question text must be between 1 and 200 characters"
	))]
	pub question_text: String,
}

/// Serializer for creating/updating choices
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ChoiceSerializer {
	pub question_id: i64,

	#[validate(length(
		min = 1,
		max = 200,
		message = "Choice text must be between 1 and 200 characters"
	))]
	pub choice_text: String,
}

#[cfg(test)]
mod tests {
	use crate::apps::polls::server::serializers::*;
	use rstest::rstest;

	#[rstest]
	fn test_question_serializer_valid() {
		let serializer = QuestionSerializer {
			question_text: "What's your favorite color?".to_string(),
		};
		assert!(serializer.validate().is_ok());
	}

	#[rstest]
	fn test_question_serializer_empty_text() {
		let serializer = QuestionSerializer {
			question_text: String::new(),
		};
		assert!(serializer.validate().is_err());
	}

	#[rstest]
	fn test_question_serializer_too_long() {
		let serializer = QuestionSerializer {
			question_text: "a".repeat(201),
		};
		assert!(serializer.validate().is_err());
	}

	#[rstest]
	fn test_choice_serializer_valid() {
		let serializer = ChoiceSerializer {
			question_id: 1,
			choice_text: "Red".to_string(),
		};
		assert!(serializer.validate().is_ok());
	}

	#[rstest]
	fn test_choice_serializer_empty_text() {
		let serializer = ChoiceSerializer {
			question_id: 1,
			choice_text: String::new(),
		};
		assert!(serializer.validate().is_err());
	}
}
