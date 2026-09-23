//! Complete immutable generations followed by one atomic active-pointer rename.

#[cfg(unix)]
use super::manifest::decode_manifest;
use super::model::*;
use super::pipeline::PreparedGeneration;
use super::snapshot::ManifestSnapshot;
#[cfg(unix)]
use super::snapshot::SnapshotOptions;
#[cfg(unix)]
use std::fs::{self, File};
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;

#[cfg(all(test, unix))]
type CheckpointHook = dyn Fn(Checkpoint) -> Result<(), AssetBuildError> + Send + Sync;

/// Publish under a configured static root without removing older generations (P0).
///
/// Publication is supported on Unix filesystems with atomic same-filesystem rename
/// and OS file locks. Other platforms fail explicitly before changing output.
/// Keep a shared persistent asset root across deployments while old pages exist.
pub struct AssetPublisher {
	root: PathBuf,
	#[cfg(all(test, unix))]
	checkpoint: Option<Box<CheckpointHook>>,
}

impl AssetPublisher {
	/// Select the output directory from resolved static settings, never a URL.
	pub fn new(root: PathBuf) -> Self {
		Self {
			root,
			#[cfg(all(test, unix))]
			checkpoint: None,
		}
	}
	/// Consume captured final bytes and activate only after durable promotion.
	pub fn publish(
		&self,
		prepared: PreparedGeneration,
	) -> Result<ManifestSnapshot, AssetBuildError> {
		#[cfg(not(unix))]
		{
			let _ = prepared;
			Err(AssetBuildError::PublicationConflict {
				reason: format!(
					"atomic asset activation at {} is currently supported only on Unix filesystems",
					self.root.display()
				),
			})
		}
		#[cfg(unix)]
		{
			self.publish_unix(prepared)
		}
	}

