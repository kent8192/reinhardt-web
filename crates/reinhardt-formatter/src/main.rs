#![warn(missing_docs)]

//! Command-line formatter for Reinhardt DSL macros.

mod format_engine;
mod formatter;
mod utils;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use colored::Colorize;
use zeroize::Zeroize;

type FormatterResult<T> = Result<T, String>;

#[derive(Parser)]
#[command(name = "reinhardt-formatter")]
#[command(about = "Format Reinhardt page!, form!, and head! DSL macros", long_about = None)]
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
	/// Format Rust code and page!/form!/head! macro DSL in source files
	///
	/// By default, formats page!/form!/head! DSL macros with Topiary, formats
	/// supported page! Rust expression islands with rustfmt, and then runs
	/// rustfmt for the surrounding Rust source.
	/// Use --with-rustfmt=false to skip only the final surrounding-source rustfmt pass.
	Fmt {
		/// Path to file or directory to format
		#[arg(value_name = "PATH")]
		path: PathBuf,

		/// Check if files are formatted without modifying them
		#[arg(long)]
		check: bool,

		/// Also run rustfmt for surrounding Rust source after DSL formatting
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

	/// Format all code: Reinhardt DSL macros via Topiary + Rust via rustfmt
	///
	/// This command formats page!/form!/head! macros first, including supported
	/// page! Rust expression islands, then runs `cargo fmt --all` on the root
	/// workspace.
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

		/// Backup any modified files before formatting
		#[arg(long)]
		backup: bool,
	},
}

