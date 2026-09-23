//! Verified immutable generation handles and atomic request snapshot selection.

use super::inventory::{build_id, digest_reader};
use super::manifest::{decode_manifest, encode_manifest};
use super::model::*;
use cap_std::fs::Dir;
use reinhardt_core::types::static_assets::AssetUrlSnapshot;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

/// Validation policy for a selected active or retained manifest (P0).
#[derive(Debug, Clone)]
pub struct SnapshotOptions {
	pub(super) mode: AssetMode,
	pub(super) manifest: PathBuf,
	explicit_manifest: bool,
	expected: Option<String>,
}

impl SnapshotOptions {
	/// Require production assets and the canonical active pointer.
	pub fn production() -> Self {
		Self {
			mode: AssetMode::Production,
			manifest: "manifest.json".into(),
			explicit_manifest: false,
			expected: None,
		}
	}
	/// Require explicit development mode; roots cannot mix modes.
	pub fn development() -> Self {
		Self {
			mode: AssetMode::Development,
			..Self::production()
		}
	}
	/// Select a manifest confined to the configured static root.
	pub fn manifest_path(mut self, path: PathBuf) -> Self {
		self.manifest = path;
		self.explicit_manifest = true;
		self
	}
	/// Require a deployment-pinned generation instead of accepting stale output.
	pub fn expected_build_id(mut self, id: String) -> Self {
		self.expected = Some(id);
		self
	}
	pub(super) fn for_mode(mode: AssetMode) -> Self {
		Self {
			mode,
			..Self::production()
		}
	}
}

/// A generation whose manifest, inventory, and every output were verified (P0).
///
/// Each request retains one instance. Old generation directories must remain on
/// disk during deployment transitions; changing the active pointer never mutates
/// this handle. File access remains confined to the selected generation.
#[derive(Debug)]
pub struct ManifestSnapshot {
	manifest: AssetManifestV2,
	manifest_bytes: Vec<u8>,
	directory: Dir,
	label: PathBuf,
	reverse: BTreeMap<String, String>,
}

impl ManifestSnapshot {
	/// Load complete version 2 output; partial, modified, or stale output fails.
	pub fn load(root: &Path, options: &SnapshotOptions) -> Result<Self, AssetBuildError> {
		let directory = Dir::open_ambient_dir(root, cap_std::ambient_authority())
			.map_err(|e| AssetBuildError::io(root, e))?;
		Self::load_from(&directory, root, options)
	}

	fn load_from(
		root: &Dir,
		label: &Path,
		options: &SnapshotOptions,
	) -> Result<Self, AssetBuildError> {
		if !options.explicit_manifest
			&& root.exists("manifest.json")
			&& root.exists("staticfiles.json")
		{
			return Err(AssetBuildError::manifest(
				"both manifest.json and staticfiles.json exist; select --asset-manifest explicitly before serving this publication",
			));
		}
		let selected = if options.manifest.is_absolute() {
			options.manifest.strip_prefix(label).map_err(|_| {
				AssetBuildError::manifest("selected manifest is outside the static root")
			})?
		} else {
			&options.manifest
		};
		let bytes = root
			.read(selected)
			.map_err(|e| AssetBuildError::io(label.join(selected), e))?;
		let DecodedAssetManifest::V2(manifest) = decode_manifest(&bytes)? else {
			return Err(AssetBuildError::manifest(
				"legacy manifest cannot serve a complete Pages generation; run buildstatic with an explicit legacy input manifest",
			));
		};
		if manifest.mode != options.mode {
			return Err(AssetBuildError::ModeMismatch {
				expected: options.mode,
				observed: manifest.mode,
			});
		}
		if let Some(expected) = &options.expected
			&& expected != &manifest.build_id
		{
			return Err(AssetBuildError::StaleGeneration {
				expected: expected.clone(),
				observed: manifest.build_id,
			});
		}
		if encode_manifest(&manifest)? != bytes {
			return Err(AssetBuildError::manifest(
				"version 2 manifest is not canonical; rebuild rather than editing generated metadata",
			));
		}
		let generation = format!("builds/{}", manifest.build_id);
		for path in ["builds", generation.as_str()] {
			if !root
				.symlink_metadata(path)
				.map_err(|e| AssetBuildError::io(label.join(path), e))?
				.is_dir()
			{
				return Err(AssetBuildError::manifest(format!(
					"generation directory {path:?} must not be a symlink or special file"
				)));
			}
		}
		let directory = root
			.open_dir(&generation)
			.map_err(|e| AssetBuildError::io(label.join(&generation), e))?;
		let generation_bytes = directory
			.read("manifest.json")
			.map_err(|e| AssetBuildError::io(label.join(&generation).join("manifest.json"), e))?;
		if generation_bytes != bytes {
			return Err(AssetBuildError::manifest(
				"active and immutable generation manifests disagree; publish the complete generation before activating it",
			));
		}
		let prefix = format!("{generation}/");
		let relative: BTreeMap<_, _> = manifest
			.paths
			.iter()
			.map(|(name, path)| {
				(
					name.clone(),
					path.strip_prefix(&prefix)
						.expect("decoded manifest paths are generation-bound")
						.to_owned(),
				)
			})
			.collect();
		let computed = build_id(
			manifest.mode,
			&relative,
			&manifest.assets,
			&manifest.entrypoints,
			&manifest.pipeline,
		)?;
		if computed != manifest.build_id {
			return Err(AssetBuildError::manifest(format!(
				"inventory digest {computed} disagrees with build ID {}; rebuild stale output",
				manifest.build_id
			)));
		}
		let mut actual = BTreeSet::new();
		collect_files(&directory, "", &mut actual)?;
		let mut expected: BTreeSet<_> = relative.values().cloned().collect();
		expected.insert("manifest.json".into());
		if actual != expected {
			return Err(AssetBuildError::manifest(format!(
				"incomplete or unexpected generation files: missing {:?}, unexpected {:?}",
				expected.difference(&actual).collect::<Vec<_>>(),
				actual.difference(&expected).collect::<Vec<_>>()
			)));
		}
		let snapshot = Self {
			reverse: relative
				.into_iter()
				.map(|(name, path)| (path, name))
				.collect(),
			manifest,
			manifest_bytes: bytes,
			directory,
			label: label.join(generation),
		};
		for logical in snapshot.manifest.assets.keys() {
			snapshot.open_asset(logical)?;
		}
		Ok(snapshot)
	}

