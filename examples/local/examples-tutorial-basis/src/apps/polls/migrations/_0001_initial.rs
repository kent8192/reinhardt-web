use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Iden, PostgresQueryBuilder, Table};

/// Initial migration creating questions and choices tables
pub struct Migration;

impl Migration {
	/// Apply migration - create tables
	pub fn up() -> String {
		// CreateTable for questions
		let questions_table = Table::create()
			.table(QuestionsTable::Table)
			.col(
				ColumnDef::new(QuestionsTable::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(QuestionsTable::QuestionText).text().not_null())
			.col(
				ColumnDef::new(QuestionsTable::PubDate)
					.timestamp_with_time_zone()
					.not_null(),
			)
			.to_owned();

		// CreateTable for choices
		let choices_table = Table::create()
			.table(ChoicesTable::Table)
			.col(
				ColumnDef::new(ChoicesTable::Id)
					.big_integer()
					.auto_increment()
					.primary_key()
					.not_null(),
			)
			.col(ColumnDef::new(ChoicesTable::QuestionId).big_integer().not_null())
			.col(ColumnDef::new(ChoicesTable::ChoiceText).text().not_null())
			.col(
				ColumnDef::new(ChoicesTable::Votes)
					.integer()
					.not_null()
					.default(0),
			)
			.foreign_key(
				&mut ForeignKey::create()
					.from(ChoicesTable::Table, ChoicesTable::QuestionId)
					.to(QuestionsTable::Table, QuestionsTable::Id)
					.on_delete(ForeignKeyAction::Cascade)
			)
			.to_owned();

		format!(
			"{};\n\n{};",
			questions_table.to_string(PostgresQueryBuilder),
			choices_table.to_string(PostgresQueryBuilder)
		)
	}

	/// Rollback migration - drop tables
	pub fn down() -> String {
		let drop_choices = Table::drop().table(ChoicesTable::Table).to_owned();

		let drop_questions = Table::drop().table(QuestionsTable::Table).to_owned();

		format!(
			"{};\n\n{};",
			drop_choices.to_string(PostgresQueryBuilder),
			drop_questions.to_string(PostgresQueryBuilder)
		)
	}
}

/// Questions table identifier
#[derive(Iden)]
enum QuestionsTable {
	Table,
	Id,
	QuestionText,
	PubDate,
}

/// Choices table identifier
#[derive(Iden)]
enum ChoicesTable {
	Table,
	Id,
	QuestionId,
	ChoiceText,
	Votes,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_migration_up() {
		let sql = Migration::up();
		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("questions"));
		assert!(sql.contains("choices"));
		assert!(sql.contains("question_text"));
		assert!(sql.contains("pub_date"));
		assert!(sql.contains("choice_text"));
		assert!(sql.contains("votes"));
		assert!(sql.contains("FOREIGN KEY"));
	}

	#[test]
	fn test_migration_down() {
		let sql = Migration::down();
		assert!(sql.contains("DROP TABLE"));
		assert!(sql.contains("choices"));
		assert!(sql.contains("questions"));
	}
}
