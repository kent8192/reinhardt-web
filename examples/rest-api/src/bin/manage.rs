//! Reinhardt Project Management CLI for example-rest-api
//!
//! This is the project-specific management command interface (equivalent to Django's manage.py).
//! Uses local reinhardt-web crate for development.

use clap::{Parser, Subcommand};
use console::style;
use example_common::manage_cli;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "manage")]
#[command(about = "Reinhardt project management interface", long_about = None)]
#[command(version)]
struct Cli {
	#[command(subcommand)]
	command: Commands,

	/// Verbosity level (can be repeated for more output)
	#[arg(short, long, action = clap::ArgAction::Count)]
	verbosity: u8,
}

#[derive(Subcommand)]
enum Commands {
	/// Create new migrations based on model changes
	Makemigrations {
		/// App labels to create migrations for
		#[arg(value_name = "APP_LABEL")]
		app_labels: Vec<String>,

		/// Dry run - don't actually write files
		#[arg(long)]
		dry_run: bool,

		/// Migration name
		#[arg(short = 'n', long, value_name = "NAME")]
		name: Option<String>,

		/// Check if migrations are missing
		#[arg(long)]
		check: bool,

		/// Create empty migration
		#[arg(long)]
		empty: bool,

		/// Migration directory
		#[arg(long, default_value = "./migrations")]
		migration_dir: PathBuf,
	},

	/// Apply database migrations
	Migrate {
		/// App label to migrate
		#[arg(value_name = "APP_LABEL")]
		app_label: Option<String>,

		/// Migration name to migrate to
		#[arg(value_name = "MIGRATION_NAME")]
		migration_name: Option<String>,

		/// Database connection string
		#[arg(long, value_name = "DATABASE")]
		database: Option<String>,

		/// Fake migration (mark as applied without running)
		#[arg(long)]
		fake: bool,

		/// Fake initial migration only
		#[arg(long)]
		fake_initial: bool,

		/// Show migration plan without applying
		#[arg(long)]
		plan: bool,

		/// Migration directory
		#[arg(long, default_value = "./migrations")]
		migration_dir: PathBuf,
	},

	/// Start the development server
	Runserver {
		/// Server address (default: 127.0.0.1:8000)
		#[arg(value_name = "ADDRESS", default_value = "127.0.0.1:8000")]
		address: String,

		/// Disable auto-reload
		#[arg(long)]
		noreload: bool,

		/// Serve static files in development mode
		#[arg(long)]
		insecure: bool,
	},

	/// Run an interactive Rust shell (REPL)
	Shell {
		/// Execute a command and exit
		#[arg(short = 'c', long, value_name = "COMMAND")]
		command: Option<String>,
	},

	/// Check the project for common issues
	Check {
		/// Check specific app
		#[arg(value_name = "APP_LABEL")]
		app_label: Option<String>,

		/// Deploy check (stricter checks)
		#[arg(long)]
		deploy: bool,
	},

	/// Collect static files into STATIC_ROOT
	Collectstatic {
		/// Clear existing files before collecting
		#[arg(long)]
		clear: bool,

		/// Do not prompt for confirmation
		#[arg(long)]
		no_input: bool,

		/// Do not actually collect, just show what would be collected
		#[arg(long)]
		dry_run: bool,

		/// Create symbolic links instead of copying files
		#[arg(long)]
		link: bool,

		/// Ignore file patterns (glob)
		#[arg(long, value_name = "PATTERN")]
		ignore: Vec<String>,
	},

	/// Display all registered URL patterns
	Showurls {
		/// Show only named URLs
		#[arg(long)]
		names: bool,
	},
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	let result = match cli.command {
		Commands::Makemigrations {
			app_labels,
			dry_run,
			name,
			check,
			empty,
			migration_dir,
		} => {
			manage_cli::run_makemigrations(
				app_labels,
				dry_run,
				name,
				check,
				empty,
				migration_dir,
				cli.verbosity,
			)
			.await
		}
		Commands::Migrate {
			app_label,
			migration_name,
			database,
			fake,
			fake_initial,
			plan,
			migration_dir,
		} => {
			manage_cli::run_migrate(
				app_label,
				migration_name,
				database,
				fake,
				fake_initial,
				plan,
				migration_dir,
				cli.verbosity,
			)
			.await
		}
		Commands::Runserver {
			address,
			noreload,
			insecure,
		} => manage_cli::run_runserver(address, noreload, insecure, cli.verbosity).await,
		Commands::Shell { command } => manage_cli::run_shell(command, cli.verbosity).await,
		Commands::Check { app_label, deploy } => {
			manage_cli::run_check(app_label, deploy, cli.verbosity).await
		}
		Commands::Collectstatic {
			clear,
			no_input,
			dry_run,
			link,
			ignore,
		} => {
			manage_cli::run_collectstatic(clear, no_input, dry_run, link, ignore, cli.verbosity)
				.await
		}
		Commands::Showurls { names } => manage_cli::run_showurls(names, cli.verbosity).await,
	};

	if let Err(e) = result {
		eprintln!("{} {}", style("Error:").red().bold(), e);
		process::exit(1);
	}

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	run().await
}