	/// Borrow the validated complete manifest.
	pub fn manifest(&self) -> &AssetManifestV2 {
		&self.manifest
	}
	/// Borrow the canonical bytes also used by the active pointer.
	pub fn manifest_bytes(&self) -> &[u8] {
		&self.manifest_bytes
	}
	/// Project public URLs without exposing filesystem locations or private options.
	pub fn url_snapshot(&self, static_url: &str) -> Result<AssetUrlSnapshot, AssetBuildError> {
		Ok(AssetUrlSnapshot::new(
			self.manifest.build_id.clone(),
			static_url.into(),
			self.manifest.paths.clone(),
		)?)
	}
	/// Read a verified asset, detecting removal or modification since startup.
	pub fn read_asset(&self, logical: &str) -> Result<Vec<u8>, AssetBuildError> {
		let mut output = Vec::new();
		self.open_asset(logical)?
			.read_to_end(&mut output)
			.map_err(|e| AssetBuildError::io(&self.label, e))?;
		Ok(output)
	}
	/// Find an exact decoded path inside this generation.
	pub fn logical_path(&self, relative: &str) -> Option<&str> {
		self.reverse.get(relative).map(String::as_str)
	}

	pub(super) fn open_asset(&self, logical: &str) -> Result<File, AssetBuildError> {
		let metadata = self.manifest.assets.get(logical).ok_or_else(|| {
			AssetBuildError::input(logical, "asset is absent from this generation")
		})?;
		let prefix = format!("builds/{}/", self.manifest.build_id);
		let relative = self.manifest.paths[logical]
			.strip_prefix(&prefix)
			.expect("validated generation path");
		let mut file = self
			.directory
			.open(relative)
			.map_err(|e| AssetBuildError::io(self.label.join(relative), e))?
			.into_std();
		let (size, sha256) = digest_reader(&mut file, &self.label.join(relative))?;
		if size != metadata.size || sha256 != metadata.sha256 {
			return Err(AssetBuildError::input(
				logical,
				format!(
					"published output is truncated or modified at {}; republish the complete generation",
					self.label.join(relative).display()
				),
			));
		}
		file.rewind()
			.map_err(|e| AssetBuildError::io(self.label.join(relative), e))?;
		Ok(file)
	}
}

