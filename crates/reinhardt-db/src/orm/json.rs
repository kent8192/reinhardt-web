//! Typed JSON field wrapper for model fields.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::ops::{Deref, DerefMut};

/// Newtype wrapper for JSON-backed model fields.
///
/// `Json<T>` serializes and deserializes exactly like `T`, while making the
/// model field type explicit enough for `#[model]` to emit JSON column
/// metadata.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
	/// Creates a typed JSON wrapper.
	pub const fn new(value: T) -> Self {
		Self(value)
	}

	/// Returns the wrapped value.
	pub fn into_inner(self) -> T {
		self.0
	}

	/// Borrows the wrapped value.
	pub const fn as_inner(&self) -> &T {
		&self.0
	}

	/// Mutably borrows the wrapped value.
	pub fn as_inner_mut(&mut self) -> &mut T {
		&mut self.0
	}

	/// Converts the wrapped value into a JSON value.
	pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error>
	where
		T: Serialize,
	{
		serde_json::to_value(&self.0)
	}

	/// Builds a typed JSON wrapper from a JSON value.
	pub fn from_json_value(value: serde_json::Value) -> Result<Self, serde_json::Error>
	where
		T: DeserializeOwned,
	{
		serde_json::from_value(value).map(Self)
	}
}

impl<T> Deref for Json<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for Json<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T> From<T> for Json<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T> Serialize for Json<T>
where
	T: Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de, T> Deserialize<'de> for Json<T>
where
	T: Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		T::deserialize(deserializer).map(Self)
	}
}

#[cfg(test)]
mod tests {
	use super::Json;
	use serde::{Deserialize, Serialize};
	use serde_json::json;

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct StyleSettings {
		indent_width: u8,
		theme: String,
	}

	#[test]
	fn json_wrapper_serializes_as_inner_value() {
		let settings = Json::new(StyleSettings {
			indent_width: 2,
			theme: "paper".to_string(),
		});

		let value = serde_json::to_value(&settings).unwrap();

		assert_eq!(
			value,
			json!({
				"indent_width": 2,
				"theme": "paper"
			})
		);
	}

	#[test]
	fn json_wrapper_deserializes_as_inner_value() {
		let settings: Json<StyleSettings> = serde_json::from_value(json!({
			"indent_width": 4,
			"theme": "ink"
		}))
		.unwrap();

		assert_eq!(settings.indent_width, 4);
		assert_eq!(settings.theme, "ink");
	}

	#[test]
	fn json_value_is_supported_as_inner_type() {
		let metadata = Json::new(json!({ "language": "ja", "tags": ["draft"] }));

		assert_eq!(metadata["language"], "ja");
		assert_eq!(metadata.to_json_value().unwrap()["tags"][0], "draft");
	}
}
