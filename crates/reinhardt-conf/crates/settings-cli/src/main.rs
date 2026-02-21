//! CLI tool for reinhardt-settings

mod commands;
mod output;

use clap::{Parser, Subcommand};
use commands::{decrypt, diff, encrypt, set, show, validate};

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

fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Validate(args) => validate::execute(args),
		Commands::Show(args) => show::execute(args),
		Commands::Set(args) => set::execute(args),
		Commands::Diff(args) => diff::execute(args),
		Commands::Encrypt(args) => encrypt::execute(args),
		Commands::Decrypt(args) => decrypt::execute(args),
	}
}