fn main() {
	let cli = Cli::parse();
	let result = match cli.command {
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

	if let Err(error) = result {
		eprintln!("{} {}", "Error:".red(), error);
		std::process::exit(1);
	}
}

#[allow(clippy::too_many_arguments)]
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
) -> FormatterResult<()> {
	use format_engine::{FormatEngine, RustfmtOptions};
	use formatter::collect_rust_files;

	if let Some(ref cp) = config_path {
		validate_config_path(cp).map_err(|e| format!("Invalid config path: {e}"))?;
		check_file_size(cp, MAX_CONFIG_FILE_SIZE)?;
	}

	let files = collect_rust_files(&path).map_err(|e| {
		format!(
			"Failed to collect files in {}: {}",
			display_path(&path),
			sanitize_error(&e)
		)
	})?;

	if files.is_empty() {
		if verbosity > 0 {
			println!("No Rust files found in {}", display_path(&path));
		}
		return Ok(());
	}

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
		println!("Using rustfmt config: {}", display_path(p));
	}

	let formatter = FormatEngine::with_rustfmt_options(options.clone());
	let mut formatted_count = 0;
	let mut unchanged_count = 0;
	let mut ignored_count = 0;
	let mut error_count = 0;
	let total_files = files.len();

	for (index, file_path) in files.iter().enumerate() {
		let progress = format!("[{}/{}]", index + 1, total_files);

		if let Err(e) = check_file_size(file_path, MAX_SOURCE_FILE_SIZE) {
			eprintln!(
				"{} {} {}: {}",
				progress.bright_blue(),
				"Skipped:".yellow(),
				display_path(file_path),
				e
			);
			error_count += 1;
			continue;
		}

		let original_content = std::fs::read_to_string(file_path).map_err(|e| {
			format!(
				"Failed to read {}: {}",
				mask_path(file_path),
				sanitize_error(&e.to_string())
			)
		})?;

		let ignore_all = formatter.has_ignore_all_marker(&original_content);
		if ignore_all && !with_rustfmt {
			ignored_count += 1;
			if verbosity > 0 {
				println!(
					"{} {} {} (reinhardt-fmt: ignore-all)",
					progress.bright_blue(),
					"Ignored:".yellow(),
					display_path(file_path)
				);
			}
			continue;
		}

		let dsl_result = if ignore_all {
			original_content.clone()
		} else {
			match formatter.format(&original_content) {
				Ok(result) => {
					if let Some(reason) = &result.skipped {
						if !with_rustfmt {
							ignored_count += 1;
							println!(
								"{} {} {} ({})",
								progress.bright_blue(),
								"Ignored:".yellow(),
								display_path(file_path),
								reason
							);
							continue;
						}
						original_content.clone()
					} else {
						result.content
					}
				}
				Err(e) => {
					eprintln!(
						"{} {} {}: {}",
						progress.bright_blue(),
						"Error".red(),
						display_path(file_path),
						sanitize_error(&e.to_string())
					);
					error_count += 1;
					continue;
				}
			}
		};

		let final_result = if with_rustfmt {
			match run_rustfmt(&dsl_result, &options) {
				Ok(output) => output,
				Err(e) => {
					eprintln!(
						"{} {} {}: rustfmt failed: {}",
						progress.bright_blue(),
						"Error".red(),
						display_path(file_path),
						sanitize_error(&e)
					);
					error_count += 1;
					continue;
				}
			}
		} else {
			dsl_result
		};

		if final_result != original_content {
			if check {
				println!("{} Would format: {}", progress, display_path(file_path));
				formatted_count += 1;
			} else {
				let mut backup_guard = None;
				if backup {
					let backup_path = create_temp_backup_path(file_path);
					create_secure_backup(file_path, &backup_path)
						.map_err(|e| format!("Failed to backup {}: {e}", mask_path(file_path)))?;
					backup_guard = Some(utils::BackupGuard::new(backup_path));
				}

				utils::atomic_write(file_path, &final_result).map_err(|e| {
					format!(
						"Failed to write {}: {}",
						mask_path(file_path),
						sanitize_error(&e.to_string())
					)
				})?;

				if let Some(ref mut guard) = backup_guard {
					guard.commit();
				}
				println!(
					"{} {} {}",
					progress.bright_blue(),
					"Formatted:".green(),
					display_path(file_path)
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
					display_path(file_path)
				);
			}
		}
	}

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
		return Err("Some files are not properly formatted".to_string());
	}

	if error_count > 0 {
		return Err(format!("{error_count} files had formatting errors"));
	}

	Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_fmt_all(
	check: bool,
	config_path: Option<PathBuf>,
	edition: Option<String>,
	style_edition: Option<String>,
	config: Option<String>,
	color: String,
	backup: bool,
	verbosity: u8,
) -> FormatterResult<()> {
	use format_engine::FormatEngine;
	use formatter::{collect_rust_files, nested_workspace_roots};
	use std::collections::HashMap;
	use std::process::{Command, Stdio};

	if let Some(ref cp) = config_path {
		validate_config_path(cp).map_err(|e| format!("Invalid config path: {e}"))?;
		check_file_size(cp, MAX_CONFIG_FILE_SIZE)?;
	}

	let project_root = find_project_root()
		.ok_or_else(|| "Could not find project root (no Cargo.toml found)".to_string())?;

	if verbosity > 0 {
		println!("Project root: {}", display_path(&project_root));
	}

	let files = collect_rust_files(&project_root)
		.map_err(|e| format!("Failed to collect files: {}", sanitize_error(&e)))?;

	let nested_roots = nested_workspace_roots(&project_root);
	let files: Vec<PathBuf> = files
		.into_iter()
		.filter(|file| !nested_roots.iter().any(|root| file.starts_with(root)))
		.collect();

	if files.is_empty() {
		if verbosity > 0 {
			println!("No Rust files found in {}", display_path(&project_root));
		}
		return Ok(());
	}

	let resolved_config_path = config_path
		.clone()
		.or_else(|| find_rustfmt_config(&project_root));
	let options = format_engine::RustfmtOptions {
		config_path: resolved_config_path.clone(),
		edition: edition.clone(),
		style_edition: style_edition.clone(),
		config: config.clone(),
		color: Some(color.clone()),
	};

	if verbosity > 0
		&& let Some(ref p) = resolved_config_path
	{
		println!("Using rustfmt config: {}", display_path(p));
	}

	let formatter = FormatEngine::with_rustfmt_options(options);
	let lock_path = project_root.join(".reinhardt-fmt.lock");
	let _lock_file = acquire_format_lock(&lock_path).map_err(|e| {
		format!("Failed to acquire format lock: {e}. Another format operation may be in progress.")
	})?;

	let mut original_contents: HashMap<PathBuf, String> = HashMap::new();
	let total_files = files.len();
	let mut dsl_file_count = 0;
	let mut error_count = 0;

	if verbosity > 0 {
		println!(
			"{} Phase 1: Formatting Reinhardt DSL macros...",
			"[Step 1/2]".bright_blue()
		);
	}

	for file_path in &files {
		if let Err(e) = check_file_size(file_path, MAX_SOURCE_FILE_SIZE) {
			eprintln!("{} Skipping oversized file: {}", "Warning:".yellow(), e);
			error_count += 1;
			continue;
		}

		let original_content = std::fs::read_to_string(file_path).map_err(|e| {
			format!(
				"Failed to read {}: {}",
				mask_path(file_path),
				sanitize_error(&e.to_string())
			)
		})?;

		original_contents.insert(file_path.clone(), original_content.clone());

		let format_result = match formatter.format(&original_content) {
			Ok(result) => result,
			Err(e) => {
				eprintln!(
					"{} DSL format failed for {}: {}",
					"Error:".red(),
					display_path(file_path),
					sanitize_error(&e.to_string())
				);
				error_count += 1;
				continue;
			}
		};

		if !format_result.contains_dsl_macro || format_result.skipped.is_some() {
			continue;
		}

		if format_result.content != original_content {
			utils::atomic_write(file_path, &format_result.content).map_err(|e| {
				format!(
					"Failed to write DSL-formatted content to {}: {}",
					mask_path(file_path),
					sanitize_error(&e.to_string())
				)
			})?;
			dsl_file_count += 1;
		}
	}

	if verbosity > 0 {
		println!(
			"{} Phase 2: Running cargo fmt --all...",
			"[Step 2/2]".bright_blue()
		);
	}

	let mut cmd = Command::new("cargo");
	cmd.arg("fmt").arg("--all");
	cmd.current_dir(&project_root);
	cmd.stdout(Stdio::inherit());
	cmd.stderr(Stdio::inherit());

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

	if verbosity > 1 {
		println!("  Running: cargo fmt --all");
	}

	let output = match cmd.output() {
		Ok(output) => output,
		Err(e) => {
			eprintln!(
				"{} cargo fmt failed: {}",
				"Error:".red(),
				sanitize_error(&e.to_string())
			);
			let rollback_paths: Vec<PathBuf> = original_contents.keys().cloned().collect();
			let rollback_errors = utils::rollback_files(&rollback_paths, &original_contents);
			utils::report_rollback_errors(&rollback_errors);
			return Err("cargo fmt failed to execute".to_string());
		}
	};

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		let sanitized_stderr = sanitize_error(&stderr);
		eprintln!(
			"{} cargo fmt exited with error: {}",
			"Error:".red(),
			sanitized_stderr
		);
		let rollback_paths: Vec<PathBuf> = original_contents.keys().cloned().collect();
		let rollback_errors = utils::rollback_files(&rollback_paths, &original_contents);
		utils::report_rollback_errors(&rollback_errors);
		return Err(format!("cargo fmt exited with error: {sanitized_stderr}"));
	}

	let mut formatted_count = 0;
	let mut unchanged_count = 0;

	for (index, file_path) in files.iter().enumerate() {
		let progress = format!("[{}/{}]", index + 1, total_files);
		let Some(original_content) = original_contents.get(file_path) else {
			continue;
		};
		let Ok(current_content) = std::fs::read_to_string(file_path) else {
			continue;
		};

		if &current_content != original_content {
			if check {
				println!("{} Would format: {}", progress, display_path(file_path));
				if let Err(e) = std::fs::write(file_path, original_content) {
					eprintln!(
						"Warning: failed to restore {} in check mode: {}",
						file_path.display(),
						e
					);
				}
			} else if verbosity > 0 {
				println!(
					"{} {} {}",
					progress.bright_blue(),
					"Formatted:".green(),
					display_path(file_path)
				);
			}
			formatted_count += 1;

			if backup && !check {
				let backup_path = create_temp_backup_path(file_path);
				match create_secure_backup(file_path, &backup_path) {
					Ok(()) => {
						let mut guard = utils::BackupGuard::new(backup_path);
						guard.commit();
					}
					Err(e) => {
						eprintln!(
							"Warning: failed to create backup for {}: {}",
							mask_path(file_path),
							e
						);
					}
				}
			}
		} else {
			unchanged_count += 1;
			if verbosity > 0 {
				println!(
					"{} {} {}",
					progress.bright_blue(),
					"Unchanged:".dimmed(),
					display_path(file_path)
				);
			}
		}
	}

	if check {
		let all_paths: Vec<PathBuf> = original_contents.keys().cloned().collect();
		let rollback_errors = utils::rollback_files(&all_paths, &original_contents);
		utils::report_rollback_errors(&rollback_errors);
	}

	secure_clear_hashmap(&mut original_contents);

	println!();
	if check {
		println!(
			"{}: {} would be formatted, {} unchanged, {} DSL files preformatted, {} errors",
			"Summary".bright_cyan(),
			formatted_count.to_string().yellow(),
			unchanged_count,
			dsl_file_count,
			if error_count > 0 {
				error_count.to_string().red()
			} else {
				error_count.to_string().green()
			}
		);
	} else {
		println!(
			"{}: {} formatted, {} unchanged, {} DSL files preformatted, {} errors",
			"Summary".bright_cyan(),
			if formatted_count > 0 {
				formatted_count.to_string().green()
			} else {
				formatted_count.to_string().dimmed()
			},
			unchanged_count,
			dsl_file_count,
			if error_count > 0 {
				error_count.to_string().red()
			} else {
				error_count.to_string().dimmed()
			}
		);
	}

	if check && formatted_count > 0 {
		return Err("Some files are not properly formatted".to_string());
	}
	if error_count > 0 {
		return Err(format!("{error_count} files had formatting errors"));
	}

	Ok(())
}

