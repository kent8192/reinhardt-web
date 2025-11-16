//! Reinhardt Admin CLI
//!
//! Global command-line tool for Reinhardt project management.
//! This is the equivalent of Django's `django-admin` command.
//!
//! ## Installation
//!
//! ```bash
//! cargo install reinhardt-admin --features binary
//! ```
//!
//! ## Usage
//!
//! ```bash
//! reinhardt-admin startproject myproject
//! reinhardt-admin startapp myapp
//! reinhardt-admin help
//! ```

use clap::{Parser, Subcommand};
use reinhardt_commands::{
	BaseCommand, CommandContext, CommandResult, StartAppCommand, StartProjectCommand,
};
use std::process;

#[derive(Parser)]
#[command(name = "reinhardt-admin")]
#[command(about = "Reinhardt project administration utility", long_about = None)]
#[command(version)]
struct Cli {
	#[command(subcommand)]
	command: Commands,

	/// Verbosity level (can be repeated)
	#[arg(short, long, action = clap::ArgAction::Count)]
	verbosity: u8,
}

#[derive(Subcommand)]
enum Commands {
	/// Create a new Reinhardt project
	Startproject {
		/// Name of the project
		#[arg(value_name = "PROJECT_NAME")]
		name: String,

		/// Directory to create the project in (defaults to current directory)
		#[arg(value_name = "DIRECTORY")]
		directory: Option<String>,

		/// Project template type: mtv (Model-Template-View) or restful (RESTful API)
		#[arg(short = 't', long, default_value = "restful")]
		template_type: String,
	},

	/// Create a new Reinhardt app
	Startapp {
		/// Name of the app
		#[arg(value_name = "APP_NAME")]
		name: String,

		/// Directory to create the app in (defaults to current directory)
		#[arg(value_name = "DIRECTORY")]
		directory: Option<String>,

		/// App template type: mtv or restful
		#[arg(short = 't', long, default_value = "restful")]
		template_type: String,
	},

	/// Display help information for a specific command
	Help {
		/// Command name to get help for
		#[arg(value_name = "COMMAND")]
		command: Option<String>,
	},

	/// Display version information
	Version,
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();

	let result = match cli.command {
		Commands::Startproject {
			name,
			directory,
			template_type,
		} => run_startproject(name, directory, template_type, cli.verbosity).await,
		Commands::Startapp {
			name,
			directory,
			template_type,
		} => run_startapp(name, directory, template_type, cli.verbosity).await,
	};

	if let Err(e) = result {
		eprintln!("Error: {}", e);
		process::exit(1);
	}
}

async fn run_startproject(
	name: String,
	directory: Option<String>,
	template_type: String,
	verbosity: u8,
) -> CommandResult<()> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);
	ctx.add_arg(name);
	if let Some(dir) = directory {
		ctx.add_arg(dir);
	}
	ctx.set_option("type".to_string(), template_type);

	let cmd = StartProjectCommand;
	cmd.execute(&ctx).await
}

async fn run_startapp(
	name: String,
	directory: Option<String>,
	template_type: String,
	verbosity: u8,
) -> CommandResult<()> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);
	ctx.add_arg(name);
	if let Some(dir) = directory {
		ctx.add_arg(dir);
	}
	ctx.set_option("type".to_string(), template_type);

	let cmd = StartAppCommand;
	cmd.execute(&ctx).await
}
