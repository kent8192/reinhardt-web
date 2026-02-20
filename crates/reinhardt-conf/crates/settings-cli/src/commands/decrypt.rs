//! Decrypt command
//!
//! # Security
//!
//! - Decrypted output files are created with restrictive permissions (0600)
//! - File operations use direct error handling instead of check-then-act patterns
//!   to avoid TOCTOU race conditions

use crate::output;
use clap::Args;
use std::path::PathBuf;

use super::key;

/// Maximum encrypted file size (50 MB).
///
/// Prevents OOM from processing extremely large files.
const MAX_ENCRYPTED_FILE_SIZE: u64 = 50 * 1024 * 1024;

#[derive(Args)]
pub(crate) struct DecryptArgs {
	/// Encrypted configuration file
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Output file (defaults to `<file>` without `.enc` extension)
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// Decryption key (32 bytes hex).
	/// WARNING: Using this option exposes the key in process list and shell history.
	/// Prefer REINHARDT_ENCRYPTION_KEY environment variable or interactive prompt.
	#[arg(short, long)]
	pub key: Option<String>,

	/// Delete encrypted file after decryption
	#[arg(short = 'd', long)]
	pub delete_encrypted: bool,
}

/// Decrypt a configuration file
///
/// The decryption key can be provided via:
/// 1. Environment variable: `REINHARDT_ENCRYPTION_KEY` (recommended)
/// 2. Interactive stdin prompt (if terminal available)
/// 3. `--key` argument (not recommended for security reasons)
pub(crate) async fn execute(args: DecryptArgs) -> anyhow::Result<()> {
	output::info(&format!("Decrypting configuration file: {:?}", args.file));

	// Check file size before reading to prevent OOM (handles existence check implicitly)
	let metadata = std::fs::metadata(&args.file).map_err(|e| {
		output::error("Encrypted file not found or inaccessible");
		anyhow::anyhow!("Cannot access file {:?}: {}", args.file, e)
	})?;

	if metadata.len() > MAX_ENCRYPTED_FILE_SIZE {
		return Err(anyhow::anyhow!(
			"Encrypted file exceeds maximum size ({} bytes, limit {} bytes)",
			metadata.len(),
			MAX_ENCRYPTED_FILE_SIZE
		));
	}

	// Get decryption key from CLI arg, env var, or stdin prompt
	// Key material is wrapped in Zeroizing and will be securely erased on drop
	let key_source = key::get_encryption_key(args.key.as_deref())?;

	// Read the encrypted content
	let encrypted = std::fs::read(&args.file)?;

	// Decrypt using the encryption module
	let encrypted_config: reinhardt_conf::settings::encryption::EncryptedConfig =
		serde_json::from_slice(&encrypted)?;
	// Clone key bytes for ConfigEncryptor; original is zeroed when key_source drops
	let encryptor =
		reinhardt_conf::settings::encryption::ConfigEncryptor::new(key_source.key_bytes.to_vec())
			.map_err(|e| anyhow::anyhow!(e))?;
	// Explicitly drop key_source to zero key material as early as possible
	drop(key_source);

	let decrypted = encryptor
		.decrypt(&encrypted_config)
		.map_err(|e| anyhow::anyhow!(e))?;

	// Determine output path
	let output_path = args.output.unwrap_or_else(|| {
		if args.file.extension().and_then(|s| s.to_str()) == Some("enc") {
			args.file.with_extension("")
		} else {
			args.file.with_extension("dec")
		}
	});

	// Write decrypted content with restrictive permissions (owner read/write only)
	write_with_restricted_permissions(&output_path, &decrypted)?;
	output::success(&format!("Decrypted file written to: {:?}", output_path));

	// Delete encrypted file if requested
	if args.delete_encrypted {
		std::fs::remove_file(&args.file)?;
		output::info("Encrypted file deleted");
	}

	Ok(())
}

/// Write file content with restrictive permissions (0600 on Unix).
///
/// Creates the file atomically with restricted permissions from the start,
/// preventing any window where the file exists with permissive permissions.
#[cfg(unix)]
fn write_with_restricted_permissions(path: &PathBuf, content: &[u8]) -> anyhow::Result<()> {
	use std::fs::OpenOptions;
	use std::io::Write;
	use std::os::unix::fs::OpenOptionsExt;

	let mut file = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.mode(0o600) // Owner read/write only
		.open(path)?;
	file.write_all(content)?;
	file.sync_all()?;
	Ok(())
}

/// Write file content with default permissions on non-Unix platforms.
#[cfg(not(unix))]
fn write_with_restricted_permissions(path: &PathBuf, content: &[u8]) -> anyhow::Result<()> {
	std::fs::write(path, content)?;
	Ok(())
}
