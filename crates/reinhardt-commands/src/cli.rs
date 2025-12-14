//! CLI runner for Reinhardt management commands
//!
//! This module provides a unified interface for executing commands from generated `manage.rs` files.
//! It handles argument parsing, command context creation, and command execution.

#[cfg(feature = "migrations")]
use crate::MakeMigrationsCommand;
use crate::base::BaseCommand;
use crate::collectstatic::{CollectStaticCommand, CollectStaticOptions};
use crate::{CheckCommand, CommandContext, MigrateCommand, RunServerCommand, ShellCommand};
use clap::{Parser, Subcommand};
use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::profile::Profile;
use reinhardt_conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
use serde_json::Value;
use std::env;
#[allow(unused)]
use std::path::PathBuf;

#[cfg(feature = "routers")]
use crate::builtin::ShowUrlsCommand;

/// Reinhardt Project Management CLI
///
/// This is the internal CLI parser used by `execute_from_command_line()`.
#[derive(Parser)]
#[command(name = "manage")]
#[command(about = "Reinhardt management interface", long_about = None)]
#[command(version)]
struct Cli {
	#[command(subcommand)]
	command: Commands,

	/// Verbosity level (can be repeated for more output)
	#[arg(short, long, action = clap::ArgAction::Count)]
	verbosity: u8,
}

/// Command-line interface commands
///
/// This enum defines all available management commands.
#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
	/// Create new migrations based on model changes
	#[cfg(feature = "migrations")]
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

/// Execute commands from command-line arguments
///
/// This is the Django-style entry point that parses command-line arguments
/// and executes the appropriate command. This should be called from `manage.rs`.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error message on failure.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_commands::execute_from_command_line;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     unsafe {
///         std::env::set_var("REINHARDT_SETTINGS_MODULE", "myproject.config.settings");
///     }
///
///     if let Err(e) = execute_from_command_line().await {
///         eprintln!("Error: {}", e);
///         std::process::exit(1);
///     }
///     Ok(())
/// }
/// ```
pub async fn execute_from_command_line() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();
	run_command(cli.command, cli.verbosity).await
}

/// Execute a command with the given verbosity level
///
/// This is the internal entry point for executing commands.
/// For most use cases, prefer using `execute_from_command_line()` instead.
///
/// # Arguments
///
/// * `command` - The command to execute
/// * `verbosity` - Verbosity level (0-3, higher is more verbose)
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error message on failure.
pub async fn run_command(
	command: Commands,
	verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	match command {
		#[cfg(feature = "migrations")]
		Commands::Makemigrations {
			app_labels,
			dry_run,
			name,
			check,
			empty,
			migration_dir: _,
		} => execute_makemigrations(app_labels, dry_run, name, check, empty, verbosity).await,
		Commands::Migrate {
			app_label,
			migration_name,
			database,
			fake,
			fake_initial,
			plan,
		} => {
			execute_migrate(MigrateParams {
				app_label,
				migration_name,
				database,
				fake,
				fake_initial,
				plan,
				verbosity,
			})
			.await
		}
		Commands::Runserver {
			address,
			noreload,
			insecure,
		} => execute_runserver(address, noreload, insecure, verbosity).await,
		Commands::Shell { command } => execute_shell(command, verbosity).await,
		Commands::Check { app_label, deploy } => execute_check(app_label, deploy, verbosity).await,
		Commands::Collectstatic {
			clear,
			no_input,
			dry_run,
			link,
			ignore,
		} => execute_collectstatic(clear, no_input, dry_run, link, ignore, verbosity).await,
		Commands::Showurls { names } => execute_showurls(names, verbosity).await,
	}
}

/// Execute the makemigrations command
#[cfg(feature = "migrations")]
async fn execute_makemigrations(
	app_labels: Vec<String>,
	dry_run: bool,
	name: Option<String>,
	check: bool,
	empty: bool,
	verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);

	if !app_labels.is_empty() {
		for label in app_labels {
			ctx.add_arg(label);
		}
	}

	if dry_run {
		ctx.set_option("dry-run".to_string(), "true".to_string());
	}
	if check {
		ctx.set_option("check".to_string(), "true".to_string());
	}
	if empty {
		ctx.set_option("empty".to_string(), "true".to_string());
	}
	if let Some(n) = name {
		ctx.set_option("name".to_string(), n);
	}

	let cmd = MakeMigrationsCommand;
	cmd.execute(&ctx).await.map_err(|e| e.into())
}

/// Parameters for the migrate command
#[derive(Debug)]
struct MigrateParams {
	app_label: Option<String>,
	migration_name: Option<String>,
	database: Option<String>,
	fake: bool,
	fake_initial: bool,
	plan: bool,
	verbosity: u8,
}

