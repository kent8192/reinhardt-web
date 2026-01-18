//! Test error for unknown rel attribute

use reinhardt::db::associations::ForeignKeyField;
use reinhardt::model;
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "users")]
#[derive(Serialize, Deserialize)]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,
}

#[model(app_label = "test", table_name = "posts")]
#[derive(Serialize, Deserialize)]
pub struct Post {
	#[field(primary_key = true)]
	pub id: i64,
	#[rel(foreign_key, unknown_option = true)]
	pub author: ForeignKeyField<User>,
}

fn main() {}
