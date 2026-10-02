/// Database constraints similar to Django's constraints
use serde::{Deserialize, Serialize};

/// Base trait for all constraints
pub trait Constraint {
	/// Converts the constraint to its SQL representation.
	fn to_sql(&self) -> String;
	/// Returns the constraint name.
	fn name(&self) -> &str;
}

/// CHECK constraint (similar to Django's CheckConstraint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraint {
	/// The name.
	pub name: String,
	/// The check.
	pub check: String,
}

impl CheckConstraint {
	/// Create a CHECK constraint to validate field values
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::constraints::CheckConstraint;
	///
	/// let constraint = CheckConstraint::new("age_check", "age >= 18");
	/// assert_eq!(constraint.name, "age_check");
	/// assert_eq!(constraint.check, "age >= 18");
	/// ```
	pub fn new(name: impl Into<String>, check: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			check: check.into(),
		}
	}
}

impl Constraint for CheckConstraint {
	fn to_sql(&self) -> String {
		format!("CONSTRAINT {} CHECK ({})", self.name, self.check)
	}

	fn name(&self) -> &str {
		&self.name
	}
}

/// UNIQUE constraint (similar to Django's UniqueConstraint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraint {
	/// The name.
	pub name: String,
	/// The fields.
	pub fields: Vec<String>,
	/// The condition.
	pub condition: Option<String>, // Partial unique constraint
}

impl UniqueConstraint {
	/// Create a UNIQUE constraint on one or more fields
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::constraints::UniqueConstraint;
	///
	/// let constraint = UniqueConstraint::new("email_unique", vec!["email".to_string()]);
	/// assert_eq!(constraint.name, "email_unique");
	/// assert_eq!(constraint.fields.len(), 1);
	/// ```
	pub fn new(name: impl Into<String>, fields: Vec<String>) -> Self {
		Self {
			name: name.into(),
			fields,
			condition: None,
		}
	}
	/// Documentation for `with_condition`
	pub fn with_condition(mut self, condition: String) -> Self {
		self.condition = Some(condition);
		self
	}
}

impl Constraint for UniqueConstraint {
	fn to_sql(&self) -> String {
		let fields = self.fields.join(", ");
		let mut sql = format!("CONSTRAINT {} UNIQUE ({})", self.name, fields);
		if let Some(ref cond) = self.condition {
			sql.push_str(&format!(" WHERE {}", cond));
		}
		sql
	}

	fn name(&self) -> &str {
		&self.name
	}
}

/// Foreign Key constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyConstraint {
	/// The name.
	pub name: String,
	/// The field.
	pub field: String,
	/// The references table.
	pub references_table: String,
	/// The references field.
	pub references_field: String,
	/// The on delete.
	pub on_delete: OnDelete,
	/// The on update.
	pub on_update: OnUpdate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Defines possible on delete values.
pub enum OnDelete {
	/// Cascade variant.
	Cascade,
	/// SetNull variant.
	SetNull,
	/// SetDefault variant.
	SetDefault,
	/// Restrict variant.
	Restrict,
	/// NoAction variant.
	NoAction,
}

impl OnDelete {
	/// Documentation for `to_sql`
	///
	pub fn to_sql(&self) -> &'static str {
		match self {
			OnDelete::Cascade => "CASCADE",
			OnDelete::SetNull => "SET NULL",
			OnDelete::SetDefault => "SET DEFAULT",
			OnDelete::Restrict => "RESTRICT",
			OnDelete::NoAction => "NO ACTION",
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Defines possible on update values.
pub enum OnUpdate {
	/// Cascade variant.
	Cascade,
	/// SetNull variant.
	SetNull,
	/// SetDefault variant.
	SetDefault,
	/// Restrict variant.
	Restrict,
	/// NoAction variant.
	NoAction,
}

impl OnUpdate {
	/// Documentation for `to_sql`
	///
	pub fn to_sql(&self) -> &'static str {
		match self {
			OnUpdate::Cascade => "CASCADE",
			OnUpdate::SetNull => "SET NULL",
			OnUpdate::SetDefault => "SET DEFAULT",
			OnUpdate::Restrict => "RESTRICT",
			OnUpdate::NoAction => "NO ACTION",
		}
	}
}

impl ForeignKeyConstraint {
	/// Create a FOREIGN KEY constraint referencing another table
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::orm::constraints::ForeignKeyConstraint;
	///
	/// let fk = ForeignKeyConstraint::new(
	///     "user_fk",
	///     "user_id",
	///     "users",
	///     "id"
	/// );
	/// assert_eq!(fk.name, "user_fk");
	/// assert_eq!(fk.references_table, "users");
	/// ```
	pub fn new(
		name: impl Into<String>,
		field: impl Into<String>,
		references_table: impl Into<String>,
		references_field: impl Into<String>,
	) -> Self {
		Self {
			name: name.into(),
			field: field.into(),
			references_table: references_table.into(),
			references_field: references_field.into(),
			on_delete: OnDelete::Restrict,
			on_update: OnUpdate::NoAction,
		}
	}
	/// Documentation for `on_delete`
	///
	pub fn on_delete(mut self, on_delete: OnDelete) -> Self {
		self.on_delete = on_delete;
		self
	}
	/// Documentation for `on_update`
	///
	pub fn on_update(mut self, on_update: OnUpdate) -> Self {
		self.on_update = on_update;
		self
	}
}

impl Constraint for ForeignKeyConstraint {
	fn to_sql(&self) -> String {
		format!(
			"CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE {} ON UPDATE {}",
			self.name,
			self.field,
			self.references_table,
			self.references_field,
			self.on_delete.to_sql(),
			self.on_update.to_sql()
		)
	}

