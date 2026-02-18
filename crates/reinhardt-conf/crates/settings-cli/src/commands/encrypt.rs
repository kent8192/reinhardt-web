//! Encrypt command

use crate::output;
use clap::Args;
use std::path::PathBuf;

use super::key;

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
pub(crate) async fn execute(args: EncryptArgs) -> anyhow::Result<()> {
	output::info(&format!("Encrypting configuration file: {:?}", args.file));

	// Check if file exists
	if !args.file.exists() {
		output::error("Configuration file not found");
		return Err(anyhow::anyhow!("File not found: {:?}", args.file));
	}

	// Get encryption key from CLI arg, env var, or stdin prompt
	let key_source = key::get_encryption_key(args.key.as_deref())?;
	let key_bytes = key_source.key_bytes;

	// Read the file content
	let content = std::fs::read_to_string(&args.file)?;

	// Encrypt using the encryption module
	let encryptor = reinhardt_conf::settings::encryption::ConfigEncryptor::new(key_bytes)
		.map_err(|e| anyhow::anyhow!(e))?;
	let encrypted_config = encryptor
		.encrypt(content.as_bytes())
		.map_err(|e| anyhow::anyhow!(e))?;
	let encrypted = serde_json::to_vec(&encrypted_config)?;

	// Determine output path
	let output_path = args
		.output
		.unwrap_or_else(|| args.file.with_extension("enc"));

	// Write encrypted content
	std::fs::write(&output_path, encrypted)?;
	output::success(&format!("Encrypted file written to: {:?}", output_path));

	// Delete original if requested
	if args.delete_original {
		std::fs::remove_file(&args.file)?;
		output::info("Original file deleted");
	}

	Ok(())
}
