//! Google Cloud Storage backend implementation.
//!
//! **Note**: This backend is not yet implemented (Phase 2).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::config::GcsConfig;
use crate::{Result, StorageBackend};

/// Google Cloud Storage backend.
#[derive(Debug, Clone)]
pub struct GcsStorage {
	_config: GcsConfig,
}

impl GcsStorage {
	/// Create a new GCS storage backend.
	pub fn new(config: GcsConfig) -> Result<Self> {
		Ok(Self { _config: config })
	}
}

#[allow(clippy::unimplemented)] // Phase 2 backend not yet implemented
#[async_trait]
impl StorageBackend for GcsStorage {
	async fn save(&self, _name: &str, _content: &[u8]) -> Result<String> {
		unimplemented!("GCS save not yet implemented")
	}

	async fn open(&self, _name: &str) -> Result<Vec<u8>> {
		unimplemented!("GCS open not yet implemented")
	}

	async fn delete(&self, _name: &str) -> Result<()> {
		unimplemented!("GCS delete not yet implemented")
	}

	async fn exists(&self, _name: &str) -> Result<bool> {
		unimplemented!("GCS exists not yet implemented")
	}

	async fn url(&self, _name: &str, _expiry_secs: u64) -> Result<String> {
		unimplemented!("GCS url not yet implemented")
	}

	async fn size(&self, _name: &str) -> Result<u64> {
		unimplemented!("GCS size not yet implemented")
	}

	async fn get_modified_time(&self, _name: &str) -> Result<DateTime<Utc>> {
		unimplemented!("GCS get_modified_time not yet implemented")
	}
}