const MAX_PROJECT_ROOT_DEPTH: usize = 10;

fn find_project_root() -> Option<PathBuf> {
	let current_dir = std::env::current_dir().ok()?;
	let mut current = current_dir.as_path();

	for _ in 0..MAX_PROJECT_ROOT_DEPTH {
		if std::fs::metadata(current.join("Cargo.toml")).is_ok() {
			return Some(current.to_path_buf());
		}
		current = current.parent()?;
	}
	None
}

fn find_rustfmt_config(start_path: &Path) -> Option<PathBuf> {
	let mut current = if start_path.is_file() {
		start_path.parent()
	} else {
		Some(start_path)
	}?;

	for _ in 0..MAX_PROJECT_ROOT_DEPTH {
		let config = current.join("rustfmt.toml");
		if std::fs::metadata(&config).is_ok() {
			return Some(config);
		}
		let hidden_config = current.join(".rustfmt.toml");
		if std::fs::metadata(&hidden_config).is_ok() {
			return Some(hidden_config);
		}
		current = current.parent()?;
	}
	None
}

fn run_rustfmt(content: &str, options: &format_engine::RustfmtOptions) -> Result<String, String> {
	use std::io::Write;
	use std::process::{Command, Stdio};

	let mut cmd = Command::new("rustfmt");
	options.apply_to_command(&mut cmd);
	if options.config_path.is_none() && options.edition.is_none() {
		cmd.arg("--edition=2024");
	}

	let mut child = cmd
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| format!("Failed to spawn rustfmt: {e}"))?;

	if let Some(mut stdin) = child.stdin.take() {
		stdin
			.write_all(content.as_bytes())
			.map_err(|e| format!("Failed to write to rustfmt stdin: {e}"))?;
	}

	let output = child
		.wait_with_output()
		.map_err(|e| format!("Failed to wait for rustfmt: {e}"))?;

	if output.status.success() {
		String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 from rustfmt: {e}"))
	} else {
		let stderr = String::from_utf8_lossy(&output.stderr);
		Err(format!("rustfmt failed: {stderr}"))
	}
}

