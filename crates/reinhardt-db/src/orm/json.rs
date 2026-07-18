//! Typed JSON field wrapper for model fields.

use super::model::Model;
use serde::de::value::{MapDeserializer, StringDeserializer};
use serde::de::{IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::collections::HashSet;
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

pub(crate) fn is_json_field_type(field_type: &str) -> bool {
	field_type.contains("JsonField")
}

pub(crate) fn deserialize_model_row<M: Model>(
	data: serde_json::Value,
	mut json_null_fields: HashSet<String>,
	mut native_json_fields: HashSet<String>,
) -> Result<M, serde_json::Error> {
	let serde_json::Value::Object(mut fields) = data else {
		return serde_json::from_value(data);
	};

	for field in M::field_metadata().into_iter().filter(|field| {
		!is_json_field_type(&field.field_type) && field.db_column_name() != field.name
	}) {
		let column_name = field.db_column_name().to_string();
		if fields.contains_key(&field.name) {
			fields.remove(&column_name);
		} else if let Some(value) = fields.remove(&column_name) {
			fields.insert(field.name, value);
		}
	}

	for field in M::field_metadata()
		.into_iter()
		.filter(|field| is_json_field_type(&field.field_type))
	{
		let column_name = field.db_column_name().to_string();
		let source_name = if fields.contains_key(&field.name) {
			field.name.clone()
		} else {
			column_name
		};
		let Some(stored_value) = fields.remove(&source_name) else {
			continue;
		};

		let was_native_json_null = json_null_fields.remove(&source_name);
		let was_native_json = native_json_fields.remove(&source_name);
		let was_json_text = stored_value.is_string() && !was_native_json;
		let parsed_value = match stored_value {
			serde_json::Value::String(text) if !was_native_json => serde_json::from_str(&text)
				.map_err(|error| {
					<serde_json::Error as serde::de::Error>::custom(format!(
						"Failed to hydrate JSON field {}.{} from column '{}': {}",
						M::table_name(),
						field.name,
						field.db_column_name(),
						error
					))
				})?,
			value => value,
		};

		if field.nullable && parsed_value.is_null() && (was_native_json_null || was_json_text) {
			json_null_fields.insert(field.name.clone());
		}

		fields.insert(field.name.clone(), parsed_value);
	}

	deserialize_model_value(serde_json::Value::Object(fields), &json_null_fields)
}

pub(crate) fn deserialize_model_value<M: Model>(
	data: serde_json::Value,
	json_null_fields: &HashSet<String>,
) -> Result<M, serde_json::Error> {
	if json_null_fields.is_empty() {
		return serde_json::from_value(data);
	}

	let serde_json::Value::Object(fields) = data else {
		return serde_json::from_value(data);
	};
	let entries = fields.into_iter().map(|(name, value)| {
		let value = if json_null_fields.contains(&name) {
			ModelFieldValue::PresentJsonNull
		} else {
			ModelFieldValue::Value(value)
		};
		(StringDeserializer::<serde_json::Error>::new(name), value)
	});
	M::deserialize(MapDeserializer::<_, serde_json::Error>::new(entries))
}

enum ModelFieldValue {
	Value(serde_json::Value),
	PresentJsonNull,
}

impl<'de> IntoDeserializer<'de, serde_json::Error> for ModelFieldValue {
	type Deserializer = Self;

	fn into_deserializer(self) -> Self::Deserializer {
		self
	}
}

impl<'de> Deserializer<'de> for ModelFieldValue {
	type Error = serde_json::Error;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self {
			Self::Value(value) => value.into_deserializer().deserialize_any(visitor),
			Self::PresentJsonNull => serde_json::Value::Null
				.into_deserializer()
				.deserialize_any(visitor),
		}
	}

	fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self {
			Self::Value(value) => value.into_deserializer().deserialize_option(visitor),
			Self::PresentJsonNull => {
				visitor.visit_some(serde_json::Value::Null.into_deserializer())
			}
		}
	}

	serde::forward_to_deserialize_any! {
		bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
		unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
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
