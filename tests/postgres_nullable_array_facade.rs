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
