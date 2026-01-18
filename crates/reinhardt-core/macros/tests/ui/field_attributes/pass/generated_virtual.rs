#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
use reinhardt_macros::model;
#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
#[model(app_label = "test", table_name = "products")]
#[derive(Serialize, Deserialize)]
struct Product {
	#[field(primary_key = true)]
	id: Option<i32>,

	price: i32,

	#[field(generated = "price * 1.1", generated_virtual = true)]
	price_with_tax: i32,
}

fn main() {}
