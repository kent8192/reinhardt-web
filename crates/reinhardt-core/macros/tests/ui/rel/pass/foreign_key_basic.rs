//! Test basic foreign_key relationship attribute with ForeignKeyField<T>

use reinhardt::db::associations::ForeignKeyField;
use reinhardt::model;
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "users")]
#[derive(Serialize, Deserialize)]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 100)]
	pub name: String,
}

#[model(app_label = "test", table_name = "posts")]
#[derive(Serialize, Deserialize)]
pub struct Post {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 200)]
	pub title: String,
	/// ForeignKeyField<User> automatically generates `author_id` column
	#[rel(foreign_key)]
	pub author: ForeignKeyField<User>,
}

fn main() {}
