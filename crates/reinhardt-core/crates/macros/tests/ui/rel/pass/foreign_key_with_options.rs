//! Test foreign_key with all options

use reinhardt::model;

#[model(app_label = "test")]
pub struct Author {
	#[field(primary_key = true)]
	pub id: i64,
	pub name: String,
}

#[model(app_label = "test")]
pub struct Book {
	#[field(primary_key = true)]
	pub id: i64,
	pub title: String,
	#[rel(
		foreign_key,
		to = Author,
		to_field = "id",
		related_name = "books",
		on_delete = Cascade,
		on_update = NoAction,
		null = false,
		db_index = true
	)]
	pub author_id: i64,
}

fn main() {}
