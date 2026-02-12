// Copyright 2024-2025 the reinhardt-db authors
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at
//
// Unless required by applicable law or agreed to in writing, software distributed under
// the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. See the License for the specific language governing
// permissions and limitations under the License.

//! Database session management for reinhardt
//!
//! This module provides database session management with query execution,
//! connection pooling, and transaction support.

use reinhardt_query::value::Value;
use uuid::Uuid;

/// Convert JSON value to reinhardt_query Value
fn json_to_reinhardt_query_value(value: &Value) -> reinhardt_query::value::Value {
	match value {
		Value::Null => reinhardt_query::value::Value::Int(None),
		Value::Bool(b) => reinhardt_query::value::Value::Bool(Some(*b)),
		Value::Number(n) => {
			if let Some(i) = n.as_i64() {
				reinhardt_query::value::Value::BigInt(Some(i))
			} else if let Some(f) = n.as_f64() {
				reinhardt_query::value::Value::Double(Some(f))
			} else {
				reinhardt_query::value::Value::Int(None)
			}
		}
		Value::String(s) => {
			// Try to parse as UUID first
			if let Ok(uuid) = Uuid::parse_str(s) {
				return reinhardt_query::value::Value::Uuid(Some(Box::new(uuid)));
			}
			reinhardt_query::value::Value::String(Some(Box::new(s.clone())))
		}
		Value::Array(_) | Value::Object(_) => {
			// For complex types, serialize as JSON string
			reinhardt_query::value::Value::String(Some(Box::new(value.to_string())))
		}
	}
}

/// Bind reinhardt_query Value to sqlx Query
fn bind_reinhardt_query_value<'a>(
	query: sqlx::query::Query<'a, sqlx::Any, sqlx::any::AnyArguments<'a>>,
	value: &reinhardt_query::value::Value,
) -> sqlx::query::Query<'a, sqlx::Any, sqlx::any::AnyArguments<'a>> {
	match value {
		reinhardt_query::value::Value::Bool(Some(b)) => query.bind(*b),
		reinhardt_query::value::Value::TinyInt(Some(i)) => query.bind(*i as i32),
		reinhardt_query::value::Value::SmallInt(Some(i)) => query.bind(*i as i32),
		reinhardt_query::value::Value::Int(Some(i)) => query.bind(*i),
		reinhardt_query::value::Value::BigInt(Some(i)) => query.bind(*i),
		reinhardt_query::value::Value::TinyUnsigned(Some(i)) => query.bind(*i as u32),
		reinhardt_query::value::Value::SmallUnsigned(Some(i)) => query.bind(*i as u32),
		reinhardt_query::value::Value::Unsigned(Some(i)) => query.bind(*i as u64),
		reinhardt_query::value::Value::BigUnsigned(Some(i)) => query.bind(*i as u64),
		reinhardt_query::value::Value::Float(Some(f)) => query.bind(*f),
		reinhardt_query::value::Value::Double(Some(f)) => query.bind(*f),
		reinhardt_query::value::Value::String(Some(s)) => query.bind(s.as_ref().clone()),
		reinhardt_query::value::Value::Bytea(Some(b)) => query.bind(b.as_ref().clone()),
		reinhardt_query::value::Value::Json(Some(j)) => query.bind(j.as_ref().clone()),
		_ => query.bind(value),
	}
}
