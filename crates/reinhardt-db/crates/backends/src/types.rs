//! Common type definitions for database abstraction

use crate::error::DatabaseError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
	Postgres,
	Sqlite,
	Mysql,
	#[cfg(feature = "mongodb-backend")]
	MongoDB,
}

/// Query value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryValue {
	Null,
	Bool(bool),
	Int(i64),
	Float(f64),
	String(String),
	Bytes(Vec<u8>),
	Timestamp(chrono::DateTime<chrono::Utc>),
}

impl From<&str> for QueryValue {
	fn from(s: &str) -> Self {
		QueryValue::String(s.to_string())
	}
}

impl From<String> for QueryValue {
	fn from(s: String) -> Self {
		QueryValue::String(s)
	}
}

impl From<i64> for QueryValue {
	fn from(i: i64) -> Self {
		QueryValue::Int(i)
	}
}

impl From<i32> for QueryValue {
	fn from(i: i32) -> Self {
		QueryValue::Int(i as i64)
	}
}

impl From<f64> for QueryValue {
	fn from(f: f64) -> Self {
		QueryValue::Float(f)
	}
}

impl From<bool> for QueryValue {
	fn from(b: bool) -> Self {
		QueryValue::Bool(b)
	}
}

impl From<chrono::DateTime<chrono::Utc>> for QueryValue {
	fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
		QueryValue::Timestamp(dt)
	}
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
	pub rows_affected: u64,
}

/// Row from query result
#[derive(Debug, Clone)]
pub struct Row {
	pub(crate) data: HashMap<String, QueryValue>,
}

impl Row {
	pub fn new() -> Self {
		Self {
			data: HashMap::new(),
		}
	}

	pub fn insert(&mut self, key: String, value: QueryValue) {
		self.data.insert(key, value);
	}

	pub fn get<T: TryFrom<QueryValue>>(&self, key: &str) -> std::result::Result<T, DatabaseError>
	where
		DatabaseError: From<<T as TryFrom<QueryValue>>::Error>,
	{
		self.data
			.get(key)
			.cloned()
			.ok_or_else(|| DatabaseError::ColumnNotFound(key.to_string()))
			.and_then(|v| v.try_into().map_err(Into::into))
	}
}

impl Default for Row {
	fn default() -> Self {
		Self::new()
	}
}

// Type conversions for QueryValue
impl TryFrom<QueryValue> for i64 {
	type Error = DatabaseError;

	fn try_from(value: QueryValue) -> std::result::Result<Self, Self::Error> {
		match value {
			QueryValue::Int(i) => Ok(i),
			_ => Err(DatabaseError::TypeError(format!(
				"Cannot convert {:?} to i64",
				value
			))),
		}
	}
}

impl TryFrom<QueryValue> for String {
	type Error = DatabaseError;

	fn try_from(value: QueryValue) -> std::result::Result<Self, Self::Error> {
		match value {
			QueryValue::String(s) => Ok(s),
			_ => Err(DatabaseError::TypeError(format!(
				"Cannot convert {:?} to String",
				value
			))),
		}
	}
}

impl TryFrom<QueryValue> for bool {
	type Error = DatabaseError;

	fn try_from(value: QueryValue) -> std::result::Result<Self, Self::Error> {
		match value {
			QueryValue::Bool(b) => Ok(b),
			_ => Err(DatabaseError::TypeError(format!(
				"Cannot convert {:?} to bool",
				value
			))),
		}
	}
}

impl TryFrom<QueryValue> for f64 {
	type Error = DatabaseError;

	fn try_from(value: QueryValue) -> std::result::Result<Self, Self::Error> {
		match value {
			QueryValue::Float(f) => Ok(f),
			_ => Err(DatabaseError::TypeError(format!(
				"Cannot convert {:?} to f64",
				value
			))),
		}
	}
}

impl TryFrom<QueryValue> for chrono::DateTime<chrono::Utc> {
	type Error = DatabaseError;

	fn try_from(value: QueryValue) -> std::result::Result<Self, Self::Error> {
		match value {
			QueryValue::Timestamp(dt) => Ok(dt),
			_ => Err(DatabaseError::TypeError(format!(
				"Cannot convert {:?} to DateTime<Utc>",
				value
			))),
		}
	}
}