fn collect_files(
	directory: &Dir,
	prefix: &str,
	output: &mut BTreeSet<String>,
) -> Result<(), AssetBuildError> {
	for entry in directory
		.entries()
		.map_err(|e| AssetBuildError::io(prefix, e))?
	{
		let entry = entry.map_err(|e| AssetBuildError::io(prefix, e))?;
		let name = entry
			.file_name()
			.into_string()
			.map_err(|_| AssetBuildError::manifest("generation has a non-UTF-8 filename"))?;
		let relative = format!("{prefix}{name}");
		let kind = entry
			.file_type()
			.map_err(|e| AssetBuildError::io(&relative, e))?;
		if kind.is_dir() {
			collect_files(
				&directory
					.open_dir(&name)
					.map_err(|e| AssetBuildError::io(&relative, e))?,
				&format!("{relative}/"),
				output,
			)?;
		} else if kind.is_file() {
			output.insert(relative);
		} else {
			return Err(AssetBuildError::manifest(format!(
				"generation contains a symlink or special file {relative:?}"
			)));
		}
	}
	Ok(())
}

#[derive(Debug)]
struct State {
	active: Arc<ManifestSnapshot>,
	generations: BTreeMap<String, Arc<ManifestSnapshot>>,
}

/// Request-pinned active and retained generations with validated reload (P0).
#[derive(Debug)]
pub struct ManifestStore {
	root: Dir,
	label: PathBuf,
	options: SnapshotOptions,
	state: RwLock<State>,
	reloading: Mutex<()>,
}

impl ManifestStore {
	/// Validate the active pointer and every retained generation before serving.
	pub fn open(root: PathBuf, options: SnapshotOptions) -> Result<Self, AssetBuildError> {
		let root = if root.is_absolute() {
			root
		} else {
			std::env::current_dir()
				.map_err(|e| AssetBuildError::io(&root, e))?
				.join(root)
		};
		let directory = Dir::open_ambient_dir(&root, cap_std::ambient_authority())
			.map_err(|e| AssetBuildError::io(&root, e))?;
		let state = load_state(&directory, &root, &options)?;
		Ok(Self {
			root: directory,
			label: root,
			options,
			state: RwLock::new(state),
			reloading: Mutex::new(()),
		})
	}
	/// Pin the active generation for an entire request.
	pub fn active(&self) -> Arc<ManifestSnapshot> {
		self.state
			.read()
			.unwrap_or_else(|e| e.into_inner())
			.active
			.clone()
	}
	/// Pin the exact retained generation requested by an old page.
	pub fn generation(&self, id: &str) -> Option<Arc<ManifestSnapshot>> {
		self.state
			.read()
			.unwrap_or_else(|e| e.into_inner())
			.generations
			.get(id)
			.cloned()
	}
	/// Validate outside the request lock; any failure preserves the last-good state.
	pub fn reload(&self) -> Result<(), AssetBuildError> {
		let _reload_guard = self.reloading.lock().unwrap_or_else(|e| e.into_inner());
		let next = load_state(&self.root, &self.label, &self.options)?;
		*self.state.write().unwrap_or_else(|e| e.into_inner()) = next;
		Ok(())
	}
}

fn load_state(
	root: &Dir,
	label: &Path,
	options: &SnapshotOptions,
) -> Result<State, AssetBuildError> {
	let active = Arc::new(ManifestSnapshot::load_from(root, label, options)?);
	let mut generations = BTreeMap::from([(active.manifest.build_id.clone(), active.clone())]);
	let builds = root
		.open_dir("builds")
		.map_err(|e| AssetBuildError::io(label.join("builds"), e))?;
	for entry in builds
		.entries()
		.map_err(|e| AssetBuildError::io(label.join("builds"), e))?
	{
		let entry = entry.map_err(|e| AssetBuildError::io(label, e))?;
		let id = entry
			.file_name()
			.into_string()
			.map_err(|_| AssetBuildError::manifest("non-UTF-8 retained generation"))?;
		if generations.contains_key(&id) {
			continue;
		}
		if id.len() != 64
			|| !id
				.bytes()
				.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
		{
			return Err(AssetBuildError::manifest(format!(
				"unrecognized retained generation {id:?}; inspect incomplete deployment output"
			)));
		}
		let retained_options = SnapshotOptions::for_mode(options.mode)
			.manifest_path(format!("builds/{id}/manifest.json").into())
			.expected_build_id(id.clone());
		let retained = ManifestSnapshot::load_from(root, label, &retained_options)?;
		generations.insert(id, Arc::new(retained));
	}
	for entry in root.entries().map_err(|e| AssetBuildError::io(label, e))? {
		let entry = entry.map_err(|e| AssetBuildError::io(label, e))?;
		if entry
			.file_name()
			.to_string_lossy()
			.starts_with(".asset-staging-")
		{
			tracing::warn!(path = ?label.join(entry.file_name()), "ignoring unpublished asset staging; inspect after any active publisher finishes");
		}
	}
	Ok(State {
		active,
		generations,
	})
}
