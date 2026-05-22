//! Password reset token model for authentication recovery
//!
//! Stores password reset tokens with expiration and usage tracking.
//! Uses ForeignKey relationship to User model.
#[allow(unused_imports)]
use super::user::User;
use chrono::{DateTime, Utc};
use reinhardt::core::serde::{Deserialize, Serialize};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
use uuid::Uuid;
/// Password reset token for user authentication recovery
///
/// Represents a password reset token with expiration.
/// `ForeignKeyField<User>` automatically generates the `user_id` column.
#[model(app_label = "auth", table_name = "auth_password_reset_token")]
#[derive(Serialize, Deserialize)]
pub struct PasswordResetToken {
	#[field(primary_key = true)]
	pub id: Uuid,
	/// User who requested password reset (generates user_id column)
	#[rel(foreign_key, related_name = "password_reset_tokens", on_delete = Cascade)]
	pub user: ForeignKeyField<User>,
	/// Reset token value (UUID v4 string)
	#[field(max_length = 255, unique = true)]
	pub token: String,
	/// Token expiration timestamp
	pub expires_at: DateTime<Utc>,
	/// Token creation timestamp
	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
	/// Whether this token has been used
	#[field(default = false)]
	pub is_used: bool,
}
