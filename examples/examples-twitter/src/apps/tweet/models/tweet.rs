//! Tweet model
#[allow(unused_imports)]
use crate::apps::auth::models::User;
use chrono::{DateTime, Utc};
use reinhardt::core::serde::{Deserialize, Serialize};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::model;
#[cfg(all(test, native))]
use sqlx::FromRow;
use uuid::Uuid;
#[model(app_label = "tweet", table_name = "tweet_tweet")]
#[derive(Serialize, Deserialize)]
#[cfg_attr(all(test, native), derive(FromRow))]
pub struct Tweet {
	#[field(primary_key = true)]
	pub id: Uuid,
	#[cfg_attr(all(test, native), sqlx(skip))]
	#[rel(foreign_key, related_name = "tweets")]
	pub user: ForeignKeyField<User>,
	#[field(max_length = 280)]
	pub content: String,
	#[field(default = 0)]
	pub like_count: i32,
	#[field(default = 0)]
	pub retweet_count: i32,
	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
	#[field(auto_now = true)]
	pub updated_at: DateTime<Utc>,
}
