//! Social account storage

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Social account linking user to provider
pub struct SocialAccount {
	pub id: Uuid,
	pub user_id: Uuid,
	pub provider: String,
	pub provider_user_id: String,
	pub email: Option<String>,
	pub display_name: Option<String>,
	pub picture: Option<String>,
	pub access_token: String,
	pub refresh_token: Option<String>,
	pub token_expires_at: DateTime<Utc>,
	pub scopes: Vec<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Social account storage trait
pub trait SocialAccountStorage {
	// Implementation pending
}