	#[cfg(unix)]
	fn publish_unix(
		&self,
		prepared: PreparedGeneration,
	) -> Result<ManifestSnapshot, AssetBuildError> {
		use std::os::unix::fs::PermissionsExt;
		fs::create_dir_all(&self.root).map_err(|e| AssetBuildError::io(&self.root, e))?;
		let root = self
			.root
			.canonicalize()
			.map_err(|e| AssetBuildError::io(&self.root, e))?;
		let directory = cap_std::fs::Dir::open_ambient_dir(&root, cap_std::ambient_authority())
			.map_err(|e| AssetBuildError::io(&root, e))?;
		let lock_path = root.join(".publication.lock");
		if directory
			.symlink_metadata(".publication.lock")
			.is_ok_and(|m| !m.is_file())
		{
			return Err(AssetBuildError::manifest(
				"publication lock must be a regular file",
			));
		}
		// Creation has a single winner. Contending publishers open that same inode
		// without combining another creation operation with opening the lock.
		let lock = match directory.open_with(
			".publication.lock",
			cap_std::fs::OpenOptions::new()
				.read(true)
				.write(true)
				.create_new(true),
		) {
			Ok(file) => file,
			Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => directory
				.open_with(
					".publication.lock",
					cap_std::fs::OpenOptions::new().read(true).write(true),
				)
				.map_err(|e| AssetBuildError::io(&lock_path, e))?,
			Err(error) => return Err(AssetBuildError::io(&lock_path, error)),
		}
		.into_std();
		lock.lock()
			.map_err(|e| AssetBuildError::io(&lock_path, e))?;
		// The file owns the OS lock until this scope exits, including error paths.
		let _lock_guard = lock;
		match directory.symlink_metadata("staticfiles.json") {
			Ok(_) => {
				return Err(AssetBuildError::manifest(
					"destination contains a competing legacy staticfiles.json; import it into a separate publication root or archive it before migration",
				));
			}
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
			Err(error) => return Err(AssetBuildError::io(root.join("staticfiles.json"), error)),
		}
		let manifest = prepared.manifest();
		let mode = manifest.mode;
		if directory.exists("manifest.json") {
			let bytes = directory
				.read("manifest.json")
				.map_err(|e| AssetBuildError::io(root.join("manifest.json"), e))?;
			if let DecodedAssetManifest::V2(_) = decode_manifest(&bytes)? {
				ManifestSnapshot::load(
					&root,
					&SnapshotOptions::for_mode(mode).manifest_path("manifest.json".into()),
				)?;
			}
		}
		if directory
			.symlink_metadata("builds")
			.is_ok_and(|m| !m.is_dir())
		{
			return Err(AssetBuildError::manifest(
				"builds must be a real directory, not a symlink or file",
			));
		}
		directory
			.create_dir_all("builds")
			.map_err(|e| AssetBuildError::io(root.join("builds"), e))?;
		let builds = directory
			.open_dir("builds")
			.map_err(|e| AssetBuildError::io(root.join("builds"), e))?;
		for entry in builds
			.entries()
			.map_err(|e| AssetBuildError::io(root.join("builds"), e))?
		{
			let entry = entry.map_err(|e| AssetBuildError::io(root.join("builds"), e))?;
			let id = entry
				.file_name()
				.into_string()
				.map_err(|_| AssetBuildError::manifest("retained generation name must be UTF-8"))?;
			let options = SnapshotOptions::for_mode(mode)
				.manifest_path(PathBuf::from("builds").join(&id).join("manifest.json"))
				.expected_build_id(id);
			ManifestSnapshot::load(&root, &options)?;
		}
		let generation = format!("builds/{}", manifest.build_id);
		let generation_manifest = PathBuf::from(&generation).join("manifest.json");
		let options = SnapshotOptions::for_mode(mode)
			.manifest_path(generation_manifest)
			.expected_build_id(manifest.build_id.clone());
		if directory.exists(&generation) {
			let existing = ManifestSnapshot::load(&root, &options)?;
			if existing.manifest_bytes() != prepared.manifest_bytes() {
				return Err(AssetBuildError::PublicationConflict {
					reason: format!(
						"existing generation {} has different manifest bytes",
						manifest.build_id
					),
				});
			}
		} else {
			let staging = tempfile::Builder::new()
				.prefix(".asset-staging-")
				.tempdir_in(&root)
				.map_err(|e| AssetBuildError::io(&root, e))?;
			let prefix = format!("{generation}/");
			for (logical, asset) in &prepared.assets {
				let relative = manifest.paths[logical]
					.strip_prefix(&prefix)
					.expect("prepared generation path");
				let path = staging.path().join(relative);
				fs::create_dir_all(path.parent().expect("asset parent"))
					.map_err(|e| AssetBuildError::io(&path, e))?;
				let mut output = File::create(&path).map_err(|e| AssetBuildError::io(&path, e))?;
				std::io::copy(&mut asset.open()?, &mut output)
					.map_err(|e| AssetBuildError::io(&path, e))?;
				output
					.sync_all()
					.map_err(|e| AssetBuildError::io(&path, e))?;
			}
			write_synced(
				&staging.path().join("manifest.json"),
				prepared.manifest_bytes(),
			)?;
			normalize_tree_permissions(staging.path())?;
			sync_tree(staging.path())?;
			#[cfg(all(test, unix))]
			self.check(Checkpoint::BeforePromotion)?;
			fs::rename(staging.path(), root.join(&generation))
				.map_err(|e| AssetBuildError::io(root.join(&generation), e))?;
			sync_directory(&root.join("builds"))?;
			#[cfg(all(test, unix))]
			self.check(Checkpoint::AfterPromotion)?;
		}
		let snapshot = ManifestSnapshot::load(&root, &options)?;
		let mut active = tempfile::Builder::new()
			.prefix(".manifest-")
			.tempfile_in(&root)
			.map_err(|e| AssetBuildError::io(&root, e))?;
		active
			.write_all(prepared.manifest_bytes())
			.map_err(|e| AssetBuildError::io(active.path(), e))?;
		active
			.as_file()
			.set_permissions(fs::Permissions::from_mode(0o644))
			.map_err(|e| AssetBuildError::io(active.path(), e))?;
		active
			.as_file()
			.sync_all()
			.map_err(|e| AssetBuildError::io(active.path(), e))?;
		#[cfg(all(test, unix))]
		self.check(Checkpoint::BeforeActivation)?;
		active
			.persist(root.join("manifest.json"))
			.map_err(|e| AssetBuildError::io(root.join("manifest.json"), e.error))?;
		sync_directory(&root)?;
		Ok(snapshot)
	}