/// Execute the migrate command
async fn execute_migrate(params: MigrateParams) -> Result<(), Box<dyn std::error::Error>> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(params.verbosity);

	if let Some(app) = params.app_label {
		ctx.add_arg(app);
		if let Some(migration) = params.migration_name {
			ctx.add_arg(migration);
		}
	}

	if params.fake {
		ctx.set_option("fake".to_string(), "true".to_string());
	}
	if params.fake_initial {
		ctx.set_option("fake-initial".to_string(), "true".to_string());
	}
	if params.plan {
		ctx.set_option("plan".to_string(), "true".to_string());
	}
	if let Some(db) = params.database {
		ctx.set_option("database".to_string(), db);
	}

	let cmd = MigrateCommand;
	cmd.execute(&ctx).await.map_err(|e| e.into())
}

/// Execute the runserver command
async fn execute_runserver(
	address: String,
	noreload: bool,
	insecure: bool,
	verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);
	ctx.add_arg(address);

	if noreload {
		ctx.set_option("noreload".to_string(), "true".to_string());
	}
	if insecure {
		ctx.set_option("insecure".to_string(), "true".to_string());
	}

	let cmd = RunServerCommand;
	cmd.execute(&ctx).await.map_err(|e| e.into())
}

/// Execute the shell command
async fn execute_shell(
	command: Option<String>,
	verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);

	if let Some(cmd_str) = command {
		ctx.set_option("command".to_string(), cmd_str);
	}

	let cmd = ShellCommand;
	cmd.execute(&ctx).await.map_err(|e| e.into())
}

/// Execute the check command
async fn execute_check(
	app_label: Option<String>,
	deploy: bool,
	verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);

	if let Some(app) = app_label {
		ctx.add_arg(app);
	}

	if deploy {
		ctx.set_option("deploy".to_string(), "true".to_string());
	}

	let cmd = CheckCommand;
	cmd.execute(&ctx).await.map_err(|e| e.into())
}

/// Execute the collectstatic command
async fn execute_collectstatic(
	clear: bool,
	no_input: bool,
	dry_run: bool,
	link: bool,
	ignore: Vec<String>,
	verbosity: u8,
) -> Result<(), Box<dyn std::error::Error>> {
	// Load settings from TOML files
	let profile_str = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
	let profile = Profile::parse(&profile_str);

	let base_dir = env::current_dir().expect("Failed to get current directory");
	let settings_dir = base_dir.join("settings");

	let merged = SettingsBuilder::new()
		.profile(profile)
		.add_source(
			DefaultSource::new()
				.with_value("debug", Value::Bool(true))
				.with_value("language_code", Value::String("en-us".to_string()))
				.with_value("time_zone", Value::String("UTC".to_string())),
		)
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.add_source(TomlFileSource::new(settings_dir.join("base.toml")))
		.add_source(TomlFileSource::new(
			settings_dir.join(format!("{}.toml", profile_str)),
		))
		.build()?;

	let settings = merged.into_typed::<reinhardt_conf::Settings>()?;

	// Convert Settings to StaticFilesConfig
	let config = settings
		.get_static_config()
		.map_err(|e| format!("Failed to get static config: {}", e))?;

	// Create options
	let options = CollectStaticOptions {
		clear,
		no_input,
		dry_run,
		interactive: !no_input,
		link,
		ignore_patterns: ignore,
		verbosity,
	};

	// Create and execute command in blocking context
	let mut cmd = CollectStaticCommand::new(config, options);
	let result = tokio::task::spawn_blocking(move || {
		// Call the sync execute() method directly (not the BaseCommand trait method)
		CollectStaticCommand::execute(&mut cmd)
	})
	.await;

	match result {
		Ok(Ok(_stats)) => Ok(()),
		Ok(Err(e)) => Err(Box::new(e) as Box<dyn std::error::Error>),
		Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
	}
}

/// Execute the showurls command
#[cfg(feature = "routers")]
async fn execute_showurls(names: bool, verbosity: u8) -> Result<(), Box<dyn std::error::Error>> {
	let mut ctx = CommandContext::default();
	ctx.set_verbosity(verbosity);

	if names {
		ctx.set_option("names".to_string(), "true".to_string());
	}

	let cmd = ShowUrlsCommand;
	cmd.execute(&ctx).await.map_err(|e| e.into())
}

#[cfg(not(feature = "routers"))]
async fn execute_showurls(_names: bool, _verbosity: u8) -> Result<(), Box<dyn std::error::Error>> {
	use colored::Colorize;
	eprintln!(
		"{}",
		"showurls command requires 'routers' feature".red().bold()
	);
	eprintln!("Enable it in your Cargo.toml:");
	eprintln!("  [dependencies]");
	eprintln!("  reinhardt-commands = {{ version = \"0.1.0\", features = [\"routers\"] }}");
	std::process::exit(1);
}
