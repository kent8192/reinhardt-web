//! Directory walker for the migrate-manouche-v2 codemod.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Returns every `*.rs` file under `root`, skipping `target/` and hidden dirs.
pub fn find_rs_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
	let mut out = Vec::new();
	for entry in WalkDir::new(root)
		.into_iter()
		.filter_entry(|e| !is_skipped(e.path()))
	{
		let entry = entry?;
		if entry.file_type().is_file()
			&& entry.path().extension().map(|e| e == "rs").unwrap_or(false)
		{
			out.push(entry.into_path());
		}
	}
	Ok(out)
}

fn is_skipped(p: &Path) -> bool {
	let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
	// Skip the cargo target directory and any hidden directory (e.g. `.git`),
	// but not the path component itself ("." / "..") used to anchor the walk.
	name == "target" || (name.starts_with('.') && name != "." && name != "..")
}
