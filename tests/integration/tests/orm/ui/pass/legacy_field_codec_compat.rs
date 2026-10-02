#![allow(unexpected_cfgs)] // Generated model cfgs are defined by the consuming crate.
//! Pass case: existing built-in model fields and nullable filters remain source-compatible.

use reinhardt::model;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[model(app_label = "compat", table_name = "legacy_field_models")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LegacyFieldModel {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 32, null = true)]
	optional_name: Option<String>,
	optional_score: Option<i64>,
	price: Decimal,
	#[field(max_length = 32)]
	tags: Vec<String>,
	attributes: HashMap<String, String>,
}

fn main() {
	let _ = LegacyFieldModel::field_optional_name().eq("alice");
	let _ = LegacyFieldModel::field_optional_score().eq(42_i64);
	let _ = LegacyFieldModel::field_optional_score().is_in([1_i64, 2_i64]);

	let model = LegacyFieldModel {
		id: None,
		optional_name: None,
		optional_score: None,
		price: Decimal::new(12345, 2),
		tags: vec!["rust".to_owned()],
		attributes: HashMap::from([("framework".to_owned(), "reinhardt".to_owned())]),
	};
	let _ = reinhardt::db::orm::Model::encode_database_fields(&model);
}
