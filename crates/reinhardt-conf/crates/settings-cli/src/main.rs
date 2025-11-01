//! CLI tool for reinhardt-settings

mod commands;
mod output;

use clap::{Parser, Subcommand};
use commands::*;

#[derive(Parser)]
#[command(name = "reinhardt-settings")]
#[command(about = "CLI tool for managing reinhardt settings", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Validate configuration files
	Validate(validate::ValidateArgs),

	/// Show configuration values
	Show(show::ShowArgs),

	/// Set a configuration value
	Set(set::SetArgs),

	/// Compare two configuration files
	Diff(diff::DiffArgs),

	/// Encrypt a configuration file
	Encrypt(encrypt::EncryptArgs),

	/// Decrypt a configuration file
	Decrypt(decrypt::DecryptArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Validate(args) => validate::execute(args).await,
		Commands::Show(args) => show::execute(args).await,
		Commands::Set(args) => set::execute(args).await,
		Commands::Diff(args) => diff::execute(args).await,
		Commands::Encrypt(args) => encrypt::execute(args).await,
		Commands::Decrypt(args) => decrypt::execute(args).await,
	}
}
