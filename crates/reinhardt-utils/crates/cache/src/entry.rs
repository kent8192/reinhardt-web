//! Internal cache entry structure

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Cache entry with expiration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CacheEntry {
	pub(crate) value: Vec<u8>,
	pub(crate) expires_at: Option<SystemTime>,
}

impl CacheEntry {
	pub(crate) fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
		let expires_at = ttl.map(|d| SystemTime::now() + d);
		Self { value, expires_at }
	}

	pub(crate) fn is_expired(&self) -> bool {
		if let Some(expires_at) = self.expires_at {
			SystemTime::now() > expires_at
		} else {
			false
		}
	}
}