	#[cfg(all(test, unix))]
	fn check(&self, checkpoint: Checkpoint) -> Result<(), AssetBuildError> {
		self.checkpoint
			.as_ref()
			.map_or(Ok(()), |hook| hook(checkpoint))
	}
}

#[cfg(unix)]
fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), AssetBuildError> {
	let mut file = File::create(path).map_err(|e| AssetBuildError::io(path, e))?;
	file.write_all(bytes)
		.map_err(|e| AssetBuildError::io(path, e))?;
	file.sync_all().map_err(|e| AssetBuildError::io(path, e))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), AssetBuildError> {
	File::open(path)
		.and_then(|file| file.sync_all())
		.map_err(|e| AssetBuildError::io(path, e))
}

#[cfg(unix)]
fn sync_tree(path: &Path) -> Result<(), AssetBuildError> {
	for entry in fs::read_dir(path).map_err(|e| AssetBuildError::io(path, e))? {
		let entry = entry.map_err(|e| AssetBuildError::io(path, e))?;
		if entry
			.file_type()
			.map_err(|e| AssetBuildError::io(entry.path(), e))?
			.is_dir()
		{
			sync_tree(&entry.path())?;
		}
	}
	sync_directory(path)
}

#[cfg(unix)]
fn normalize_tree_permissions(path: &Path) -> Result<(), AssetBuildError> {
	use std::os::unix::fs::PermissionsExt;
	for entry in fs::read_dir(path).map_err(|e| AssetBuildError::io(path, e))? {
		let entry = entry.map_err(|e| AssetBuildError::io(path, e))?;
		let child = entry.path();
		let metadata = fs::symlink_metadata(&child).map_err(|e| AssetBuildError::io(&child, e))?;
		if metadata.file_type().is_dir() {
			normalize_tree_permissions(&child)?;
			fs::set_permissions(&child, fs::Permissions::from_mode(0o755))
				.map_err(|e| AssetBuildError::io(&child, e))?;
		} else if metadata.file_type().is_file() {
			fs::set_permissions(&child, fs::Permissions::from_mode(0o644))
				.map_err(|e| AssetBuildError::io(&child, e))?;
		} else {
			return Err(AssetBuildError::manifest(format!(
				"staging generation contains a symlink or special file at {}",
				child.display()
			)));
		}
	}
	fs::set_permissions(path, fs::Permissions::from_mode(0o755))
		.map_err(|e| AssetBuildError::io(path, e))
}

#[cfg(all(test, unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Checkpoint {
	BeforePromotion,
	AfterPromotion,
	BeforeActivation,
}

#[cfg(all(test, unix))]
mod tests {
	use super::*;
	use crate::staticfiles::publication::{AssetInput, AssetPipeline};
	use rstest::rstest;

	fn prepare(bytes: &[u8]) -> PreparedGeneration {
		let mut pipeline = AssetPipeline::new();
		pipeline
			.add_input(AssetInput::bytes("a.txt", bytes.to_vec()))
			.unwrap();
		pipeline.prepare(AssetMode::Production).unwrap()
	}

	#[rstest]
	fn normalize_tree_permissions_covers_nested_assets_and_manifests() {
		use std::os::unix::fs::PermissionsExt;
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let nested = root.path().join("builds/generation/js");
		fs::create_dir_all(&nested).unwrap();
		let asset = nested.join("app.js");
		let manifest = root.path().join("builds/generation/manifest.json");
		fs::write(&asset, b"asset").unwrap();
		fs::write(&manifest, b"manifest").unwrap();
		for path in [
			root.path().to_path_buf(),
			root.path().join("builds"),
			root.path().join("builds/generation"),
			nested,
		] {
			fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
		}
		for path in [&asset, &manifest] {
			fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
		}

		// Act
		normalize_tree_permissions(root.path()).unwrap();

		// Assert
		assert_eq!(
			fs::metadata(&asset).unwrap().permissions().mode() & 0o777,
			0o644
		);
		assert_eq!(
			fs::metadata(&manifest).unwrap().permissions().mode() & 0o777,
			0o644
		);
		assert_eq!(
			fs::metadata(root.path().join("builds/generation/js"))
				.unwrap()
				.permissions()
				.mode() & 0o777,
			0o755
		);
	}

