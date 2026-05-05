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
//
// `dead_code` allowed: this type is consumed by `debounced_watcher` (Task 5).
// Remove the attribute once the watcher pipeline lands.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceRoots {
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
	//
	// `dead_code` allowed: callers land in subsequent tasks (Task 5
	// wires this into `debounced_watcher`).
	#[allow(dead_code)]
	pub(crate) fn from_metadata(metadata: &cargo_metadata::Metadata, cwd_manifest: &Path) -> Self {
		// Index packages by manifest path and by name for two lookup styles
		// (BFS uses the `path` field on a Dependency to find the next pkg).
		let pkg_by_manifest: std::collections::HashMap<PathBuf, &cargo_metadata::Package> =
			metadata
				.packages
				.iter()
				.map(|p| (PathBuf::from(p.manifest_path.as_str()), p))
				.collect();

		let Some(anchor) = pkg_by_manifest.get(cwd_manifest).copied() else {
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
			let manifest = PathBuf::from(pkg.manifest_path.as_str());
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
				let dep_manifest = PathBuf::from(dep_path.as_str()).join("Cargo.toml");
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
}
