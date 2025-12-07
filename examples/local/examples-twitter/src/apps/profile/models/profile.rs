//! Profile model for user profiles
//!
//! One-to-one relationship with User model.
//! Uses reinhardt ORM (Manager/QuerySet) for database operations.

use chrono::{DateTime, Utc};
use reinhardt::db::associations::OneToOneField;
use reinhardt::model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Imports used by OneToOneField<T> type inference
#[allow(unused_imports)]
use crate::apps::auth::models::User;

/// Profile model representing a user's profile information
///
/// One-to-one relationship with User model.
/// OneToOneField<T> automatically generates the `_id` column with UNIQUE constraint.
#[model(app_label = "profile", table_name = "profile_profile")]
#[derive(Serialize, Deserialize)]
pub struct Profile {
	#[field(primary_key = true)]
	pub id: Uuid,

	/// User this profile belongs to (generates user_id column with UNIQUE)
	#[rel(one_to_one, related_name = "profile")]
	pub user: OneToOneField<User>,

	#[field(max_length = 500)]
	pub bio: String,

	#[field(max_length = 255)]
	pub avatar_url: Option<String>,

	#[field(max_length = 255)]
	pub location: Option<String>,

	#[field(max_length = 255)]
	pub website: Option<String>,

	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,

	#[field(auto_now = true)]
	pub updated_at: DateTime<Utc>,
}
