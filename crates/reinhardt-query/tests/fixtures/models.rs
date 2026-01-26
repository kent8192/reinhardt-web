//! Test model definitions for DML integration tests

use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[model(table_name = "users")]
#[derive(Serialize, Deserialize, Clone)]
pub struct Users {
	#[field(primary_key = true)]
	pub id: i32,

	#[field(max_length = 255)]
	pub name: String,

	#[field(max_length = 255, unique = true)]
	pub email: String,

	pub age: Option<i32>,

	#[field(default = true)]
	pub active: bool,
}

#[model(table_name = "products")]
#[derive(Serialize, Deserialize, Clone)]
pub struct Products {
	#[field(primary_key = true)]
	pub id: i32,

	#[field(max_length = 255)]
	pub name: String,

	#[field(max_length = 100)]
	pub sku: String,

	pub price: i64,

	pub stock: i32,

	#[field(default = true)]
	pub available: bool,
}

#[model(table_name = "orders")]
#[derive(Serialize, Deserialize, Clone)]
pub struct Orders {
	#[field(primary_key = true)]
	pub id: i32,

	pub user_id: i32,

	pub total_amount: i64,

	#[field(max_length = 50)]
	pub status: String,
}