#[cfg(unix)]
fn create_secure_backup(source: &Path, backup_path: &Path) -> Result<(), std::io::Error> {
	use std::fs::OpenOptions;
	use std::io::Read;
	use std::os::unix::fs::OpenOptionsExt;

	let mut content = Vec::new();
	let mut file = std::fs::File::open(source)?;
	file.read_to_end(&mut content)?;

	let mut backup_file = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.mode(0o600)
		.open(backup_path)?;

	std::io::copy(&mut content.as_slice(), &mut backup_file)?;
	content.zeroize();
	Ok(())
}

#[cfg(not(unix))]
fn create_secure_backup(source: &Path, backup_path: &Path) -> Result<(), std::io::Error> {
	use std::io::Read;

	let mut content = Vec::new();
	let mut file = std::fs::File::open(source)?;
	file.read_to_end(&mut content)?;
	std::fs::write(backup_path, &content)?;
	content.zeroize();
	Ok(())
}

fn create_temp_backup_path(source: &Path) -> PathBuf {
	let file_name = source
		.file_name()
		.unwrap_or_else(|| std::ffi::OsStr::new("unknown"));
	let backup_name = format!("reinhardt-fmt-{}.bak", file_name.to_string_lossy());
	std::env::temp_dir().join(backup_name)
}

