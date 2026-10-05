#![cfg(feature = "db-postgres")]

// The package is reinhardt-web, while its library target is named reinhardt.
extern crate reinhardt as reinhardt_web;

use reinhardt::db::migrations::FieldType;
use reinhardt::db::orm::{DatabaseField, DatabaseScalar};
use serde::{Deserialize, Serialize};

#[derive(reinhardt::Model, Debug, Clone, Serialize, Deserialize)]
#[model(
	app_label = "nullable_array_facade",
	table_name = "nullable_array_facade"
)]
struct NullableArrays {
	#[field(primary_key = true)]
	id: i64,
	strings: Vec<Option<String>>,
	integers: Vec<Option<i32>>,
	big_integers: Vec<Option<i64>>,
	booleans: Vec<Option<bool>>,
	floats: Vec<Option<f32>>,
	doubles: Vec<Option<f64>>,
	uuids: Vec<Option<uuid::Uuid>>,
	optional_strings: Option<Vec<Option<String>>>,
	#[field(max_length = 32)]
	bounded_strings: Vec<Option<String>>,
}

macro_rules! grouped_nullable_array_model {
	($name:ident, $element:ty) => {
		#[derive(reinhardt::Model, Debug, Clone, Serialize, Deserialize)]
		#[model(app_label = "nullable_array_facade")]
		struct $name {
			#[field(primary_key = true)]
			id: i64,
			items: Vec<Option<$element>>,
			optional_items: Option<Vec<Option<$element>>>,
		}
	};
}

grouped_nullable_array_model!(GroupedStringArrays, String);
grouped_nullable_array_model!(GroupedIntArrays, i32);
grouped_nullable_array_model!(GroupedBigIntArrays, i64);
grouped_nullable_array_model!(GroupedBoolArrays, bool);
grouped_nullable_array_model!(GroupedFloatArrays, f32);
grouped_nullable_array_model!(GroupedDoubleArrays, f64);
grouped_nullable_array_model!(GroupedUuidArrays, uuid::Uuid);

#[rstest::rstest]
#[case("GroupedStringArrays", FieldType::Text)]
#[case("GroupedIntArrays", FieldType::Integer)]
#[case("GroupedBigIntArrays", FieldType::BigInteger)]
#[case("GroupedBoolArrays", FieldType::Boolean)]
#[case("GroupedFloatArrays", FieldType::Float)]
#[case("GroupedDoubleArrays", FieldType::Double)]
#[case("GroupedUuidArrays", FieldType::Uuid)]
fn facade_postgres_derives_grouped_nullable_array_types(
	#[case] model_name: &str,
	#[case] element_type: FieldType,
) {
	// Arrange: macro_rules type arguments expand as transparent syn::Type::Group nodes.
	let models = reinhardt::db::migrations::global_registry().get_models();
	let schema = models
		.iter()
		.find(|model| model.model_name == model_name)
		.unwrap();

	// Act / Assert
	for name in ["items", "optional_items"] {
		assert_eq!(
			schema.fields[name].field_type,
			FieldType::Array(Box::new(element_type.clone())),
			"{model_name}.{name}"
		);
	}
}

#[rstest::rstest]
fn facade_postgres_derives_nullable_array_schema_and_codecs() {
	// Arrange
	let models = reinhardt::db::migrations::global_registry().get_models();
	let schema = models
		.iter()
		.find(|model| model.model_name == "NullableArrays")
		.unwrap();
	let expected = [
		("strings", FieldType::Text),
		("integers", FieldType::Integer),
		("big_integers", FieldType::BigInteger),
		("booleans", FieldType::Boolean),
		("floats", FieldType::Float),
		("doubles", FieldType::Double),
		("uuids", FieldType::Uuid),
		("optional_strings", FieldType::Text),
		("bounded_strings", FieldType::VarChar(32)),
	];

	// Act / Assert
	for (name, element) in expected {
		let field = schema.fields.get(name).unwrap();
		assert_eq!(
			field.field_type,
			FieldType::Array(Box::new(element)),
			"{name}"
		);
	}
	let values = vec![Some("kept".to_owned()), None];
	let encoded = values.encode_database().unwrap().into_database_value();
	assert_eq!(
		Vec::<Option<String>>::from_database_value(encoded).unwrap(),
		values
	);
}
