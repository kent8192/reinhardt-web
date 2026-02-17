//! Decrypt command

use crate::output;
use clap::Args;
use std::path::PathBuf;

use super::key;

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

	// Check if file exists
	if !args.file.exists() {
		output::error("Encrypted file not found");
		return Err(anyhow::anyhow!("File not found: {:?}", args.file));
	}

	// Get decryption key from CLI arg, env var, or stdin prompt
	let key_source = key::get_encryption_key(args.key.as_deref())?;
	let key_bytes = key_source.key_bytes;

	// Read the encrypted content
	let encrypted = std::fs::read(&args.file)?;

	// Decrypt using the encryption module
	let encrypted_config: reinhardt_conf::settings::encryption::EncryptedConfig =
		serde_json::from_slice(&encrypted)?;
	let encryptor = reinhardt_conf::settings::encryption::ConfigEncryptor::new(key_bytes)
		.map_err(|e| anyhow::anyhow!(e))?;
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

	// Write decrypted content
	std::fs::write(&output_path, decrypted)?;
	output::success(&format!("Decrypted file written to: {:?}", output_path));

	// Delete encrypted file if requested
	if args.delete_encrypted {
		std::fs::remove_file(&args.file)?;
		output::info("Encrypted file deleted");
	}

	Ok(())
}
