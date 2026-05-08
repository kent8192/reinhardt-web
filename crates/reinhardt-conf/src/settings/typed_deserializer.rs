//! Type-aware deserializer adapter.
//!
//! Wraps `&serde_json::Value` and, at the visitor boundary, coerces
//! `Value::String` into the target Rust type when the visitor expects a
//! non-String type. Coercion strategies are per-shape:
//!
//! | Visitor expectation | Coercion                               |
//! |---------------------|----------------------------------------|
//! | scalar (bool/int/   | `FromStr`                              |
//! |   float/char)       |                                        |
//! | `Option<T>`         | empty string -> None, else recurse     |
//! | `Vec<T>`            | parse as JSON array, recurse per item  |
//! | `Map<K, V>`         | parse as JSON object, recurse per item |
//! | `Vec<u8>` (bytes)   | base64 (STANDARD)                      |
//! | struct/tuple/non-   | `CoercionError::UnsupportedShape`      |
//! |   unit enum         |                                        |
//!
//! The wrapper never re-runs interpolation: the resolved string is
//! consumed exactly once.

use super::interpolation::KeyPath;
use serde::de::{DeserializeSeed, Deserializer, MapAccess, Visitor};
use serde::forward_to_deserialize_any;

/// Implement a `deserialize_<int>` method that coerces `Value::String`
/// into the target integer type via `FromStr`. For non-string inputs, it
/// delegates to `serde_json::Value`'s deserializer and re-wraps any
/// resulting error as `CoercionError::Parse` so callers always see a
/// uniform error shape carrying `key_path` / `target_type`.
macro_rules! impl_int_coerce {
	($method:ident, $visit:ident, $ty:ty, $type_name:expr) => {
		fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
		where
			V: Visitor<'de>,
		{
			match self.value {
				serde_json::Value::String(s) => {
					let parsed: $ty =
						s.parse()
							.map_err(|e: std::num::ParseIntError| CoercionError::Parse {
								target_type: $type_name.to_string(),
								value: s.clone(),
								key_path: self.key_path.to_string(),
								source: Box::new(e),
							})?;
					visitor.$visit(parsed)
				}
				other => other
					.clone()
					.$method(visitor)
					.map_err(|e| CoercionError::Parse {
						target_type: $type_name.to_string(),
						value: other.to_string(),
						key_path: self.key_path.to_string(),
						source: Box::new(e),
					}),
			}
		}
	};
}

/// Coercion failure surfaced from `TypedSettingsDeserializer`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoercionError {
	/// Failed to parse a string into the visitor's expected scalar /
	/// bytes / collection-element shape.
	#[error(
		"failed to coerce string `{value}` into {target_type} at \
		key `{key_path}`: {source}"
	)]
	Parse {
		/// `Visitor::expecting()` output for the destination type.
		target_type: String,
		/// Original string value that failed to coerce.
		value: String,
		/// Dot/bracket-separated TOML key path.
		key_path: String,
		/// Underlying parse error.
		#[source]
		source: Box<dyn std::error::Error + Send + Sync>,
	},

	/// The visitor expected a shape (struct, tuple, non-unit enum) that
	/// has no defined string -> T coercion.
	#[error(
		"cannot coerce string into {target} at key `{key_path}`: \
		use field-by-field interpolation instead"
	)]
	UnsupportedShape {
		/// Description of the unsupported destination shape.
		target: &'static str,
		/// Dot/bracket-separated TOML key path.
		key_path: String,
	},

	/// Pass-through for serde-internal errors.
	#[error(transparent)]
	Serde(#[from] serde::de::value::Error),
}

impl serde::de::Error for CoercionError {
	fn custom<T: std::fmt::Display>(msg: T) -> Self {
		CoercionError::Serde(serde::de::Error::custom(msg))
	}
}

/// Type-aware adapter around `&serde_json::Value`.
///
/// Phase 2 skeleton: forwards every `deserialize_*` to `serde_json`'s
/// own deserializer. Subsequent phases override individual visitor
/// callbacks to coerce `Value::String` into the target Rust type.
pub struct TypedSettingsDeserializer<'de> {
	value: &'de serde_json::Value,
	key_path: KeyPath,
}

impl<'de> TypedSettingsDeserializer<'de> {
	/// Wrap a borrowed `serde_json::Value`. The returned deserializer
	/// coerces string values when the visitor expects a non-String
	/// target.
	pub fn new(value: &'de serde_json::Value) -> Self {
		Self {
			value,
			key_path: KeyPath::default(),
		}
	}
}

