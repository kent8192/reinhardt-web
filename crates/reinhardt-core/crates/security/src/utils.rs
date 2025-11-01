//! Security utility functions

use sha2::{Digest, Sha256};
/// Generate a secure random token
///
pub fn generate_token(length: usize) -> String {
	use rand::Rng;
	let mut rng = rand::rng();
	(0..length)
		.map(|_| {
			let idx = rng.random_range(0..62);
			"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
				.chars()
				.nth(idx)
				.unwrap()
		})
		.collect()
}
/// Hash a string with SHA256
///
pub fn hash_sha256(input: &str) -> String {
	let mut hasher = Sha256::new();
	hasher.update(input.as_bytes());
	format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_token() {
		let token = generate_token(32);
		assert_eq!(token.len(), 32);
	}

	#[test]
	fn test_hash_sha256() {
		let hash = hash_sha256("test");
		assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
	}
}
