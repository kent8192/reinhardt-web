//! Integration tests for reinhardt-orm
//!
//! These tests verify the public API of the crate

use reinhardt_orm::{
	Abs, AnnotationValue, Cast, Concat, Greatest, IsolationLevel, Lower, Model, Round,
	SoftDeletable, SoftDelete, SqlType, Sqrt, Timestamped, Timestamps, Transaction,
	TransactionState, Upper, Value, F, Q,
};
use reinhardt_core::validators::TableName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestUser {
	id: Option<i64>,
	username: String,
	email: String,
	timestamps: Timestamps,
	soft_delete: SoftDelete,
}

const TEST_USER_TABLE: TableName = TableName::new_const("test_user");

impl Model for TestUser {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		TEST_USER_TABLE.as_str()
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

impl Timestamped for TestUser {
	fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
		self.timestamps.created_at
	}

	fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
		self.timestamps.updated_at
	}

	fn set_updated_at(&mut self, time: chrono::DateTime<chrono::Utc>) {
		self.timestamps.updated_at = time;
	}
}

impl SoftDeletable for TestUser {
	fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
		self.soft_delete.deleted_at
	}

	fn set_deleted_at(&mut self, time: Option<chrono::DateTime<chrono::Utc>>) {
		self.soft_delete.deleted_at = time;
	}
}

#[test]
fn test_model_trait() {
	let mut user = TestUser {
		id: None,
		username: "testuser".to_string(),
		email: "test@example.com".to_string(),
		timestamps: Timestamps::now(),
		soft_delete: SoftDelete::new(),
	};

	assert_eq!(TestUser::table_name(), "test_user");
	assert_eq!(user.primary_key(), None);

	user.set_primary_key(1);
	assert_eq!(user.primary_key(), Some(&1));
}

#[test]
fn test_timestamps() {
	let timestamps = Timestamps::now();
	assert!(timestamps.created_at <= chrono::Utc::now());
	assert!(timestamps.updated_at <= chrono::Utc::now());

	let mut ts = timestamps.clone();
	std::thread::sleep(std::time::Duration::from_millis(10));
	ts.touch();
	assert!(ts.updated_at > timestamps.updated_at);
}

#[test]
fn test_soft_delete() {
	let mut soft_delete = SoftDelete::new();
	assert!(!soft_delete.deleted_at.is_some());

	soft_delete.delete();
	assert!(soft_delete.deleted_at.is_some());

	soft_delete.restore();
	assert!(!soft_delete.deleted_at.is_some());
}

// Core expression tests (F and Q)
#[test]
fn test_orm_integration_f_expression() {
	let f = F::new("total_price");
	assert_eq!(f.to_sql(), "total_price");
}

#[test]
fn test_q_expression() {
	let q = Q::new("age", ">=", "18").and(Q::new("active", "=", "true"));
	let sql = q.to_sql();
	assert!(sql.contains("age >= 18"));
	assert!(sql.contains("active = true"));
	assert!(sql.contains("AND"));
}

#[test]
fn test_q_or_expression() {
	let q = Q::new("status", "=", "'active'").or(Q::new("status", "=", "'pending'"));
	let sql = q.to_sql();
	assert!(sql.contains("OR"));
}

#[test]
fn test_q_not_expression() {
	let q = Q::new("deleted", "=", "1").not();
	let sql = q.to_sql();
	assert!(sql.contains("NOT"));
}

#[test]
fn test_transaction_begin_commit() {
	let mut tx = Transaction::new();
	let begin_sql = tx.begin().unwrap();
	assert_eq!(begin_sql, "BEGIN TRANSACTION");
	assert!(tx.is_active());

	let commit_sql = tx.commit().unwrap();
	assert_eq!(commit_sql, "COMMIT");
	assert!(!tx.is_active());
}

#[test]
fn test_transaction_begin_rollback() {
	let mut tx = Transaction::new();
	tx.begin().unwrap();
	let rollback_sql = tx.rollback().unwrap();
	assert_eq!(rollback_sql, "ROLLBACK");
	assert_eq!(tx.state().unwrap(), TransactionState::RolledBack);
}