impl<'de> Deserializer<'de> for TypedSettingsDeserializer<'de> {
	type Error = CoercionError;

	// Phase 3a (#4226): bool + integers are explicitly overridden below
	// to coerce `Value::String` via `FromStr`. `map`/`struct` are also
	// overridden so that per-field dispatch flows through this
	// deserializer (otherwise the scalar overrides would be unreachable
	// when the top-level value is an object). Other shapes still forward
	// to `deserialize_any` until later phases.
	forward_to_deserialize_any! {
		f32 f64 char str string bytes byte_buf option unit unit_struct
		newtype_struct seq tuple tuple_struct enum identifier ignored_any
	}

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		// Delegate to serde_json's deserializer; map its error into ours.
		self.value
			.clone()
			.deserialize_any(visitor)
			.map_err(|e| CoercionError::Parse {
				target_type: "any".to_string(),
				value: self.value.to_string(),
				key_path: self.key_path.to_string(),
				source: Box::new(e),
			})
	}

	fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			serde_json::Value::String(s) => {
				let parsed: bool =
					s.parse()
						.map_err(|e: std::str::ParseBoolError| CoercionError::Parse {
							target_type: "bool".to_string(),
							value: s.clone(),
							key_path: self.key_path.to_string(),
							source: Box::new(e),
						})?;
				visitor.visit_bool(parsed)
			}
			other => other
				.clone()
				.deserialize_bool(visitor)
				.map_err(|e| CoercionError::Parse {
					target_type: "bool".to_string(),
					value: other.to_string(),
					key_path: self.key_path.to_string(),
					source: Box::new(e),
				}),
		}
	}

	impl_int_coerce!(deserialize_i8, visit_i8, i8, "i8");
	impl_int_coerce!(deserialize_i16, visit_i16, i16, "i16");
	impl_int_coerce!(deserialize_i32, visit_i32, i32, "i32");
	impl_int_coerce!(deserialize_i64, visit_i64, i64, "i64");
	impl_int_coerce!(deserialize_i128, visit_i128, i128, "i128");
	impl_int_coerce!(deserialize_u8, visit_u8, u8, "u8");
	impl_int_coerce!(deserialize_u16, visit_u16, u16, "u16");
	impl_int_coerce!(deserialize_u32, visit_u32, u32, "u32");
	impl_int_coerce!(deserialize_u64, visit_u64, u64, "u64");
	impl_int_coerce!(deserialize_u128, visit_u128, u128, "u128");

	fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.value {
			serde_json::Value::Object(map) => visitor.visit_map(TypedMapAccess {
				iter: map.iter(),
				pending_key: None,
				pending_value: None,
				key_path: self.key_path,
			}),
			other => other
				.clone()
				.deserialize_map(visitor)
				.map_err(|e| CoercionError::Parse {
					target_type: "map".to_string(),
					value: other.to_string(),
					key_path: self.key_path.to_string(),
					source: Box::new(e),
				}),
		}
	}

	fn deserialize_struct<V>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		// Structs deserialize via `visit_map`; route through the same
		// adapter as `deserialize_map`.
		self.deserialize_map(visitor)
	}
}

/// `MapAccess` adapter that re-wraps each value in a
/// `TypedSettingsDeserializer` so per-field type overrides apply, and
/// pushes the current key onto `KeyPath` so error messages identify the
/// failing field.
struct TypedMapAccess<'de> {
	iter: serde_json::map::Iter<'de>,
	pending_key: Option<&'de str>,
	pending_value: Option<&'de serde_json::Value>,
	key_path: KeyPath,
}

impl<'de> MapAccess<'de> for TypedMapAccess<'de> {
	type Error = CoercionError;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: DeserializeSeed<'de>,
	{
		match self.iter.next() {
			Some((k, v)) => {
				self.pending_key = Some(k.as_str());
				self.pending_value = Some(v);
				// Keys are always strings in JSON objects; deserialize
				// directly via serde_json's borrowed-str deserializer.
				let key = seed
					.deserialize(serde::de::value::BorrowedStrDeserializer::<
						'de,
						serde::de::value::Error,
					>::new(k.as_str()))
					.map_err(|e| CoercionError::Parse {
						target_type: "map_key".to_string(),
						value: k.clone(),
						key_path: self.key_path.to_string(),
						source: Box::new(e),
					})?;
				Ok(Some(key))
			}
			None => Ok(None),
		}
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
	where
		V: DeserializeSeed<'de>,
	{
		let value = self
			.pending_value
			.take()
			.ok_or_else(|| CoercionError::Parse {
				target_type: "map_value".to_string(),
				value: String::new(),
				key_path: self.key_path.to_string(),
				source: "next_value_seed called before next_key_seed".into(),
			})?;
		let key = self.pending_key.take();

		// Build a child `KeyPath` that includes the current field name
		// so coercion errors identify the failing key.
		let mut child_path = self.key_path.clone();
		if let Some(k) = key {
			child_path.push_key(k);
		}
		let de = TypedSettingsDeserializer {
			value,
			key_path: child_path,
		};
		seed.deserialize(de)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serde::Deserialize;

	#[rstest]
	fn skeleton_passthrough_for_native_int() {
		// Arrange — native JSON int, no coerce needed.
		let v = serde_json::json!({ "port": 5432 });

		#[derive(Debug, Deserialize, PartialEq)]
		struct S {
			port: u16,
		}

		// Act
		let de = TypedSettingsDeserializer::new(&v);
		let got: S = S::deserialize(de).expect("should deserialize");

		// Assert
		assert_eq!(got, S { port: 5432 });
	}
}
