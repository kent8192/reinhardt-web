//! Enumerate source/manifest paths to watch for hot-reload.
//!
//! The hot-reload watcher needs to know which directories and `Cargo.toml`
//! files to subscribe to. We compute that set by starting from the anchor
//! package (the crate whose `Cargo.toml` matches `cwd_manifest`) and walking
//! the dependency graph through *path* dependencies only — registry/git deps
//! live in `~/.cargo/registry` and never change while the user is editing.
//!
//! The traversal is a BFS keyed on the stable `PackageId::repr` so cycles
//! cannot trap us. For each visited package we collect:
//!
//! * `manifest_path.parent().join("src")` as a recursive watch root, and
//! * `manifest_path` itself as a single-file watch target (catches edits to
//!   `Cargo.toml` that should trigger a rebuild).
//!
//! Output vectors are sorted and de-duplicated so callers (and tests) get a
//! deterministic order.

use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

/// Directories and manifest files to watch for hot-reload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRoots {
	/// Per-package `src/` directories to watch recursively.
	pub src_dirs: Vec<PathBuf>,
	/// Per-package `Cargo.toml` files to watch as single files.
	pub manifest_files: Vec<PathBuf>,
}

impl SourceRoots {
	/// Compute watch targets by BFS from the package whose manifest matches
	/// `cwd_manifest`, traversing only path-based dependencies.
	///
	/// Registry and git dependencies are skipped (their `Dependency::path`
	/// is `None`). If no package in the metadata matches `cwd_manifest`,
	/// the result is empty.
	///
	/// All manifest paths are canonicalized before comparison so that a
	/// `cwd_manifest` that traverses a symlink (or differs in case on
	/// case-insensitive filesystems) still matches the corresponding
	/// `cargo metadata` entry.
	pub fn from_metadata(metadata: &cargo_metadata::Metadata, cwd_manifest: &Path) -> Self {
		// Index packages by canonicalized manifest path. Falling back to the
		// raw value when canonicalization fails preserves behavior for
		// synthetic test fixtures whose paths don't exist on disk.
		let pkg_by_manifest: std::collections::HashMap<PathBuf, &cargo_metadata::Package> =
			metadata
				.packages
				.iter()
				.map(|p| {
					(
						canonicalize_or_keep(Path::new(p.manifest_path.as_str())),
						p,
					)
				})
				.collect();

		let cwd_canonical = canonicalize_or_keep(cwd_manifest);

		let Some(anchor) = pkg_by_manifest.get(&cwd_canonical).copied() else {
			return SourceRoots {
				src_dirs: Vec::new(),
				manifest_files: Vec::new(),
			};
		};

		let mut visited: BTreeSet<String> = BTreeSet::new();
		let mut queue: VecDeque<&cargo_metadata::Package> = VecDeque::new();
		let mut src_dirs: BTreeSet<PathBuf> = BTreeSet::new();
		let mut manifest_files: BTreeSet<PathBuf> = BTreeSet::new();

		queue.push_back(anchor);
		visited.insert(anchor.id.repr.clone());

		while let Some(pkg) = queue.pop_front() {
			let manifest = canonicalize_or_keep(Path::new(pkg.manifest_path.as_str()));
			if let Some(parent) = manifest.parent() {
				src_dirs.insert(parent.join("src"));
			}
			manifest_files.insert(manifest);

			for dep in &pkg.dependencies {
				// Only follow path dependencies. Registry/git deps live
				// in immutable cache locations and would only add noise.
				let Some(dep_path) = dep.path.as_ref() else {
					continue;
				};
				let dep_manifest =
					canonicalize_or_keep(&PathBuf::from(dep_path.as_str()).join("Cargo.toml"));
				let Some(next) = pkg_by_manifest.get(&dep_manifest).copied() else {
					continue;
				};
				if visited.insert(next.id.repr.clone()) {
					queue.push_back(next);
				}
			}
		}

		SourceRoots {
			src_dirs: src_dirs.into_iter().collect(),
			manifest_files: manifest_files.into_iter().collect(),
		}
	}
}

