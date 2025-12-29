//! Reinhardt Admin CLI
//!
//! Global command-line tool for Reinhardt project management.
//! This is the equivalent of Django's `django-admin` command.
//!
//! ## Installation
//!
//! ```bash
//! cargo install reinhardt-commands
//! ```
//!
//! ## Usage
//!
//! ```bash
//! reinhardt-admin startproject myproject
//! reinhardt-admin startapp myapp
//! reinhardt-admin fmt src/
//! reinhardt-admin help
//! ```

mod ast_formatter;
mod formatter;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use colored::Colorize;
use reinhardt_commands::{
	BaseCommand, CommandContext, CommandResult, PluginDisableCommand, PluginEnableCommand,
	PluginInfoCommand, PluginInstallCommand, PluginListCommand, PluginRemoveCommand,
	PluginSearchCommand, PluginUpdateCommand, StartAppCommand, StartProjectCommand,
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

	/// Manage Reinhardt plugins (Dentdelion)
	Plugin {
		#[command(subcommand)]
		subcommand: PluginCommands,
	},

	/// Format page! macro DSL in Rust source files
	Fmt {
		/// Path to file or directory to format
		#[arg(value_name = "PATH")]
		path: PathBuf,

		/// Check if files are formatted without modifying them
		#[arg(long)]
		check: bool,
	},
}

/// Plugin management subcommands
#[derive(Subcommand)]
enum PluginCommands {
	/// List installed plugins
	List {
		/// Show detailed information
		#[arg(short, long)]
		verbose: bool,

		/// Show only enabled plugins
		#[arg(long)]
		enabled: bool,

		/// Show only disabled plugins
		#[arg(long)]
		disabled: bool,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},

	/// Show plugin information
	Info {
		/// Plugin name (e.g., auth-delion)
		#[arg(value_name = "NAME")]
		name: String,

		/// Fetch info from crates.io instead of local
		#[arg(long)]
		remote: bool,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},

	/// Install a plugin from crates.io
	Install {
		/// Plugin name (e.g., auth-delion)
		#[arg(value_name = "NAME")]
		name: String,

		/// Specific version to install
		#[arg(long)]
		version: Option<String>,

		/// Skip confirmation prompt
		#[arg(short, long)]
		yes: bool,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},

	/// Remove a plugin
	Remove {
		/// Plugin name to remove
		#[arg(value_name = "NAME")]
		name: String,

		/// Also remove plugin configuration
		#[arg(long)]
		purge: bool,

		/// Skip confirmation prompt
		#[arg(short, long)]
		yes: bool,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},

	/// Enable a disabled plugin
	Enable {
		/// Plugin name to enable
		#[arg(value_name = "NAME")]
		name: String,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},

	/// Disable an enabled plugin
	Disable {
		/// Plugin name to disable
		#[arg(value_name = "NAME")]
		name: String,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},

	/// Search for plugins on crates.io
	Search {
		/// Search query
		#[arg(value_name = "QUERY")]
		query: String,

		/// Maximum number of results
		#[arg(long, default_value = "10")]
		limit: u64,
	},

	/// Update plugin(s) to latest version
	Update {
		/// Plugin name (omit for --all)
		#[arg(value_name = "NAME")]
		name: Option<String>,

		/// Update all plugins
		#[arg(long)]
		all: bool,

		/// Skip confirmation prompt
		#[arg(short, long)]
		yes: bool,

		/// Project root directory
		#[arg(long)]
		project_root: Option<String>,
	},
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
		Commands::Plugin { subcommand } => run_plugin(subcommand, cli.verbosity).await,
		Commands::Fmt { path, check } => run_fmt(path, check, cli.verbosity),
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

async fn run_plugin(subcommand: PluginCommands, verbosity: u8) -> CommandResult<()> {
	match subcommand {
		PluginCommands::List {
			verbose,
			enabled,
			disabled,
			project_root,
		} => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			if verbose {
				ctx.set_option("verbose".to_string(), "true".to_string());
			}
			if enabled {
				ctx.set_option("enabled".to_string(), "true".to_string());
			}
			if disabled {
				ctx.set_option("disabled".to_string(), "true".to_string());
			}
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginListCommand.execute(&ctx).await
		}
		PluginCommands::Info {
			name,
			remote,
			project_root,
		} => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			ctx.add_arg(name);
			if remote {
				ctx.set_option("remote".to_string(), "true".to_string());
			}
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginInfoCommand.execute(&ctx).await
		}
		PluginCommands::Install {
			name,
			version,
			yes,
			project_root,
		} => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			ctx.add_arg(name);
			if let Some(v) = version {
				ctx.set_option("version".to_string(), v);
			}
			if yes {
				ctx.set_option("yes".to_string(), "true".to_string());
			}
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginInstallCommand.execute(&ctx).await
		}
		PluginCommands::Remove {
			name,
			purge,
			yes,
			project_root,
		} => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			ctx.add_arg(name);
			if purge {
				ctx.set_option("purge".to_string(), "true".to_string());
			}
			if yes {
				ctx.set_option("yes".to_string(), "true".to_string());
			}
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginRemoveCommand.execute(&ctx).await
		}
		PluginCommands::Enable { name, project_root } => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			ctx.add_arg(name);
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginEnableCommand.execute(&ctx).await
		}
		PluginCommands::Disable { name, project_root } => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			ctx.add_arg(name);
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginDisableCommand.execute(&ctx).await
		}
		PluginCommands::Search { query, limit } => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			ctx.add_arg(query);
			ctx.set_option("limit".to_string(), limit.to_string());
			PluginSearchCommand.execute(&ctx).await
		}
		PluginCommands::Update {
			name,
			all,
			yes,
			project_root,
		} => {
			let mut ctx = CommandContext::default();
			ctx.set_verbosity(verbosity);
			if let Some(n) = name {
				ctx.add_arg(n);
			}
			if all {
				ctx.set_option("all".to_string(), "true".to_string());
			}
			if yes {
				ctx.set_option("yes".to_string(), "true".to_string());
			}
			if let Some(root) = project_root {
				ctx.set_option("project-root".to_string(), root);
			}
			PluginUpdateCommand.execute(&ctx).await
		}
	}
}

