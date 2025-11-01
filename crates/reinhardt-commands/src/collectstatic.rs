//! Collect static files command
//!
//! Django-style static file collection for production deployment

use crate::CommandResult;
use crate::{BaseCommand, CommandContext};
use async_trait::async_trait;
use reinhardt_static::{StaticFilesConfig, StaticFilesFinder};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CollectStaticOptions {
	pub clear: bool,
	pub no_input: bool,
	pub dry_run: bool,
	pub interactive: bool,
	pub verbosity: u8,
	pub link: bool,
	pub ignore_patterns: Vec<String>,
}

impl Default for CollectStaticOptions {
	fn default() -> Self {
		Self {
			clear: false,
			no_input: false,
			dry_run: false,
			interactive: true,
			verbosity: 1,
			link: false,
			ignore_patterns: Vec::new(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct CollectStaticStats {
	pub copied: usize,
	pub skipped: usize,
	pub deleted: usize,
	pub unmodified: usize,
}

impl CollectStaticStats {
	pub fn new() -> Self {
		Self {
			copied: 0,
			skipped: 0,
			deleted: 0,
			unmodified: 0,
		}
	}
}

impl Default for CollectStaticStats {
	fn default() -> Self {
		Self::new()
	}
}

pub struct CollectStaticCommand {
	config: StaticFilesConfig,
	options: CollectStaticOptions,
}

impl CollectStaticCommand {
	pub fn new(config: StaticFilesConfig, options: CollectStaticOptions) -> Self {
		Self { config, options }
	}

	/// Execute the collectstatic command
	pub fn execute(&mut self) -> Result<CollectStaticStats, io::Error> {
		let mut stats = CollectStaticStats::new();

		// Validate configuration
		self.validate_config()?;

		// Clear destination if requested
		if self.options.clear {
			self.clear_destination(&mut stats)?;
		}

		// Create destination directory if it doesn't exist
		if !self.options.dry_run {
			fs::create_dir_all(&self.config.static_root)?;
		}

		// Collect files from all source directories
		let finder = StaticFilesFinder::new(self.config.staticfiles_dirs.clone());
		let all_files = finder.find_all();

		if self.options.verbosity > 0 {
			println!("Found {} static files", all_files.len());
		}

		// Use a HashSet to track unique file paths and ensure we process each file only once
		// Later sources override earlier ones due to the order of find_all()
		let mut processed_files = std::collections::HashSet::new();
		let files_to_process: Vec<String> = all_files.into_iter().rev().collect();

		// Process each file (reversed so later sources are processed first)
		for file_path in &files_to_process {
			// Skip if already processed (handles duplicates from multiple source dirs)
			if !processed_files.insert(file_path.clone()) {
				continue;
			}

			if self.should_ignore(file_path) {
				if self.options.verbosity > 1 {
					println!("Ignoring: {}", file_path);
				}
				stats.skipped += 1;
				continue;
			}

			match self.copy_file(file_path)? {
				CopyResult::Copied => stats.copied += 1,
				CopyResult::Skipped => stats.skipped += 1,
				CopyResult::Unmodified => stats.unmodified += 1,
			}
		}

		// Print summary
		if self.options.verbosity > 0 {
			self.print_summary(&stats);
		}

		Ok(stats)
	}

	fn validate_config(&self) -> Result<(), io::Error> {
		if self.config.static_root.as_os_str().is_empty() {
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"STATIC_ROOT is not configured",
			));
		}

		Ok(())
	}

	fn clear_destination(&self, stats: &mut CollectStaticStats) -> Result<(), io::Error> {
		if !self.config.static_root.exists() {
			return Ok(());
		}

		if self.options.verbosity > 0 {
			println!(
				"Clearing existing files from {}",
				self.config.static_root.display()
			);
		}

		if !self.options.dry_run {
			for entry in fs::read_dir(&self.config.static_root)? {
				let entry = entry?;
				let path = entry.path();

				if path.is_file() {
					fs::remove_file(&path)?;
					stats.deleted += 1;
				} else if path.is_dir() {
					fs::remove_dir_all(&path)?;
					stats.deleted += 1;
				}
			}
		}

		Ok(())
	}

	fn find_in_reverse(&self, path: &str) -> Result<std::path::PathBuf, io::Error> {
		// Search directories in reverse order to prioritize later sources
		for dir in self.config.staticfiles_dirs.iter().rev() {
			let file_path = dir.join(path);
			if file_path.exists() {
				return Ok(file_path);
			}
		}
		Err(io::Error::new(
			io::ErrorKind::NotFound,
			format!("File not found in any directory: {}", path),
		))
	}

	fn should_ignore(&self, file_path: &str) -> bool {
		// Check if file is hidden (starts with dot)
		if let Some(file_name) = std::path::Path::new(file_path).file_name()
			&& let Some(name_str) = file_name.to_str()
				&& name_str.starts_with('.') {
					return true;
				}

		// Check ignore patterns
		for pattern in &self.options.ignore_patterns {
			// Simple wildcard matching: "*.ext" matches any file ending with .ext
			if pattern.starts_with("*.") {
				let ext = &pattern[1..]; // Remove the *
				if file_path.ends_with(ext) {
					return true;
				}
			} else if file_path.contains(pattern) {
				return true;
			}
		}
		false
	}

	fn copy_file(&self, relative_path: &str) -> Result<CopyResult, io::Error> {
		// Find source file - search directories in reverse order to prioritize later sources
		let source_path = self.find_in_reverse(relative_path)?;
		let dest_path = self.config.static_root.join(relative_path);

		// Check if file already exists and is identical
		if dest_path.exists() && !self.options.clear
			&& self.files_identical(&source_path, &dest_path)? {
				if self.options.verbosity > 1 {
					println!("Unmodified: {}", relative_path);
				}
				return Ok(CopyResult::Unmodified);
			}

		if self.options.verbosity > 1 {
			println!(
				"Copying: {} → {}",
				source_path.display(),
				dest_path.display()
			);
		}

		if self.options.dry_run {
			return Ok(CopyResult::Copied);
		}

		// Create parent directories
		if let Some(parent) = dest_path.parent() {
			fs::create_dir_all(parent)?;
		}

		// Copy or link file
		if self.options.link {
			self.create_symlink(&source_path, &dest_path)?;
		} else {
			fs::copy(&source_path, &dest_path)?;
		}

		Ok(CopyResult::Copied)
	}

	#[cfg(unix)]
	fn create_symlink(&self, source: &Path, dest: &Path) -> Result<(), io::Error> {
		use std::os::unix::fs::symlink;

		// Remove existing file/symlink
		if dest.exists() || dest.symlink_metadata().is_ok() {
			fs::remove_file(dest)?;
		}

		symlink(source, dest)
	}

	#[cfg(not(unix))]
	fn create_symlink(&self, source: &Path, dest: &Path) -> Result<(), io::Error> {
		// Fallback to copy on non-Unix systems
		if dest.exists() {
			fs::remove_file(dest)?;
		}
		fs::copy(source, dest)?;
		Ok(())
	}

	fn files_identical(&self, path1: &Path, path2: &Path) -> Result<bool, io::Error> {
		let meta1 = fs::metadata(path1)?;
		let meta2 = fs::metadata(path2)?;

		// Quick check: if sizes differ, files are different
		if meta1.len() != meta2.len() {
			return Ok(false);
		}

		// For small files, compare content
		if meta1.len() < 1024 * 1024 {
			// 1MB threshold
			let content1 = fs::read(path1)?;
			let content2 = fs::read(path2)?;
			return Ok(content1 == content2);
		}

		// For large files, assume identical if same size
		// (more sophisticated comparison could use checksums)
		Ok(true)
	}

	fn print_summary(&self, stats: &CollectStaticStats) {
		println!("\n{} static files copied", stats.copied);

		if stats.unmodified > 0 {
			println!("{} files unmodified", stats.unmodified);
		}

		if stats.skipped > 0 {
			println!("{} files skipped", stats.skipped);
		}

		if stats.deleted > 0 {
			println!("{} files deleted", stats.deleted);
		}
	}
}

#[derive(Debug, PartialEq)]
enum CopyResult {
	Copied,
	#[allow(dead_code)]
	Skipped,
	Unmodified,
}

#[async_trait]
impl BaseCommand for CollectStaticCommand {
	fn name(&self) -> &str {
		"collectstatic"
	}

	async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
		// BaseCommand requires async, but our logic is sync
		// We simply return Ok as the actual execution happens via the sync execute() method
		Ok(())
	}
}

impl Clone for CollectStaticCommand {
	fn clone(&self) -> Self {
		Self {
			config: self.config.clone(),
			options: self.options.clone(),
		}
	}
}
