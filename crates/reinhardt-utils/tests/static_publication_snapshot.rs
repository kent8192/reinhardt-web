#![cfg(all(feature = "asset-publication", unix))]

use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetPublisher, ManifestSnapshot, ManifestStore,
	PreparedGeneration, SnapshotOptions,
};
use rstest::rstest;

fn prepare(bytes: &[u8], mode: AssetMode) -> PreparedGeneration {
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes("a.txt", bytes.to_vec()))
		.unwrap();
	pipeline.prepare(mode).unwrap()
}

#[rstest]
fn active_and_retained_generations_survive_publication_and_rollback() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let publisher = AssetPublisher::new(root.path().into());
	let a = publisher
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	let original_manifest = std::fs::read(root.path().join("manifest.json")).unwrap();
	let store = ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap();
	let pinned = store.active();
	// Act
	let b = publisher
		.publish(prepare(b"two", AssetMode::Production))
		.unwrap();
	store.reload().unwrap();
	// Assert
	assert_eq!(store.active().manifest().build_id, b.manifest().build_id);
	assert_eq!(pinned.read_asset("a.txt").unwrap(), b"one");
	assert_eq!(
		store
			.generation(&a.manifest().build_id)
			.unwrap()
			.read_asset("a.txt")
			.unwrap(),
		b"one"
	);
	assert_eq!(
		std::fs::read(
			root.path()
				.join(format!("builds/{}/manifest.json", b.manifest().build_id))
		)
		.unwrap(),
		std::fs::read(root.path().join("manifest.json")).unwrap()
	);
	// Act: a deliberately selected old generation is still valid.
	std::fs::write(root.path().join("manifest.json"), original_manifest).unwrap();
	store.reload().unwrap();
	// Assert
	assert_eq!(store.active().manifest().build_id, a.manifest().build_id);
}

#[rstest]
fn corrupt_or_stale_reload_keeps_the_last_good_snapshot() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let active = AssetPublisher::new(root.path().into())
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	let store = ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap();
	// Act
	let stale = ManifestSnapshot::load(
		root.path(),
		&SnapshotOptions::production().expected_build_id("f".repeat(64)),
	);
	std::fs::write(root.path().join("manifest.json"), b"incomplete manifest").unwrap();
	let reload = store.reload();
	// Assert
	assert!(stale.unwrap_err().to_string().contains("stale"));
	assert!(reload.is_err());
	assert_eq!(
		store.active().manifest().build_id,
		active.manifest().build_id
	);
}

#[rstest]
#[case(b"changed".as_slice())]
#[case(b"".as_slice())]
fn tampered_or_truncated_assets_are_not_a_ready_generation(#[case] bytes: &[u8]) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	std::fs::write(root.path().join(&snapshot.manifest().paths["a.txt"]), bytes).unwrap();
	// Act
	let loaded = ManifestSnapshot::load(root.path(), &SnapshotOptions::production());
	// Assert
	assert!(loaded.unwrap_err().to_string().contains("a.txt"));
	assert!(snapshot.read_asset("a.txt").is_err());
}

#[rstest]
fn competing_defaults_require_explicit_manifest_selection() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	AssetPublisher::new(root.path().into())
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	std::fs::write(
		root.path().join("staticfiles.json"),
		br#"{"paths":{"a.txt":"a.old.txt"}}"#,
	)
	.unwrap();
	// Act
	let ambiguous = ManifestSnapshot::load(root.path(), &SnapshotOptions::production());
	let explicit = ManifestSnapshot::load(
		root.path(),
		&SnapshotOptions::production().manifest_path("manifest.json".into()),
	);
	// Assert
	assert!(ambiguous.is_err());
	assert_eq!(explicit.unwrap().read_asset("a.txt").unwrap(), b"one");
}

#[rstest]
fn abandoned_staging_is_ignored_but_malformed_retained_output_fails_reload() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	let staging = tempfile::Builder::new()
		.prefix(".asset-staging-")
		.tempdir_in(root.path())
		.unwrap();
	std::fs::write(staging.path().join("manifest.json"), b"unfinished").unwrap();
	let store = ManifestStore::open(root.path().into(), SnapshotOptions::production()).unwrap();
	let retained = root.path().join("builds").join("d".repeat(64));
	std::fs::create_dir_all(&retained).unwrap();
	std::fs::write(retained.join("manifest.json"), b"unfinished").unwrap();
	// Act
	let result = store.reload();
	// Assert
	assert!(result.is_err());
	assert_eq!(
		store.active().manifest().build_id,
		snapshot.manifest().build_id
	);
	assert_eq!(store.active().read_asset("a.txt").unwrap(), b"one");
}
