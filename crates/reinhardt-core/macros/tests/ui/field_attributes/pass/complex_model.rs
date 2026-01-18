use reinhardt_macros::model;
use serde::{Deserialize, Serialize};

#[model(app_label = "test", table_name = "articles")]
#[derive(Serialize, Deserialize)]
struct Article {
	#[cfg(feature = "db-postgres")]
	#[field(primary_key = true, identity_by_default = true)]
	id: Option<i64>,

	#[cfg(not(feature = "db-postgres"))]
	#[field(primary_key = true)]
	id: Option<i64>,

	#[field(max_length = 255, collate = "utf8mb4_unicode_ci")]
	title: String,

	#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
	#[field(max_length = 5000, fulltext = true)]
	content: String,

	#[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
	#[field(max_length = 5000)]
	content: String,

	#[field(max_length = 255, generated = "UPPER(title)", generated_stored = true)]
	title_upper: String,

	#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
	#[field(comment = "Article creation timestamp")]
	created_at: i64,

	#[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
	created_at: i64,

	#[cfg(feature = "db-mysql")]
	#[field(on_update_current_timestamp = true)]
	updated_at: i64,

	#[cfg(not(feature = "db-mysql"))]
	updated_at: i64,
}

fn main() {}