fn mask_path(path: &Path) -> String {
	path.file_name()
		.map(|name| format!("<...>/{}", name.to_string_lossy()))
		.unwrap_or_else(|| "<file>".to_string())
}

fn display_path(path: &Path) -> String {
	if let Ok(cwd) = std::env::current_dir()
		&& let Ok(relative) = path.strip_prefix(&cwd)
	{
		return relative.display().to_string();
	}
	mask_path(path)
}

fn sanitize_error(error: &str) -> String {
	use std::sync::LazyLock;

	static PATH_RE: LazyLock<regex::Regex> =
		LazyLock::new(|| regex::Regex::new(r"(/[a-zA-Z0-9._-]+){3,}").unwrap());
	static DB_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
		regex::Regex::new(r"(?i)(postgres|mysql|sqlite|mongodb|redis)://[^\s]+").unwrap()
	});
	static TOKEN_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
		regex::Regex::new(
			r"(?i)(api[_\-]?key|token|secret|password|auth)[=:]\s*['\x22]?[a-zA-Z0-9+/=_\-]{8,}",
		)
		.unwrap()
	});

	let sanitized = PATH_RE.replace_all(error, |caps: &regex::Captures| {
		let matched_path = Path::new(caps.get(0).unwrap().as_str());
		mask_path(matched_path)
	});
	let sanitized = DB_RE.replace_all(&sanitized, "[REDACTED_DATABASE_URL]");
	let sanitized = TOKEN_RE.replace_all(&sanitized, "[REDACTED_CREDENTIAL]");
	sanitized.to_string()
}

fn secure_clear_hashmap(map: &mut std::collections::HashMap<PathBuf, String>) {
	for (_, value) in map.iter_mut() {
		value.zeroize();
	}
	map.clear();
}

fn validate_config_path(path: &Path) -> Result<(), String> {
	let path_str = path.to_string_lossy();
	if path_str.contains("..") {
		return Err(format!(
			"Config path contains path traversal sequence: {}",
			mask_path(path)
		));
	}

	#[cfg(unix)]
	if path_str.starts_with("/dev/")
		|| path_str.starts_with("/proc/")
		|| path_str.starts_with("/sys/")
	{
		return Err(format!(
			"Config path refers to a special device: {}",
			mask_path(path)
		));
	}

	let symlink_meta = std::fs::symlink_metadata(path)
		.map_err(|e| format!("Config path is not accessible: {} ({})", mask_path(path), e))?;
	if symlink_meta.file_type().is_symlink() {
		return Err(format!(
			"Config path is a symlink, which is not allowed: {}",
			mask_path(path)
		));
	}
	if !symlink_meta.is_file() {
		return Err(format!(
			"Config path is not a regular file: {}",
			mask_path(path)
		));
	}
	Ok(())
}

const MAX_CONFIG_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_SOURCE_FILE_SIZE: u64 = 5 * 1024 * 1024;

fn check_file_size(path: &Path, max_size: u64) -> Result<(), String> {
	match std::fs::metadata(path) {
		Ok(metadata) => {
			if metadata.len() > max_size {
				Err(format!(
					"File {} exceeds maximum allowed size ({} bytes, limit {} bytes)",
					mask_path(path),
					metadata.len(),
					max_size
				))
			} else {
				Ok(())
			}
		}
		Err(e) => Err(format!(
			"Failed to check file size for {}: {}",
			mask_path(path),
			e
		)),
	}
}

fn acquire_format_lock(lock_path: &Path) -> Result<FormatLockGuard, std::io::Error> {
	use std::fs::OpenOptions;
	use std::io::Write;

	let mut file = OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(lock_path)?;
	let _ = writeln!(file, "{}", std::process::id());

	Ok(FormatLockGuard {
		path: lock_path.to_path_buf(),
	})
}

struct FormatLockGuard {
	path: PathBuf,
}

impl Drop for FormatLockGuard {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}
