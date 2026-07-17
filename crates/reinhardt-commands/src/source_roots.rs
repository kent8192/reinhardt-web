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
	/// Workspace `Cargo.lock`. Watched separately so that `cargo update`
	/// against git/registry deps still triggers a rebuild, even when no
	/// path-dep source changes. `None` only when `from_metadata` cannot
	/// anchor on the supplied manifest. See issue #4214.
	pub lockfile: Option<PathBuf>,
}

impl SourceRoots {
	/// Resolve a supplied Cargo package name to its workspace member manifest.
	///
	/// `cargo metadata` represents virtual workspace roots without a package, so
	/// callers that accept `--package` must use the selected member manifest as
	/// the traversal anchor rather than the current directory's `Cargo.toml`.
	pub fn selected_package_manifest(
		metadata: &cargo_metadata::Metadata,
		requested_package: &str,
	) -> Result<PathBuf, String> {
		let matches: Vec<_> = metadata
			.workspace_packages()
			.into_iter()
			.filter(|package| package.name.as_str() == requested_package)
			.collect();

		match matches.as_slice() {
			[package] => Ok(PathBuf::from(package.manifest_path.as_str())),
			[] => Err(format!("Cargo package `{requested_package}` was not found")),
			_ => Err(format!(
				"Cargo package name `{requested_package}` is ambiguous; select a unique package"
			)),
		}
	}

	/// Compute watch targets by BFS from the package whose manifest matches
	/// `cwd_manifest`, traversing only path-based dependencies.
	///
	/// Registry and git dependencies are skipped (their `Dependency::path`
	/// is `None`). If no package in the metadata matches `cwd_manifest`,
	/// the result is empty.
	pub fn from_metadata(metadata: &cargo_metadata::Metadata, cwd_manifest: &Path) -> Self {
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
				lockfile: None,
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
		manifest_files.insert(PathBuf::from(metadata.workspace_root.as_str()).join("Cargo.toml"));

		SourceRoots {
			src_dirs: src_dirs.into_iter().collect(),
			manifest_files: manifest_files.into_iter().collect(),
			lockfile: Some(PathBuf::from(metadata.workspace_root.as_str()).join("Cargo.lock")),
		}
	}

	/// Add another package graph's watch targets while keeping a deterministic set.
	pub fn merge(&mut self, other: Self) {
		self.src_dirs.extend(other.src_dirs);
		self.src_dirs.sort_unstable();
		self.src_dirs.dedup();
		self.manifest_files.extend(other.manifest_files);
		self.manifest_files.sort_unstable();
		self.manifest_files.dedup();
		self.lockfile = self.lockfile.take().or(other.lockfile);
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
		assert_eq!(
			roots.lockfile,
			Some(PathBuf::from("/fixtures/single_crate/Cargo.lock"))
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
				PathBuf::from("/fixtures/ws/Cargo.toml"),
				PathBuf::from("/fixtures/ws/app/Cargo.toml"),
				PathBuf::from("/fixtures/ws/shared/Cargo.toml"),
			]
		);
		assert_eq!(
			roots.lockfile,
			Some(PathBuf::from("/fixtures/ws/Cargo.lock"))
		);
	}

	#[rstest]
	fn selected_workspace_package_provides_the_watch_anchor() {
		// Arrange
		let metadata = parse_metadata(WORKSPACE_JSON);
		let workspace_manifest = PathBuf::from("/fixtures/ws/Cargo.toml");

		// Act
		let anchor = SourceRoots::selected_package_manifest(&metadata, "app")
			.expect("select the requested workspace package");
		let roots = SourceRoots::from_metadata(&metadata, &anchor);

		// Assert
		assert_eq!(anchor, PathBuf::from("/fixtures/ws/app/Cargo.toml"));
		assert_eq!(
			roots.src_dirs,
			vec![
				PathBuf::from("/fixtures/ws/app/src"),
				PathBuf::from("/fixtures/ws/shared/src"),
			]
		);
		assert_ne!(anchor, workspace_manifest);
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
		assert_eq!(
			roots.lockfile,
			Some(PathBuf::from("/fixtures/registry_dep/Cargo.lock"))
		);
	}

	#[rstest]
	fn anchor_not_in_metadata_yields_no_lockfile() {
		// Arrange: use the workspace fixture but a manifest path that
		// matches no package. The function should hit the early-return
		// branch where every output (including lockfile) is empty/None.
		let metadata = parse_metadata(WORKSPACE_JSON);
		let anchor_manifest = PathBuf::from("/nonexistent/Cargo.toml");

		// Act
		let roots = SourceRoots::from_metadata(&metadata, &anchor_manifest);

		// Assert
		assert!(roots.src_dirs.is_empty());
		assert!(roots.manifest_files.is_empty());
		assert_eq!(roots.lockfile, None);
	}

	#[rstest]
	fn merging_roots_keeps_server_and_pages_packages() {
		// Arrange
		let mut roots = SourceRoots {
			src_dirs: vec![PathBuf::from("/fixtures/ws/server/src")],
			manifest_files: vec![PathBuf::from("/fixtures/ws/server/Cargo.toml")],
			lockfile: Some(PathBuf::from("/fixtures/ws/Cargo.lock")),
		};
		let pages_roots = SourceRoots {
			src_dirs: vec![PathBuf::from("/fixtures/ws/frontend/src")],
			manifest_files: vec![PathBuf::from("/fixtures/ws/frontend/Cargo.toml")],
			lockfile: Some(PathBuf::from("/fixtures/ws/Cargo.lock")),
		};

		// Act
		roots.merge(pages_roots);

		// Assert
		assert_eq!(
			roots.src_dirs,
			vec![
				PathBuf::from("/fixtures/ws/frontend/src"),
				PathBuf::from("/fixtures/ws/server/src"),
			]
		);
		assert_eq!(
			roots.manifest_files,
			vec![
				PathBuf::from("/fixtures/ws/frontend/Cargo.toml"),
				PathBuf::from("/fixtures/ws/server/Cargo.toml"),
			]
		);
		assert_eq!(
			roots.lockfile,
			Some(PathBuf::from("/fixtures/ws/Cargo.lock"))
		);
	}
}