fn run_fmt(path: PathBuf, check: bool, verbosity: u8) -> CommandResult<()> {
	use ast_formatter::AstPageFormatter;
	use formatter::collect_rust_files;

	let files = collect_rust_files(&path).map_err(|e| {
		reinhardt_commands::CommandError::ExecutionError(format!("Failed to collect files: {}", e))
	})?;

	if files.is_empty() {
		if verbosity > 0 {
			println!("No Rust files found in {:?}", path);
		}
		return Ok(());
	}

	let formatter = AstPageFormatter::new();
	let mut formatted_count = 0;
	let mut unchanged_count = 0;
	let mut ignored_count = 0;
	let mut error_count = 0;

	let total_files = files.len();

	for (index, file_path) in files.iter().enumerate() {
		let progress = format!("[{}/{}]", index + 1, total_files);
		let content = std::fs::read_to_string(file_path).map_err(|e| {
			reinhardt_commands::CommandError::ExecutionError(format!(
				"Failed to read {}: {}",
				file_path.display(),
				e
			))
		})?;

		match formatter.format(&content) {
			Ok(result) => {
				// Check if formatting was skipped
				if let Some(reason) = &result.skipped {
					use crate::ast_formatter::SkipReason;
					match reason {
						SkipReason::NoPageMacro => {
							// Skip files without page! macros (no logging, no counting)
							continue;
						}
						SkipReason::FileWideMarker | SkipReason::AllMacrosIgnored => {
							// Log ignored files with reason
							ignored_count += 1;
							println!(
								"{} {} {} ({})",
								progress.bright_blue(),
								"Ignored:".yellow(),
								file_path.display(),
								reason
							);
							continue;
						}
					}
				}

				if result.content != content {
					if check {
						// Check mode: report unformatted files
						println!("{} Would format: {}", progress, file_path.display());
						formatted_count += 1;
					} else {
						// Format mode: write changes
						std::fs::write(file_path, &result.content).map_err(|e| {
							reinhardt_commands::CommandError::ExecutionError(format!(
								"Failed to write {}: {}",
								file_path.display(),
								e
							))
						})?;
						// Color output: success in green
						println!(
							"{} {} {}",
							progress.bright_blue(),
							"Formatted:".green(),
							file_path.display()
						);
						formatted_count += 1;
					}
				} else {
					unchanged_count += 1;
					// Always show unchanged files (verbosity condition removed)
					println!(
						"{} {} {}",
						progress.bright_blue(),
						"Unchanged:".dimmed(),
						file_path.display()
					);
				}
			}
			Err(e) => {
				// Color output: errors in red
				eprintln!(
					"{} {} {}: {}",
					progress.bright_blue(),
					"Error".red(),
					file_path.display(),
					e
				);
				error_count += 1;
			}
		}
	}

	// Always show summary (remove verbosity condition)
	println!();
	if check {
		println!(
			"{}: {} would be formatted, {} unchanged, {} ignored, {} errors",
			"Summary".bright_cyan(),
			formatted_count.to_string().yellow(),
			unchanged_count,
			ignored_count,
			if error_count > 0 {
				error_count.to_string().red()
			} else {
				error_count.to_string().green()
			}
		);
	} else {
		println!(
			"{}: {} formatted, {} unchanged, {} ignored, {} errors",
			"Summary".bright_cyan(),
			if formatted_count > 0 {
				formatted_count.to_string().green()
			} else {
				formatted_count.to_string().dimmed()
			},
			unchanged_count,
			ignored_count,
			if error_count > 0 {
				error_count.to_string().red()
			} else {
				error_count.to_string().dimmed()
			}
		);
	}

	if check && formatted_count > 0 {
		return Err(reinhardt_commands::CommandError::ExecutionError(
			"Some files are not properly formatted".to_string(),
		));
	}

	if error_count > 0 {
		return Err(reinhardt_commands::CommandError::ExecutionError(format!(
			"{} files had formatting errors",
			error_count
		)));
	}

	Ok(())
}
