//! Test basic one_to_one relationship attribute with OneToOneField<T>

use reinhardt::db::associations::OneToOneField;
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

#[model(app_label = "test", table_name = "profiles")]
#[derive(Serialize, Deserialize)]
pub struct Profile {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 500)]
	pub bio: String,
	/// OneToOneField<User> automatically generates `user_id` column with UNIQUE constraint
	#[rel(one_to_one, related_name = "profile")]
	pub user: OneToOneField<User>,
}

fn main() {}
