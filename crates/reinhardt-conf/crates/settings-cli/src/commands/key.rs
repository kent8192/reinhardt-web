//! Secure key input handling

use crate::output;
use std::io::{self, IsTerminal};
use zeroize::Zeroizing;

/// Environment variable name for encryption key
pub(crate) const ENV_KEY_NAME: &str = "REINHARDT_ENCRYPTION_KEY";

/// Key source for encryption/decryption
pub(crate) struct KeySource {
	/// The key bytes (32 bytes for AES-256), zeroed on drop
	pub(crate) key_bytes: Zeroizing<Vec<u8>>,
	/// Source of the key (for logging purposes)
	#[allow(dead_code)] // Used for diagnostic logging
	pub(crate) source: KeySourceInfo,
}

/// Information about where the key came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeySourceInfo {
	/// Key was provided via environment variable
	EnvironmentVariable,
	/// Key was read from stdin prompt
	StdinPrompt,
	/// Key was provided via CLI argument (deprecated, security risk)
	CliArgument,
}

/// Get encryption key from various sources with priority:
/// 1. CLI argument (if provided, with security warning)
/// 2. Environment variable REINHARDT_ENCRYPTION_KEY
/// 3. Stdin prompt (if terminal is available)
pub(crate) fn get_encryption_key(cli_key: Option<&str>) -> anyhow::Result<KeySource> {
	// Priority 1: CLI argument (with security warning)
	if let Some(key) = cli_key {
		output::warning("WARNING: Passing encryption key via --key argument is insecure!");
		output::warning(
			"         The key will be visible in process list, shell history, and logs.",
		);
		output::warning(
			"         Consider using environment variable REINHARDT_ENCRYPTION_KEY instead.",
		);

		let key_bytes = decode_and_validate_key(key)?;
		return Ok(KeySource {
			key_bytes: Zeroizing::new(key_bytes),
			source: KeySourceInfo::CliArgument,
		});
	}

	// Priority 2: Environment variable
	// Using nested if to avoid let chains (requires Rust 2024)
	#[allow(clippy::collapsible_if)]
	if let Ok(key) = std::env::var(ENV_KEY_NAME) {
		if !key.is_empty() {
			output::info(&format!(
				"Using encryption key from environment variable {}",
				ENV_KEY_NAME
			));
			let key_bytes = decode_and_validate_key(&key)?;
			return Ok(KeySource {
				key_bytes: Zeroizing::new(key_bytes),
				source: KeySourceInfo::EnvironmentVariable,
			});
		}
	}

	// Priority 3: Stdin prompt
	if io::stdin().is_terminal() {
		let prompt = "Enter encryption key (32 bytes hex): ";
		let key = rpassword::prompt_password(prompt)
			.map_err(|e| anyhow::anyhow!("Failed to read key from stdin: {}", e))?;

		let key_bytes = decode_and_validate_key(&key)?;
		return Ok(KeySource {
			key_bytes: Zeroizing::new(key_bytes),
			source: KeySourceInfo::StdinPrompt,
		});
	}

	// No key source available
	Err(anyhow::anyhow!(
		"No encryption key provided. Use one of:\n\
		 \x20  1. Environment variable: export {}=<key>\n\
		 \x20  2. Stdin prompt (interactive mode)\n\
		 \x20  3. CLI argument: --key <key> (not recommended)",
		ENV_KEY_NAME
	))
}

/// Decode hex key and validate length
fn decode_and_validate_key(key: &str) -> anyhow::Result<Vec<u8>> {
	let key_bytes = hex::decode(key.trim()).map_err(|e| {
		output::error("Invalid encryption key format (expected hex)");
		anyhow::anyhow!("Invalid hex key: {}", e)
	})?;

	if key_bytes.len() != 32 {
		output::error("Encryption key must be exactly 32 bytes (64 hex characters)");
		return Err(anyhow::anyhow!(
			"Invalid key length: {} bytes (expected 32)",
			key_bytes.len()
		));
	}

	Ok(key_bytes)
}
