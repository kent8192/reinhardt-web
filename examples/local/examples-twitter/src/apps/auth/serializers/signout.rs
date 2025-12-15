//! Signout serializers
//!
//! Serializers for user signout endpoints

use reinhardt::rest::{Schema, ToSchema};
use serde::{Deserialize, Serialize};

/// Response data for successful signout
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct SignoutResponse {
	/// Success message
	pub message: String,
}