	fn name(&self) -> &str {
		&self.name
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[rstest::rstest]
	#[case("age_check", "age >= 0", "CONSTRAINT age_check CHECK (age >= 0)")]
	#[case(
		"test_constraint",
		"value > 0",
		"CONSTRAINT test_constraint CHECK (value > 0)"
	)]
	#[case(
		"test_constraint",
		"age >= 18",
		"CONSTRAINT test_constraint CHECK (age >= 18)"
	)]
	fn test_constraints_check(#[case] name: &str, #[case] condition: &str, #[case] sql: &str) {
		let constraint = CheckConstraint::new(name, condition);
		assert_eq!(constraint.to_sql(), sql);
		assert_eq!(constraint.name(), name);
	}

	#[test]
	fn test_unique_constraint() {
		let constraint = UniqueConstraint::new("unique_email", vec!["email".to_string()]);
		assert_eq!(
			constraint.to_sql(),
			"CONSTRAINT unique_email UNIQUE (email)"
		);
	}

	#[test]
	fn test_unique_constraint_multiple_fields() {
		let constraint = UniqueConstraint::new(
			"unique_user_email",
			vec!["user_id".to_string(), "email".to_string()],
		);
		assert_eq!(
			constraint.to_sql(),
			"CONSTRAINT unique_user_email UNIQUE (user_id, email)"
		);
	}

	#[test]
	fn test_unique_constraint_with_condition() {
		let constraint = UniqueConstraint::new("unique_active_email", vec!["email".to_string()])
			.with_condition("deleted_at IS NULL".to_string());
		assert_eq!(
			constraint.to_sql(),
			"CONSTRAINT unique_active_email UNIQUE (email) WHERE deleted_at IS NULL"
		);
	}

	#[test]
	fn test_constraints_foreign_key_constraint() {
		let constraint = ForeignKeyConstraint::new("fk_user", "user_id", "users", "id");
		let sql = constraint.to_sql();
		assert_eq!(
			sql,
			"CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT ON UPDATE NO ACTION",
			"Expected exact foreign key constraint SQL, got: {}",
			sql
		);
	}

	#[test]
	fn test_foreign_key_cascade() {
		let constraint = ForeignKeyConstraint::new("fk_post", "post_id", "posts", "id")
			.on_delete(OnDelete::Cascade)
			.on_update(OnUpdate::Cascade);
		let sql = constraint.to_sql();
		assert_eq!(
			sql,
			"CONSTRAINT fk_post FOREIGN KEY (post_id) REFERENCES posts (id) ON DELETE CASCADE ON UPDATE CASCADE",
			"Expected exact foreign key constraint SQL with CASCADE actions, got: {}",
			sql
		);
	}

	#[test]
	fn test_foreign_key_set_null() {
		let constraint = ForeignKeyConstraint::new("fk_author", "author_id", "users", "id")
			.on_delete(OnDelete::SetNull);
		let sql = constraint.to_sql();
		assert_eq!(
			sql,
			"CONSTRAINT fk_author FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL ON UPDATE NO ACTION",
			"Expected exact foreign key constraint SQL with SET NULL action, got: {}",
			sql
		);
	}
}
