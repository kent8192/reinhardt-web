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

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use colored::Colorize;
use reinhardt_commands::{
	BaseCommand, CommandContext, CommandResult, PluginDisableCommand, PluginEnableCommand,
	PluginInfoCommand, PluginInstallCommand, PluginListCommand, PluginRemoveCommand,
	PluginSearchCommand, PluginUpdateCommand, StartAppCommand, StartProjectCommand,
};
use std::process;
use zeroize::Zeroize;

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

	/// Format Rust code and page! macro DSL in source files
	///
	/// By default, runs both rustfmt (protecting page! macros) and page! DSL formatting.
	/// Use --with-rustfmt=false to only format page! macro DSL.
	Fmt {
		/// Path to file or directory to format
		#[arg(value_name = "PATH")]
		path: PathBuf,

		/// Check if files are formatted without modifying them
		#[arg(long)]
		check: bool,

		/// Also run rustfmt (with page! macro protection)
		#[arg(long, default_value = "true", action = clap::ArgAction::Set)]
		with_rustfmt: bool,

		/// Path to rustfmt.toml configuration file
		#[arg(long, value_name = "PATH")]
		config_path: Option<PathBuf>,

		/// Rust edition to use (overrides rustfmt.toml)
		#[arg(long, value_name = "EDITION")]
		edition: Option<String>,

		/// Style edition to use (overrides rustfmt.toml)
		#[arg(long, value_name = "EDITION")]
		style_edition: Option<String>,

		/// Set options from command line (overrides rustfmt.toml)
		/// Format: key1=val1,key2=val2
		#[arg(long, value_name = "OPTIONS")]
		config: Option<String>,

		/// Use colored output
		#[arg(long, value_name = "WHEN", default_value = "auto")]
		color: String,

		/// Backup any modified files
		#[arg(long)]
		backup: bool,
	},

	/// Format all code: Rust (via rustfmt) + page! DSL (via reinhardt-fmt)
	///
	/// This command protects page! macros from rustfmt, runs rustfmt on the
	/// surrounding Rust code, restores the macros, and then formats them with
	/// the page! DSL formatter.
	///
	/// Formats all Rust files in the project (searches for Cargo.toml to find project root).
	FmtAll {
		/// Check if files are formatted without modifying them
		#[arg(long)]
		check: bool,

		/// Path to rustfmt.toml configuration file
		/// If not specified, searches upward from current directory for rustfmt.toml
		#[arg(long, value_name = "PATH")]
		config_path: Option<PathBuf>,

		/// Rust edition to use (overrides rustfmt.toml)
		#[arg(long, value_name = "EDITION")]
		edition: Option<String>,

		/// Style edition to use (overrides rustfmt.toml)
		#[arg(long, value_name = "EDITION")]
		style_edition: Option<String>,

		/// Set options from command line (overrides rustfmt.toml)
		/// Format: key1=val1,key2=val2
		#[arg(long, value_name = "OPTIONS")]
		config: Option<String>,

		/// Use colored output
		#[arg(long, value_name = "WHEN", default_value = "auto")]
		color: String,

		/// Backup any modified files
		#[arg(long)]
		backup: bool,
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
		Commands::Fmt {
			path,
			check,
			with_rustfmt,
			config_path,
			edition,
			style_edition,
			config,
			color,
			backup,
		} => run_fmt(
			path,
			check,
			with_rustfmt,
			config_path,
			edition,
			style_edition,
			config,
			color,
			backup,
			cli.verbosity,
		),
		Commands::FmtAll {
			check,
			config_path,
			edition,
			style_edition,
			config,
			color,
			backup,
		} => run_fmt_all(
			check,
			config_path,
			edition,
			style_edition,
			config,
			color,
			backup,
			cli.verbosity,
		),
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

#[allow(clippy::too_many_arguments)] // CLI command handler with many options
fn run_fmt(
	path: PathBuf,
	check: bool,
	with_rustfmt: bool,
	config_path: Option<PathBuf>,
	edition: Option<String>,
	style_edition: Option<String>,
	config: Option<String>,
	color: String,
	backup: bool,
	verbosity: u8,
) -> CommandResult<()> {
	use ast_formatter::{AstPageFormatter, RustfmtOptions};
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

	// Resolve config path
	let resolved_config_path = config_path.or_else(|| find_rustfmt_config(&path));

	let options = RustfmtOptions {
		config_path: resolved_config_path.clone(),
		edition,
		style_edition,
		config,
		color: Some(color),
	};

	if verbosity > 0
		&& let Some(ref p) = resolved_config_path
	{
		println!("Using rustfmt config: {}", p.display());
	}

	let formatter = if let Some(ref config) = resolved_config_path {
		AstPageFormatter::with_config(config.clone())
	} else {
		AstPageFormatter::new()
	};
	let mut formatted_count = 0;
	let mut unchanged_count = 0;
	let mut ignored_count = 0;
	let mut error_count = 0;

	let total_files = files.len();

	for (index, file_path) in files.iter().enumerate() {
		let progress = format!("[{}/{}]", index + 1, total_files);
		let original_content = std::fs::read_to_string(file_path).map_err(|e| {
			reinhardt_commands::CommandError::ExecutionError(format!(
				"Failed to read {}: {}",
				file_path.display(),
				e
			))
		})?;

		// Check for file-wide ignore marker BEFORE any processing
		if formatter.has_ignore_all_marker(&original_content) {
			ignored_count += 1;
			if verbosity > 0 {
				println!(
					"{} {} {} (reinhardt-fmt: ignore-all)",
					progress.bright_blue(),
					"Ignored:".yellow(),
					file_path.display()
				);
			}
			continue;
		}

		// Process the file based on with_rustfmt option
		let final_result = if with_rustfmt {
			// Pipeline: protect -> rustfmt -> restore -> page! format
			// Step 1: Protect page! macros
			let protect_result = formatter.protect_page_macros(&original_content);

			// Step 2: Run rustfmt on protected content
			let rustfmt_output = match run_rustfmt(&protect_result.protected_content, &options) {
				Ok(output) => output,
				Err(e) => {
					eprintln!(
						"{} {} {}: rustfmt failed: {}",
						progress.bright_blue(),
						"Error".red(),
						file_path.display(),
						e
					);
					error_count += 1;
					continue;
				}
			};

			// Step 3: Restore page! macros
			let restored =
				AstPageFormatter::restore_page_macros(&rustfmt_output, &protect_result.backups);

			// Step 4: Format page! macros with reinhardt-fmt
			match formatter.format(&restored) {
				Ok(result) => result.content,
				Err(e) => {
					eprintln!(
						"{} {} {}: page! format failed: {}",
						progress.bright_blue(),
						"Error".red(),
						file_path.display(),
						e
					);
					error_count += 1;
					continue;
				}
			}
		} else {
			// Original behavior: page! DSL only
			match formatter.format(&original_content) {
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
					result.content
				}
				Err(e) => {
					eprintln!(
						"{} {} {}: {}",
						progress.bright_blue(),
						"Error".red(),
						file_path.display(),
						e
					);
					error_count += 1;
					continue;
				}
			}
		};

		// Compare with original
		if final_result != original_content {
			if check {
				// Check mode: report unformatted files
				println!("{} Would format: {}", progress, file_path.display());
				formatted_count += 1;
			} else {
				// Backup if requested
				if backup {
					let backup_path = file_path.with_extension("rs.bak");
					create_secure_backup(file_path, &backup_path).map_err(|e| {
						reinhardt_commands::CommandError::ExecutionError(format!(
							"Failed to backup {}: {}",
							mask_path(file_path),
							e
						))
					})?;
				}

				// Format mode: write changes
				std::fs::write(file_path, &final_result).map_err(|e| {
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
			if verbosity > 0 {
				println!(
					"{} {} {}",
					progress.bright_blue(),
					"Unchanged:".dimmed(),
					file_path.display()
				);
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

/// Format all code: Rust (via rustfmt) + page! DSL (via reinhardt-fmt)
///
/// Pipeline using `cargo fmt --all`:
/// 1. Protect: page!(...)  → __reinhardt_placeholder__!(/*n*/) (write to disk)
/// 2. cargo fmt --all: Format Rust code (placeholders are not touched)
/// 3. Restore: __reinhardt_placeholder__!(/*n*/) → page!(...)
/// 4. reinhardt-fmt: Format page! macro contents
#[allow(clippy::too_many_arguments)] // CLI command handler with many options
fn run_fmt_all(
	check: bool,
	config_path: Option<PathBuf>,
	edition: Option<String>,
	style_edition: Option<String>,
	config: Option<String>,
	color: String,
	backup: bool,
	verbosity: u8,
) -> CommandResult<()> {
	use ast_formatter::AstPageFormatter;
	use formatter::collect_rust_files;
	use std::collections::HashMap;
	use std::process::{Command, Stdio};

	// Find project root
	let project_root = find_project_root().ok_or_else(|| {
		reinhardt_commands::CommandError::ExecutionError(
			"Could not find project root (no Cargo.toml found)".to_string(),
		)
	})?;

	if verbosity > 0 {
		println!("Project root: {}", project_root.display());
	}

	let files = collect_rust_files(&project_root).map_err(|e| {
		reinhardt_commands::CommandError::ExecutionError(format!("Failed to collect files: {}", e))
	})?;

	if files.is_empty() {
		if verbosity > 0 {
			println!("No Rust files found in {:?}", project_root);
		}
		return Ok(());
	}

	let formatter = AstPageFormatter::new();

	// Store original contents for comparison and rollback
	let mut original_contents: HashMap<PathBuf, String> = HashMap::new();
	// Store backup info for protected files
	let mut protected_files: Vec<(PathBuf, Vec<ast_formatter::PageMacroBackup>)> = Vec::new();

	let total_files = files.len();
	let mut page_macro_count = 0;

	// Phase 1: Protect page! macros and write to disk
	if verbosity > 0 {
		println!(
			"{} Phase 1: Protecting page! macros...",
			"[Step 1/3]".bright_blue()
		);
	}

	for file_path in &files {
		let original_content = std::fs::read_to_string(file_path).map_err(|e| {
			reinhardt_commands::CommandError::ExecutionError(format!(
				"Failed to read {}: {}",
				file_path.display(),
				e
			))
		})?;

		// Store original content for comparison
		original_contents.insert(file_path.clone(), original_content.clone());

		// Skip if ignore-all marker is present
		if formatter.has_ignore_all_marker(&original_content) {
			continue;
		}

		// Skip files without page! macros
		if !original_content.contains("page!(") {
			continue;
		}

		let protect_result = formatter.protect_page_macros(&original_content);

		// Only process if there are actual macros to protect
		if !protect_result.backups.is_empty() {
			page_macro_count += protect_result.backups.len();

			// Write protected content to disk (cargo fmt will format it)
			std::fs::write(file_path, &protect_result.protected_content).map_err(|e| {
				reinhardt_commands::CommandError::ExecutionError(format!(
					"Failed to write protected content to {}: {}",
					file_path.display(),
					e
				))
			})?;

			protected_files.push((file_path.clone(), protect_result.backups));
		}
	}

	if verbosity > 0 {
		println!(
			"  Protected {} page! macros in {} files",
			page_macro_count,
			protected_files.len()
		);
	}

	// Phase 2: Run cargo fmt --all
	if verbosity > 0 {
		println!(
			"{} Phase 2: Running cargo fmt --all...",
			"[Step 2/3]".bright_blue()
		);
	}

	let mut cmd = Command::new("cargo");
	cmd.arg("fmt").arg("--all");
	cmd.current_dir(&project_root);
	cmd.stdout(Stdio::inherit());
	cmd.stderr(Stdio::inherit());

	// Add rustfmt options via "--" separator
	let has_rustfmt_options = config_path.is_some()
		|| edition.is_some()
		|| style_edition.is_some()
		|| config.is_some()
		|| color != "auto";

	if has_rustfmt_options {
		cmd.arg("--");

		if let Some(ref path) = config_path {
			cmd.arg("--config-path").arg(path);
		}

		if let Some(ref ed) = edition {
			cmd.arg("--edition").arg(ed);
		}

		if let Some(ref se) = style_edition {
			cmd.arg("--style-edition").arg(se);
		}

		if let Some(ref cfg) = config {
			cmd.arg("--config").arg(cfg);
		}

		if color != "auto" {
			cmd.arg("--color").arg(&color);
		}
	}

	if verbosity > 0 {
		println!("  Command: {:?}", cmd);
	}

	let cargo_fmt_result = cmd.output();

	// If cargo fmt fails, restore original files
	if let Err(e) = &cargo_fmt_result {
		eprintln!("{} cargo fmt failed: {}", "Error:".red(), e);
		// Restore original files
		for (file_path, original_content) in &original_contents {
			let _ = std::fs::write(file_path, original_content);
		}
		return Err(reinhardt_commands::CommandError::ExecutionError(format!(
			"cargo fmt failed: {}",
			e
		)));
	}

	let output = cargo_fmt_result.unwrap();
	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		eprintln!("{} cargo fmt exited with error: {}", "Error:".red(), stderr);
		// Restore original files
		for (file_path, original_content) in &original_contents {
			let _ = std::fs::write(file_path, original_content);
		}
		return Err(reinhardt_commands::CommandError::ExecutionError(format!(
			"cargo fmt exited with error: {}",
			stderr
		)));
	}

	// Phase 3: Restore page! macros and format DSL
	if verbosity > 0 {
		println!(
			"{} Phase 3: Restoring and formatting page! macros...",
			"[Step 3/3]".bright_blue()
		);
	}

	let mut error_count = 0;

	for (file_path, backups) in &protected_files {
		// Read the cargo-fmt formatted content
		let formatted_content = match std::fs::read_to_string(file_path) {
			Ok(content) => content,
			Err(e) => {
				eprintln!(
					"{} Failed to read {}: {}",
					"Error:".red(),
					file_path.display(),
					e
				);
				error_count += 1;
				continue;
			}
		};

		// Restore page! macros
		let restored = AstPageFormatter::restore_page_macros(&formatted_content, backups);

		// Format page! macro DSL
		let final_result = match formatter.format(&restored) {
			Ok(result) => result.content,
			Err(e) => {
				eprintln!(
					"{} page! format failed for {}: {}",
					"Error:".red(),
					file_path.display(),
					e
				);
				error_count += 1;
				// Write restored content anyway (without DSL formatting)
				let _ = std::fs::write(file_path, &restored);
				continue;
			}
		};

		// Write final result
		if let Err(e) = std::fs::write(file_path, &final_result) {
			eprintln!(
				"{} Failed to write {}: {}",
				"Error:".red(),
				file_path.display(),
				e
			);
			error_count += 1;
		}
	}

	// Compare and count changes
	let mut formatted_count = 0;
	let mut unchanged_count = 0;

	for (index, file_path) in files.iter().enumerate() {
		let progress = format!("[{}/{}]", index + 1, total_files);

		let original_content = match original_contents.get(file_path) {
			Some(content) => content,
			None => continue,
		};

		let current_content = match std::fs::read_to_string(file_path) {
			Ok(content) => content,
			Err(_) => continue,
		};

		if &current_content != original_content {
			if check {
				println!("{} Would format: {}", progress, file_path.display());
				// Restore original content in check mode
				let _ = std::fs::write(file_path, original_content);
			} else if verbosity > 0 {
				println!(
					"{} {} {}",
					progress.bright_blue(),
					"Formatted:".green(),
					file_path.display()
				);
			}
			formatted_count += 1;

			// Create backup if requested
			if backup && !check {
				let backup_path = file_path.with_extension("rs.bak");
				// Write backup with secure content handling
				let mut content_copy = original_content.clone();
				let _ = std::fs::write(&backup_path, &content_copy);
				content_copy.zeroize();
			}
		} else {
			unchanged_count += 1;
			if verbosity > 0 {
				println!(
					"{} {} {}",
					progress.bright_blue(),
					"Unchanged:".dimmed(),
					file_path.display()
				);
			}
		}
	}

	// In check mode, restore all files to original state
	if check {
		for (file_path, original_content) in &original_contents {
			let _ = std::fs::write(file_path, original_content);
		}
	}

	// Securely clear original contents to prevent sensitive data lingering in memory
	secure_clear_hashmap(&mut original_contents);

	// Summary
	println!();
	if check {
		println!(
			"{}: {} would be formatted, {} unchanged, {} errors",
			"Summary".bright_cyan(),
			formatted_count.to_string().yellow(),
			unchanged_count,
			if error_count > 0 {
				error_count.to_string().red()
			} else {
				error_count.to_string().green()
			}
		);
	} else {
		println!(
			"{}: {} formatted, {} unchanged, {} errors",
			"Summary".bright_cyan(),
			if formatted_count > 0 {
				formatted_count.to_string().green()
			} else {
				formatted_count.to_string().dimmed()
			},
			unchanged_count,
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

/// Find the project root by searching upward for Cargo.toml.
fn find_project_root() -> Option<PathBuf> {
	let current_dir = std::env::current_dir().ok()?;
	let mut current = current_dir.as_path();

	loop {
		if current.join("Cargo.toml").exists() {
			return Some(current.to_path_buf());
		}
		current = current.parent()?;
	}
}

/// Find rustfmt.toml by searching upward from a path.
fn find_rustfmt_config(start_path: &Path) -> Option<PathBuf> {
	let mut current = if start_path.is_file() {
		start_path.parent()
	} else {
		Some(start_path)
	}?;

	loop {
		let config = current.join("rustfmt.toml");
		if config.exists() {
			return Some(config);
		}
		let hidden_config = current.join(".rustfmt.toml");
		if hidden_config.exists() {
			return Some(hidden_config);
		}
		if current.join("Cargo.toml").exists() {
			break;
		}
		current = current.parent()?;
	}
	None
}

/// Run rustfmt on content and return formatted output.
fn run_rustfmt(content: &str, options: &ast_formatter::RustfmtOptions) -> Result<String, String> {
	use std::io::Write;
	use std::process::{Command, Stdio};

	let mut cmd = Command::new("rustfmt");
	options.apply_to_command(&mut cmd);

	// Fallback to default edition if no config is specified
	if options.config_path.is_none() && options.edition.is_none() {
		cmd.arg("--edition=2024");
	}

	let mut child = cmd
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| format!("Failed to spawn rustfmt: {}", e))?;

	if let Some(mut stdin) = child.stdin.take() {
		stdin
			.write_all(content.as_bytes())
			.map_err(|e| format!("Failed to write to rustfmt stdin: {}", e))?;
	}

	let output = child
		.wait_with_output()
		.map_err(|e| format!("Failed to wait for rustfmt: {}", e))?;

	if output.status.success() {
		String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 from rustfmt: {}", e))
	} else {
		let stderr = String::from_utf8_lossy(&output.stderr);
		Err(format!("rustfmt failed: {}", stderr))
	}
}

/// Create a backup file with restrictive permissions (0600).
///
/// # Security
///
/// Sets file permissions to 0600 (read/write for owner only) to prevent
/// unauthorized access to backup files which may contain sensitive code.
#[cfg(unix)]
fn create_secure_backup(source: &Path, backup_path: &Path) -> Result<(), std::io::Error> {
	use std::fs::OpenOptions;
	use std::io::Read;
	use std::os::unix::fs::OpenOptionsExt;

	// Read source content
	let mut content = Vec::new();
	let mut file = std::fs::File::open(source)?;
	file.read_to_end(&mut content)?;

	// Create backup with restrictive permissions
	let mut backup_file = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.mode(0o600) // Owner read/write only
		.open(backup_path)?;

	std::io::copy(&mut content.as_slice(), &mut backup_file)?;

	// Zeroize the content buffer to prevent sensitive data lingering in memory
	content.zeroize();

	Ok(())
}

/// Create a backup file with restrictive permissions (0600).
///
/// # Security
///
/// On non-Unix systems, this creates the backup file normally with default permissions.
/// A warning is logged recommending Unix for production deployments.
#[cfg(not(unix))]
fn create_secure_backup(source: &Path, backup_path: &Path) -> Result<(), std::io::Error> {
	use std::io::Read;

	// Read source content
	let mut content = Vec::new();
	let mut file = std::fs::File::open(source)?;
	file.read_to_end(&mut content)?;

	// Create backup with default permissions (Windows ACLs handle security)
	std::fs::write(backup_path, &content)?;

	// Zeroize the content buffer
	content.zeroize();

	Ok(())
}

/// Mask sensitive file path in error messages.
///
/// Returns a masked version of the path that only shows the filename,
/// preventing full path disclosure in error output.
fn mask_path(path: &Path) -> String {
	path.file_name()
		.map(|name| format!("<...>/{}", name.to_string_lossy()))
		.unwrap_or_else(|| "<file>".to_string())
}

/// Securely clear a HashMap containing sensitive string data.
///
/// # Security
///
/// Uses zeroize to overwrite string values in memory before dropping,
/// preventing sensitive data from remaining in memory.
fn secure_clear_hashmap(map: &mut std::collections::HashMap<PathBuf, String>) {
	for (_, value) in map.iter_mut() {
		value.zeroize();
	}
	map.clear();
}
