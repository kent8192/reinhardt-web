//! Serialize MySQL bulk rows while retaining top-level optional NULL values.

use serde::ser::{Error, Impossible, SerializeMap, SerializeStruct};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn serialize<T: ?Sized + Serialize>(
	model: &T,
) -> Result<(Value, HashSet<String>), serde_json::Error> {
	let mut sql_null_fields = HashSet::new();
	let value = model.serialize(ModelSerializer {
		sql_null_fields: &mut sql_null_fields,
	})?;
	Ok((value, sql_null_fields))
}

struct ModelSerializer<'a> {
	sql_null_fields: &'a mut HashSet<String>,
}

// All non-object shapes use serde_json's normal serializer. Only top-level
// model fields need the distinction between Option::None and JSON null.
macro_rules! forward_json {
	($($method:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
		$(fn $method(self, $($arg: $ty),*) -> Result<$result, Self::Error> {
			Serializer::$method(serde_json::value::Serializer, $($arg),*)
		})*
	};
}

impl<'a> Serializer for ModelSerializer<'a> {
	type Ok = Value;
	type Error = serde_json::Error;
	type SerializeSeq = <serde_json::value::Serializer as Serializer>::SerializeSeq;
	type SerializeTuple = <serde_json::value::Serializer as Serializer>::SerializeTuple;
	type SerializeTupleStruct = <serde_json::value::Serializer as Serializer>::SerializeTupleStruct;
	type SerializeTupleVariant =
		<serde_json::value::Serializer as Serializer>::SerializeTupleVariant;
	type SerializeMap = FieldsSerializer<'a>;
	type SerializeStruct = FieldsSerializer<'a>;
	type SerializeStructVariant =
		<serde_json::value::Serializer as Serializer>::SerializeStructVariant;

	forward_json! {
		serialize_bool(value: bool) -> Value;
		serialize_i8(value: i8) -> Value;
		serialize_i16(value: i16) -> Value;
		serialize_i32(value: i32) -> Value;
		serialize_i64(value: i64) -> Value;
		serialize_i128(value: i128) -> Value;
		serialize_u8(value: u8) -> Value;
		serialize_u16(value: u16) -> Value;
		serialize_u32(value: u32) -> Value;
		serialize_u64(value: u64) -> Value;
		serialize_u128(value: u128) -> Value;
		serialize_f32(value: f32) -> Value;
		serialize_f64(value: f64) -> Value;
		serialize_char(value: char) -> Value;
		serialize_str(value: &str) -> Value;
		serialize_bytes(value: &[u8]) -> Value;
		serialize_none() -> Value;
		serialize_unit() -> Value;
		serialize_unit_struct(name: &'static str) -> Value;
		serialize_unit_variant(name: &'static str, index: u32, variant: &'static str) -> Value;
		serialize_seq(len: Option<usize>) -> Self::SerializeSeq;
		serialize_tuple(len: usize) -> Self::SerializeTuple;
		serialize_tuple_struct(name: &'static str, len: usize) -> Self::SerializeTupleStruct;
		serialize_tuple_variant(name: &'static str, index: u32, variant: &'static str, len: usize) -> Self::SerializeTupleVariant;
		serialize_struct_variant(name: &'static str, index: u32, variant: &'static str, len: usize) -> Self::SerializeStructVariant;
	}

	fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value, Self::Error> {
		value.serialize(self)
	}

	fn serialize_newtype_struct<T: ?Sized + Serialize>(
		self,
		_name: &'static str,
		value: &T,
	) -> Result<Value, Self::Error> {
		value.serialize(self)
	}

	fn serialize_newtype_variant<T: ?Sized + Serialize>(
		self,
		name: &'static str,
		index: u32,
		variant: &'static str,
		value: &T,
	) -> Result<Value, Self::Error> {
		serde_json::value::Serializer.serialize_newtype_variant(name, index, variant, value)
	}

	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		Ok(FieldsSerializer {
			values: serde_json::Map::new(),
			next_key: None,
			sql_null_fields: self.sql_null_fields,
		})
	}

	fn serialize_struct(
		self,
		_name: &'static str,
		len: usize,
	) -> Result<Self::SerializeStruct, Self::Error> {
		self.serialize_map(Some(len))
	}

	fn collect_str<T: ?Sized + std::fmt::Display>(self, value: &T) -> Result<Value, Self::Error> {
		serde_json::value::Serializer.collect_str(value)
	}
}

struct FieldsSerializer<'a> {
	values: serde_json::Map<String, Value>,
	next_key: Option<String>,
	sql_null_fields: &'a mut HashSet<String>,
}

impl FieldsSerializer<'_> {
	fn insert<T: ?Sized + Serialize>(
		&mut self,
		key: &str,
		value: &T,
	) -> Result<(), serde_json::Error> {
		let json = serde_json::to_value(value)?;
		// Only null values need a second, shallow probe. Nested JSON nulls are
		// serialized normally, and no sentinel can collide with a user document.
		if json.is_null() && value.serialize(IsNoneSerializer)? {
			self.sql_null_fields.insert(key.to_owned());
		} else {
			self.sql_null_fields.remove(key);
		}
		self.values.insert(key.to_owned(), json);
		Ok(())
	}
}

