#![allow(dead_code, unexpected_cfgs)]

use chrono::{DateTime, Utc};
use reinhardt::db::orm::Model;
use reinhardt::model;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[model(table_name = "users")]
struct User {
	#[field(primary_key = true)]
	id: i64,
	#[field(max_length = 255)]
	username: String,
	#[field(max_length = 255)]
	email: String,
	age: i32,
	created_at: DateTime<Utc>,
}

async fn documented_queryset_chain() -> reinhardt::Result<Vec<User>> {
	let users = User::objects()
		.filter(User::field_age().gte(18))
		.filter(User::field_email().icontains("example.com"))
		.filter(User::field_id().is_in([1_i64, 2, 3]))
		.filter(User::field_created_at().year().gte(2026))
		.order_by(&["-created_at"])
		.limit(10)
		.all()
		.await?;

	Ok(users)
}

fn documented_composite_filter_chain() -> String {
	User::objects()
		.filter(
			User::field_username()
				.iexact("admin")
				.or(User::field_email().icontains("example.com").not()),
		)
		.to_sql()
}

fn main() {}
