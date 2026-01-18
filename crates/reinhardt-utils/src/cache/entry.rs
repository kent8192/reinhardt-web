//! Internal cache entry structure

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Cache entry with expiration and timestamp tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CacheEntry {
	pub(crate) value: Vec<u8>,
	pub(crate) expires_at: Option<SystemTime>,
	pub(crate) created_at: SystemTime,
	pub(crate) accessed_at: Option<SystemTime>,
}

impl CacheEntry {
	pub(crate) fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
		let now = SystemTime::now();
		let expires_at = ttl.map(|d| now + d);
		Self {
			value,
			expires_at,
			created_at: now,
			accessed_at: None,
		}
	}

	pub(crate) fn is_expired(&self) -> bool {
		if let Some(expires_at) = self.expires_at {
			SystemTime::now() > expires_at
		} else {
			false
		}
	}

	/// Update the last accessed timestamp
	pub(crate) fn touch(&mut self) {
		self.accessed_at = Some(SystemTime::now());
	}
}