/// Resolve symlinks and case differences via `std::fs::canonicalize`, falling
/// back to the input path when the file cannot be resolved (does not exist,
/// permission denied, etc.).
///
/// The fallback keeps the function total so synthetic test fixtures that
/// reference paths under `/fixtures/...` continue to round-trip unchanged.
fn canonicalize_or_keep(path: &Path) -> PathBuf {
	std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::path::PathBuf;

	const SINGLE_CRATE_JSON: &str =
		include_str!("../tests/fixtures/source_roots/single_crate.json");
	const WORKSPACE_JSON: &str = include_str!("../tests/fixtures/source_roots/workspace.json");
	const REGISTRY_DEP_JSON: &str =
		include_str!("../tests/fixtures/source_roots/registry_dep.json");

	fn parse_metadata(json: &str) -> cargo_metadata::Metadata {
		serde_json::from_str(json).expect("fixture must deserialize as cargo_metadata::Metadata")
	}

	#[rstest]
	fn anchor_only_single_crate_returns_one_src_dir_and_manifest() {
		// Arrange
		let metadata = parse_metadata(SINGLE_CRATE_JSON);
		let anchor_manifest = PathBuf::from("/fixtures/single_crate/Cargo.toml");

		// Act
		let roots = SourceRoots::from_metadata(&metadata, &anchor_manifest);

		// Assert
		assert_eq!(
			roots.src_dirs,
			vec![PathBuf::from("/fixtures/single_crate/src")]
		);
		assert_eq!(
			roots.manifest_files,
			vec![PathBuf::from("/fixtures/single_crate/Cargo.toml")]
		);
	}

	#[rstest]
	fn workspace_with_path_dep_includes_both_crate_roots() {
		// Arrange
		let metadata = parse_metadata(WORKSPACE_JSON);
		let anchor_manifest = PathBuf::from("/fixtures/ws/app/Cargo.toml");

		// Act
		let roots = SourceRoots::from_metadata(&metadata, &anchor_manifest);

		// Assert
		assert_eq!(
			roots.src_dirs,
			vec![
				PathBuf::from("/fixtures/ws/app/src"),
				PathBuf::from("/fixtures/ws/shared/src"),
			]
		);
		assert_eq!(
			roots.manifest_files,
			vec![
				PathBuf::from("/fixtures/ws/app/Cargo.toml"),
				PathBuf::from("/fixtures/ws/shared/Cargo.toml"),
			]
		);
	}

	#[rstest]
	fn registry_dependencies_are_excluded() {
		// Arrange
		let metadata = parse_metadata(REGISTRY_DEP_JSON);
		let anchor_manifest = PathBuf::from("/fixtures/registry_dep/Cargo.toml");

		// Act
		let roots = SourceRoots::from_metadata(&metadata, &anchor_manifest);

		// Assert: the only watched dir/manifest is the anchor itself; the
		// registry-resolved `serde` dep must not appear.
		assert_eq!(
			roots.src_dirs,
			vec![PathBuf::from("/fixtures/registry_dep/src")]
		);
		assert_eq!(
			roots.manifest_files,
			vec![PathBuf::from("/fixtures/registry_dep/Cargo.toml")]
		);
	}

	/// Build a one-package metadata blob whose `manifest_path` points at
	/// `manifest_path_str`. Used by the canonicalization regression test.
	fn synthetic_single_crate_metadata(manifest_path_str: &str) -> cargo_metadata::Metadata {
		let parent = Path::new(manifest_path_str)
			.parent()
			.expect("manifest path must have a parent")
			.to_string_lossy()
			.into_owned();
		let id = format!("path+file://{parent}#0.1.0");
		let json = format!(
			r#"{{
  "packages": [{{
    "name": "anchor",
    "version": "0.1.0",
    "id": "{id}",
    "license": null,
    "license_file": null,
    "description": null,
    "source": null,
    "dependencies": [],
    "targets": [],
    "features": {{}},
    "manifest_path": "{manifest_path_str}",
    "metadata": null,
    "publish": null,
    "authors": [],
    "categories": [],
    "keywords": [],
    "readme": null,
    "repository": null,
    "homepage": null,
    "documentation": null,
    "edition": "2024",
    "links": null,
    "default_run": null,
    "rust_version": null
  }}],
  "workspace_members": ["{id}"],
  "workspace_default_members": ["{id}"],
  "resolve": null,
  "target_directory": "/tmp/canonicalize_test_target",
  "build_directory": "/tmp/canonicalize_test_target",
  "version": 1,
  "workspace_root": "{parent}",
  "metadata": null
}}"#,
		);
		serde_json::from_str(&json).expect("synthetic metadata must deserialize")
	}

	/// `cwd_manifest` going through a symlink must still resolve to the
	/// canonicalized package manifest entry — without canonicalization the
	/// PathBuf comparison fails and the watcher silently watches nothing.
	#[cfg(unix)]
	#[rstest]
	fn cwd_manifest_through_symlink_resolves_to_canonical_package() {
		// Arrange: real crate dir + Cargo.toml on disk, plus a symlink dir
		// pointing at the real one. cargo metadata reports the real path;
		// the user's cwd_manifest goes through the symlink.
		let tmp = tempfile::tempdir().expect("create tempdir");
		let real_dir = tmp.path().join("real_crate");
		std::fs::create_dir(&real_dir).unwrap();
		let real_manifest = real_dir.join("Cargo.toml");
		std::fs::write(&real_manifest, b"[package]\nname = \"anchor\"\n").unwrap();

		let link_dir = tmp.path().join("link_crate");
		std::os::unix::fs::symlink(&real_dir, &link_dir).expect("create symlink");
		let link_manifest = link_dir.join("Cargo.toml");

		let metadata = synthetic_single_crate_metadata(&real_manifest.to_string_lossy());

		// Act: anchor lookup must succeed even though the `cwd_manifest`
		// path traverses the symlink.
		let roots = SourceRoots::from_metadata(&metadata, &link_manifest);

		// Assert: a non-empty result indicates the lookup matched. Compare
		// against the canonicalized manifest path (the function exposes
		// canonicalized paths to callers).
		let canonical = std::fs::canonicalize(&real_manifest).unwrap();
		assert_eq!(roots.manifest_files, vec![canonical.clone()]);
		assert_eq!(roots.src_dirs, vec![canonical.parent().unwrap().join("src")]);
	}
}