	#[rstest]
	#[case(Checkpoint::BeforePromotion)]
	#[case(Checkpoint::AfterPromotion)]
	#[case(Checkpoint::BeforeActivation)]
	fn failure_before_activation_leaves_complete_old_generation(#[case] stop: Checkpoint) {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let initial = AssetPublisher::new(root.path().into())
			.publish(prepare(b"one"))
			.unwrap();
		let original = fs::read(root.path().join("manifest.json")).unwrap();
		let publisher = AssetPublisher {
			root: root.path().into(),
			checkpoint: Some(Box::new(move |checkpoint| {
				if checkpoint == stop {
					Err(AssetBuildError::PublicationConflict {
						reason: "injected interruption".into(),
					})
				} else {
					Ok(())
				}
			})),
		};
		// Act
		let failed = publisher.publish(prepare(b"two"));
		// Assert
		assert!(failed.is_err());
		assert_eq!(
			fs::read(root.path().join("manifest.json")).unwrap(),
			original
		);
		assert_eq!(
			ManifestSnapshot::load(root.path(), &SnapshotOptions::production())
				.unwrap()
				.manifest()
				.build_id,
			initial.manifest().build_id
		);
	}

	struct ChildGuard(std::process::Child);
	impl Drop for ChildGuard {
		fn drop(&mut self) {
			let _ = self.0.kill();
			let _ = self.0.wait();
		}
	}

	#[rstest]
	fn crash_child() {
		let Some(root) = std::env::var_os("REINHARDT_ASSET_CRASH_TEST_ROOT") else {
			return;
		};
		let publisher = AssetPublisher {
			root: root.into(),
			checkpoint: Some(Box::new(|checkpoint| {
				if checkpoint == Checkpoint::AfterPromotion {
					println!("GENERATION_PROMOTED");
					std::io::stdout().flush().unwrap();
					let mut signal = [0];
					std::io::Read::read_exact(&mut std::io::stdin(), &mut signal).unwrap();
				}
				Ok(())
			})),
		};
		publisher.publish(prepare(b"two")).unwrap();
	}

	#[rstest]
	fn killed_publisher_releases_os_lock_without_activating_unfinished_pointer() {
		use std::io::BufRead;
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let a = AssetPublisher::new(root.path().into())
			.publish(prepare(b"one"))
			.unwrap();
		let child = std::process::Command::new(std::env::current_exe().unwrap())
			.args([
				"--exact",
				"staticfiles::publication::publisher::tests::crash_child",
				"--nocapture",
			])
			.env("REINHARDT_ASSET_CRASH_TEST_ROOT", root.path())
			.stdin(std::process::Stdio::piped())
			.stdout(std::process::Stdio::piped())
			.spawn()
			.unwrap();
		let mut guard = ChildGuard(child);
		let mut reader = std::io::BufReader::new(guard.0.stdout.take().unwrap());
		let mut line = String::new();
		loop {
			line.clear();
			assert_ne!(
				reader.read_line(&mut line).unwrap(),
				0,
				"child exited before promotion"
			);
			if line.contains("GENERATION_PROMOTED") {
				break;
			}
		}
		// Act
		guard.0.kill().unwrap();
		guard.0.wait().unwrap();
		let unchanged =
			ManifestSnapshot::load(root.path(), &SnapshotOptions::production()).unwrap();
		let recovered = AssetPublisher::new(root.path().into())
			.publish(prepare(b"two"))
			.unwrap();
		// Assert
		assert_eq!(unchanged.manifest().build_id, a.manifest().build_id);
		assert_eq!(recovered.read_asset("a.txt").unwrap(), b"two");
	}
}