#[test]
fn test_nested_transactions() {
	let mut tx = Transaction::new();
	tx.begin().unwrap();
	assert_eq!(tx.depth(), 1);

	let nested_sql = tx.begin().unwrap();
	assert!(nested_sql.contains("SAVEPOINT"));
	assert_eq!(tx.depth(), 2);

	let commit_sql = tx.commit().unwrap();
	assert!(commit_sql.contains("RELEASE SAVEPOINT"));
	assert_eq!(tx.depth(), 1);

	tx.commit().unwrap();
	assert_eq!(tx.depth(), 0);
}

#[test]
fn test_transaction_with_isolation_level() {
	let mut tx = Transaction::new().with_isolation_level(IsolationLevel::Serializable);
	let sql = tx.begin().unwrap();
	assert!(sql.contains("ISOLATION LEVEL SERIALIZABLE"));
}

#[test]
fn test_transaction_savepoint() {
	let mut tx = Transaction::new();
	tx.begin().unwrap();

	let savepoint_sql = tx.savepoint("my_sp").unwrap();
	assert_eq!(savepoint_sql, "SAVEPOINT my_sp");

	let release_sql = tx.release_savepoint("my_sp").unwrap();
	assert_eq!(release_sql, "RELEASE SAVEPOINT my_sp");
}

#[test]
fn test_transaction_rollback_to_savepoint() {
	let mut tx = Transaction::new();
	tx.begin().unwrap();
	tx.savepoint("checkpoint").unwrap();

	let rollback_sql = tx.rollback_to_savepoint("checkpoint").unwrap();
	assert_eq!(rollback_sql, "ROLLBACK TO SAVEPOINT checkpoint");
}

// Database functions tests
#[test]
fn test_cast_function() {
	let cast = Cast::new(AnnotationValue::Field(F::new("age")), SqlType::Text);
	assert_eq!(cast.to_sql(), "CAST(age AS TEXT)");
}

#[test]
fn test_greatest_function() {
	let greatest = Greatest::new(vec![
		AnnotationValue::Field(F::new("a")),
		AnnotationValue::Field(F::new("b")),
		AnnotationValue::Field(F::new("c")),
	])
	.unwrap();
	assert_eq!(greatest.to_sql(), "GREATEST(a, b, c)");
}

#[test]
fn test_concat_function() {
	let concat = Concat::new(vec![
		AnnotationValue::Field(F::new("first_name")),
		AnnotationValue::Value(Value::String(" ".into())),
		AnnotationValue::Field(F::new("last_name")),
	])
	.unwrap();
	assert_eq!(concat.to_sql(), "CONCAT(first_name, ' ', last_name)");
}

#[test]
fn test_upper_lower_functions() {
	let upper = Upper::new(AnnotationValue::Field(F::new("name")));
	assert_eq!(upper.to_sql(), "UPPER(name)");

	let lower = Lower::new(AnnotationValue::Field(F::new("EMAIL")));
	assert_eq!(lower.to_sql(), "LOWER(EMAIL)");
}

#[test]
fn test_math_functions() {
	let abs = Abs::new(AnnotationValue::Field(F::new("balance")));
	assert_eq!(abs.to_sql(), "ABS(balance)");

	let round = Round::new(AnnotationValue::Field(F::new("price")), Some(2));
	assert_eq!(round.to_sql(), "ROUND(price, 2)");

	let sqrt = Sqrt::new(AnnotationValue::Field(F::new("area")));
	assert_eq!(sqrt.to_sql(), "SQRT(area)");
}

#[test]
fn test_annotate_with_cast() {
	let cast = Cast::new(AnnotationValue::Field(F::new("price")), SqlType::Integer);
	let cast_sql = cast.to_sql();
	assert!(cast_sql.contains("CAST"));
	assert!(cast_sql.contains("INTEGER"));
}

#[test]
fn test_annotate_with_concat() {
	let concat = Concat::new(vec![
		AnnotationValue::Field(F::new("street")),
		AnnotationValue::Value(Value::String(", ".into())),
		AnnotationValue::Field(F::new("city")),
	])
	.unwrap();
	let sql = concat.to_sql();
	assert!(sql.contains("CONCAT"));
	assert!(sql.contains("street"));
	assert!(sql.contains("city"));
}
