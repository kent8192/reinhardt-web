//! Encrypt command

use crate::output;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct EncryptArgs {
	/// Configuration file to encrypt
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Output file (defaults to `<file>.enc`)
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// Encryption key (32 bytes hex)
	#[arg(short, long)]
	pub key: String,

	/// Delete original file after encryption
	#[arg(short = 'd', long)]
	pub delete_original: bool,
}
/// Documentation for `execute`
///
pub(crate) async fn execute(args: EncryptArgs) -> anyhow::Result<()> {
	{
		output::info(&format!("Encrypting configuration file: {:?}", args.file));

		// Check if file exists
		if !args.file.exists() {
			output::error("Configuration file not found");
			return Err(anyhow::anyhow!("File not found: {:?}", args.file));
		}

		// Decode the encryption key from hex
		let key_bytes = hex::decode(&args.key).map_err(|e| {
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
}
