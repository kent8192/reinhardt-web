use reinhardt_macros::model;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[model(app_label = "test", table_name = "articles")]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
	#[field(fulltext = true)]
	content: String,

	#[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
	content: String,

	#[field(generated = "UPPER(title)", generated_stored = true)]
	title_upper: String,

	#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
	#[field(comment = "Article creation timestamp")]
	created_at: NaiveDateTime,

	#[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
	created_at: NaiveDateTime,

	#[cfg(feature = "db-mysql")]
	#[field(on_update_current_timestamp = true)]
	updated_at: NaiveDateTime,

	#[cfg(not(feature = "db-mysql"))]
	updated_at: NaiveDateTime,
}

fn main() {}
