//! Encrypt command
//!
//! # Security
//!
//! - File operations use direct error handling instead of check-then-act patterns
//!   to avoid TOCTOU race conditions
//! - File size limits prevent OOM from extremely large inputs
//! - Encrypted output is written with restricted permissions (0600)

use crate::output;
use clap::Args;
use std::path::PathBuf;

use super::key;

/// Maximum configuration file size for encryption (50 MB).
const MAX_CONFIG_FILE_SIZE: u64 = 50 * 1024 * 1024;

#[derive(Args)]
pub(crate) struct EncryptArgs {
	/// Configuration file to encrypt
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Output file (defaults to `<file>.enc`)
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// Encryption key (32 bytes hex).
	/// WARNING: Using this option exposes the key in process list and shell history.
	/// Prefer REINHARDT_ENCRYPTION_KEY environment variable or interactive prompt.
	#[arg(short, long)]
	pub key: Option<String>,

	/// Delete original file after encryption
	#[arg(short = 'd', long)]
	pub delete_original: bool,
}

/// Encrypt a configuration file
///
/// The encryption key can be provided via:
/// 1. Environment variable: `REINHARDT_ENCRYPTION_KEY` (recommended)
/// 2. Interactive stdin prompt (if terminal available)
/// 3. `--key` argument (not recommended for security reasons)
pub(crate) fn execute(args: EncryptArgs) -> anyhow::Result<()> {
	output::info(&format!("Encrypting configuration file: {:?}", args.file));

	// Check file size and existence in one operation (TOCTOU mitigation)
	let metadata = std::fs::metadata(&args.file).map_err(|e| {
		output::error("Configuration file not found or inaccessible");
		anyhow::anyhow!("Cannot access file {:?}: {}", args.file, e)
	})?;

	if metadata.len() > MAX_CONFIG_FILE_SIZE {
		return Err(anyhow::anyhow!(
			"Configuration file exceeds maximum size ({} bytes, limit {} bytes)",
			metadata.len(),
			MAX_CONFIG_FILE_SIZE
		));
	}

	// Get encryption key from CLI arg, env var, or stdin prompt
	// Key material is wrapped in Zeroizing and will be securely erased on drop
	let key_source = key::get_encryption_key(args.key.as_deref())?;

	// Read the file content
	let content = std::fs::read_to_string(&args.file)?;

	// Encrypt using the encryption module
	// Clone key bytes for ConfigEncryptor; original is zeroed when key_source drops
	let encryptor =
		reinhardt_conf::settings::encryption::ConfigEncryptor::new(key_source.key_bytes.to_vec())
			.map_err(|e| anyhow::anyhow!(e))?;
	// Explicitly drop key_source to zero key material as early as possible
	drop(key_source);

	let encrypted_config = encryptor
		.encrypt(content.as_bytes())
		.map_err(|e| anyhow::anyhow!(e))?;
	let encrypted = serde_json::to_vec(&encrypted_config)?;

	// Determine output path
	let output_path = args
		.output
		.unwrap_or_else(|| args.file.with_extension("enc"));

	// Write encrypted content with restricted permissions
	write_encrypted_output(&output_path, &encrypted)?;
	output::success(&format!("Encrypted file written to: {:?}", output_path));

	// Delete original if requested
	if args.delete_original {
		std::fs::remove_file(&args.file)?;
		output::info("Original file deleted");
	}

	Ok(())
}

/// Write encrypted output with restrictive permissions (0600 on Unix).
#[cfg(unix)]
fn write_encrypted_output(path: &PathBuf, content: &[u8]) -> anyhow::Result<()> {
	use std::fs::OpenOptions;
	use std::io::Write;
	use std::os::unix::fs::OpenOptionsExt;

	let mut file = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.mode(0o600)
		.open(path)?;
	file.write_all(content)?;
	file.sync_all()?;
	Ok(())
}

/// Write encrypted output with default permissions on non-Unix platforms.
#[cfg(not(unix))]
fn write_encrypted_output(path: &PathBuf, content: &[u8]) -> anyhow::Result<()> {
	std::fs::write(path, content)?;
	Ok(())
}
