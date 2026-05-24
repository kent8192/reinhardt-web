//! Directory walker for the migrate-manouche-v2 codemod.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Returns every `*.rs` file under `root`, skipping `target/` and hidden dirs.
///
/// The `root` itself is never skipped even if its file_name starts with `.`
/// (e.g. macOS `tempdir()` returns paths under `/var/folders/.../T/.tmpXXX`).
pub fn find_rs_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
	let mut out = Vec::new();
	for entry in WalkDir::new(root)
		.into_iter()
		.filter_entry(|e| !is_skipped_descendant(root, e.path()))
	{
		let entry = entry?;
		if entry.file_type().is_file()
			&& entry.path().extension().map(|e| e == "rs").unwrap_or(false)
		{
			out.push(entry.into_path());
		}
	}
	out.sort();
	Ok(out)
}

fn is_skipped_descendant(root: &Path, p: &Path) -> bool {
	// Never skip the anchor itself.
	if p == root {
		return false;
	}
	let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
	// Skip the cargo target directory and any hidden directory (e.g. `.git`),
	// but only when encountered as a descendant of `root`.
	// Files in hidden directories are not skipped because `filter_entry`
	// already prevents descent into them — we only need to filter
	// hidden directories themselves.
	if name == "target" {
		return true;
	}
	p.is_dir() && name.starts_with('.')
}
