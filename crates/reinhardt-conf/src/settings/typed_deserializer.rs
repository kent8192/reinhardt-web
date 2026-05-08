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
use serde::de::{Deserializer, Visitor};
use serde::forward_to_deserialize_any;

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

	// Skeleton: forward everything to serde_json's deserializer for
	// now. Phase 3+ overrides each `deserialize_*` we care about.
	forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
		string bytes byte_buf option unit unit_struct newtype_struct seq
		tuple tuple_struct map struct enum identifier ignored_any
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
