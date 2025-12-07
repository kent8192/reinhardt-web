//! Test foreign_key with all options using ForeignKeyField<T>

use reinhardt::db::associations::ForeignKeyField;
use reinhardt::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "authors")]
pub struct Author {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 100)]
	pub name: String,
}

#[derive(Serialize, Deserialize)]
#[model(app_label = "test", table_name = "books")]
pub struct Book {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 200)]
	pub title: String,
	/// ForeignKeyField<Author> with custom options
	/// - db_column: Custom column name "writer_id" instead of "author_id"
	/// - related_name: Reverse accessor name "books"
	/// - on_delete/on_update: Cascade actions
	#[rel(
		foreign_key,
		db_column = "writer_id",
		to_field = "id",
		related_name = "books",
		on_delete = Cascade,
		on_update = NoAction,
		null = false,
		db_index = true
	)]
	pub author: ForeignKeyField<Author>,
}

fn main() {}