impl SerializeStruct for FieldsSerializer<'_> {
	type Ok = Value;
	type Error = serde_json::Error;

	fn serialize_field<T: ?Sized + Serialize>(
		&mut self,
		key: &'static str,
		value: &T,
	) -> Result<(), Self::Error> {
		self.insert(key, value)
	}

	fn end(self) -> Result<Value, Self::Error> {
		Ok(Value::Object(self.values))
	}
}

impl SerializeMap for FieldsSerializer<'_> {
	type Ok = Value;
	type Error = serde_json::Error;

	fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
		// Delegate key normalization, including renamed/flattened model fields,
		// to serde_json instead of duplicating its accepted map key types.
		let mut key_map = serde_json::value::Serializer.serialize_map(Some(1))?;
		key_map.serialize_entry(key, &())?;
		let Value::Object(keys) = SerializeMap::end(key_map)? else {
			return Err(Error::custom(
				"Model field names must serialize as map keys",
			));
		};
		self.next_key = keys.into_iter().next().map(|(key, _)| key);
		Ok(())
	}

	fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
		let key = self
			.next_key
			.take()
			.ok_or_else(|| Error::custom("Model field value is missing its key"))?;
		self.insert(&key, value)
	}

	fn end(self) -> Result<Value, Self::Error> {
		Ok(Value::Object(self.values))
	}
}

// A serde_json null can originate from None, Some(JSON null), a unit value,
// or a non-finite float. Probe only this shape and retain the outer Option.
struct IsNoneSerializer;

impl Serializer for IsNoneSerializer {
	type Ok = bool;
	type Error = serde_json::Error;
	type SerializeSeq = Impossible<bool, Self::Error>;
	type SerializeTuple = Impossible<bool, Self::Error>;
	type SerializeTupleStruct = Impossible<bool, Self::Error>;
	type SerializeTupleVariant = Impossible<bool, Self::Error>;
	type SerializeMap = Impossible<bool, Self::Error>;
	type SerializeStruct = Impossible<bool, Self::Error>;
	type SerializeStructVariant = Impossible<bool, Self::Error>;

	fn serialize_bool(self, _value: bool) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_i8(self, _value: i8) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_i16(self, _value: i16) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_i32(self, _value: i32) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_i64(self, _value: i64) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_i128(self, _value: i128) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_u8(self, _value: u8) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_u16(self, _value: u16) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_u32(self, _value: u32) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_u64(self, _value: u64) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_u128(self, _value: u128) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_f32(self, _value: f32) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_f64(self, _value: f64) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_char(self, _value: char) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_str(self, _value: &str) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_bytes(self, _value: &[u8]) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_unit_variant(
		self,
		_name: &'static str,
		_index: u32,
		_variant: &'static str,
	) -> Result<bool, serde_json::Error> {
		Ok(false)
	}

	fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, serde_json::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, serde_json::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_tuple_struct(
		self,
		_name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeTupleStruct, serde_json::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_tuple_variant(
		self,
		_name: &'static str,
		_index: u32,
		_variant: &'static str,
		_len: usize,
	) -> Result<Self::SerializeTupleVariant, serde_json::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_struct_variant(
		self,
		_name: &'static str,
		_index: u32,
		_variant: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStructVariant, serde_json::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_none(self) -> Result<bool, Self::Error> {
		Ok(true)
	}

	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_struct(
		self,
		_name: &'static str,
		_len: usize,
	) -> Result<Self::SerializeStruct, Self::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn serialize_unit(self) -> Result<bool, Self::Error> {
		Ok(false)
	}

	fn serialize_unit_struct(self, _name: &'static str) -> Result<bool, Self::Error> {
		Ok(false)
	}

	fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<bool, Self::Error> {
		Ok(false)
	}

	fn serialize_newtype_struct<T: ?Sized + Serialize>(
		self,
		_name: &'static str,
		value: &T,
	) -> Result<bool, Self::Error> {
		value.serialize(self)
	}

	fn serialize_newtype_variant<T: ?Sized + Serialize>(
		self,
		_name: &'static str,
		_index: u32,
		_variant: &'static str,
		_value: &T,
	) -> Result<bool, Self::Error> {
		Err(Error::custom("Null value changed its serialization shape"))
	}

	fn collect_str<T: ?Sized + std::fmt::Display>(self, _value: &T) -> Result<bool, Self::Error> {
		Ok(false)
	}
}

#[cfg(test)]
mod tests {
	use super::serialize;
	use rstest::rstest;
	use serde::Serialize;
	use serde_json::{Value, json};
	use std::collections::{BTreeMap, HashSet};

	#[derive(Serialize)]
	struct Record {
		required: Value,
		optional: Option<Value>,
		nested: Value,
	}

	#[rstest]
	#[case(None, true)]
	#[case(Some(Value::Null), false)]
	#[case(Some(json!("null")), false)]
	fn preserves_outer_option(#[case] optional: Option<Value>, #[case] is_sql_null: bool) {
		// Arrange
		let model = Record {
			required: Value::Null,
			optional,
			nested: json!({"array": [null], "__reinhardt_sql_null": true}),
		};
		// Act
		let (value, sql_null_fields) = serialize(&model).unwrap();
		// Assert
		assert_eq!(value, serde_json::to_value(&model).unwrap());
		assert_eq!(
			sql_null_fields,
			if is_sql_null {
				HashSet::from(["optional".to_owned()])
			} else {
				HashSet::new()
			}
		);
	}

	#[derive(Serialize)]
	struct Renamed {
		#[serde(rename = "stored_json")]
		value: Option<Value>,
	}

	#[derive(Serialize)]
	struct Flattened {
		#[serde(flatten)]
		inner: Renamed,
		#[serde(skip_serializing_if = "Option::is_none")]
		skipped: Option<Value>,
	}

	#[rstest]
	#[case(None, true)]
	#[case(Some(Value::Null), false)]
	fn preserves_flattened_and_renamed_options(
		#[case] value: Option<Value>,
		#[case] is_sql_null: bool,
	) {
		// Arrange
		let model = Flattened {
			inner: Renamed { value },
			skipped: None,
		};
		// Act
		let (value, sql_null_fields) = serialize(&model).unwrap();
		// Assert
		assert_eq!(value, json!({"stored_json": null}));
		assert_eq!(sql_null_fields.contains("stored_json"), is_sql_null);
		assert!(!sql_null_fields.contains("skipped"));
		assert_eq!(sql_null_fields.len(), usize::from(is_sql_null));
	}

	#[rstest]
	fn preserves_map_option_values() {
		// Arrange
		let model = BTreeMap::from([("absent", None), ("json_null", Some(Value::Null))]);
		// Act
		let (value, sql_null_fields) = serialize(&model).unwrap();
		// Assert
		assert_eq!(value, json!({"absent": null, "json_null": null}));
		assert_eq!(sql_null_fields, HashSet::from(["absent".to_owned()]));
	}
}
