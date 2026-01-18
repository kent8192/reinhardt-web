//! Decrypt command

use crate::output;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct DecryptArgs {
	/// Encrypted configuration file
	#[arg(value_name = "FILE")]
	pub file: PathBuf,

	/// Output file (defaults to `<file>` without `.enc` extension)
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// Decryption key (32 bytes hex)
	#[arg(short, long)]
	pub key: String,

	/// Delete encrypted file after decryption
	#[arg(short = 'd', long)]
	pub delete_encrypted: bool,
}
/// Documentation for `execute`
///
pub(crate) async fn execute(args: DecryptArgs) -> anyhow::Result<()> {
	{
		output::info(&format!("Decrypting configuration file: {:?}", args.file));

		// Check if file exists
		if !args.file.exists() {
			output::error("Encrypted file not found");
			return Err(anyhow::anyhow!("File not found: {:?}", args.file));
		}

		// Decode the decryption key from hex
		let key_bytes = hex::decode(&args.key).map_err(|e| {
			output::error("Invalid decryption key format (expected hex)");
			anyhow::anyhow!("Invalid hex key: {}", e)
		})?;

		if key_bytes.len() != 32 {
			output::error("Decryption key must be exactly 32 bytes (64 hex characters)");
			return Err(anyhow::anyhow!(
				"Invalid key length: {} bytes (expected 32)",
				key_bytes.len()
			));
		}

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
}
