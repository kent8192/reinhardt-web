//! migrate CLI command
//!
//! Applies migrations to the database.

use clap::Parser;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use reinhardt_migrations::{MigrateCommand, MigrateOptions};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "migrate")]
#[command(about = "Updates database schema", long_about = None)]
struct Args {
	/// App label of an application to synchronize the state
	#[arg(value_name = "APP_LABEL")]
	app_label: Option<String>,

	/// Migration name to migrate to (use "zero" to unapply all)
	#[arg(value_name = "MIGRATION_NAME")]
	migration_name: Option<String>,

	/// Database connection string
	#[arg(long, default_value = "sqlite::memory:")]
	database: String,

	/// Mark migrations as run without actually running them
	#[arg(long)]
	fake: bool,

	/// Shows a list of the migration actions that will be performed
	#[arg(long)]
	plan: bool,

	/// Migration directory (default: ./migrations)
	#[arg(long, default_value = "./migrations")]
	migration_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	println!("{}", style("Running migrations...").cyan().bold());

	let options = MigrateOptions {
		app_label: args.app_label,
		migration_name: args.migration_name,
		fake: args.fake,
		database: Some(args.database),
		plan: args.plan,
		migrations_dir: args.migration_dir.to_string_lossy().into_owned(),
	};

	let cmd = MigrateCommand::new(options);

	// Show a progress spinner
	let spinner = ProgressBar::new_spinner();
	spinner.set_style(
		ProgressStyle::default_spinner()
			.template("{spinner:.cyan} {msg}")
			.unwrap(),
	);
	spinner.set_message("Checking migrations...");
	spinner.enable_steady_tick(std::time::Duration::from_millis(100));

	cmd.execute();

	spinner.finish_with_message(
		style("Migrations applied successfully!")
			.green()
			.to_string(),
	);
	Ok(())
}
