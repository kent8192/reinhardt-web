#![cfg(all(feature = "asset-publication", unix))]

use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetPublisher, ManifestSnapshot, SnapshotOptions,
};
use rstest::rstest;

fn prepare(
	bytes: &[u8],
	mode: AssetMode,
) -> reinhardt_utils::staticfiles::publication::PreparedGeneration {
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes("a.txt", bytes.to_vec()))
		.unwrap();
	pipeline.prepare(mode).unwrap()
}

#[rstest]
fn identical_publication_reuses_complete_generation_and_rejects_mode_switch() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let publisher = AssetPublisher::new(root.path().into());
	let a = publisher
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	// Act
	let same = publisher
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	let development = publisher.publish(prepare(b"one", AssetMode::Development));
	// Assert
	assert_eq!(same.manifest().build_id, a.manifest().build_id);
	assert!(
		development
			.unwrap_err()
			.to_string()
			.contains("mode mismatch")
	);
	assert_eq!(
		ManifestSnapshot::load(root.path(), &SnapshotOptions::production())
			.unwrap()
			.manifest()
			.build_id,
		a.manifest().build_id
	);
}

#[rstest]
fn concurrent_publishers_leave_a_complete_active_generation() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let one = prepare(b"one", AssetMode::Production);
	let two = prepare(b"two", AssetMode::Production);
	let ids = [
		one.manifest().build_id.clone(),
		two.manifest().build_id.clone(),
	];
	let barrier = std::sync::Barrier::new(2);
	// Act
	std::thread::scope(|scope| {
		let a = scope.spawn(|| {
			barrier.wait();
			AssetPublisher::new(root.path().into())
				.publish(one)
				.unwrap();
		});
		let b = scope.spawn(|| {
			barrier.wait();
			AssetPublisher::new(root.path().into())
				.publish(two)
				.unwrap();
		});
		a.join().unwrap();
		b.join().unwrap();
	});
	let loaded = ManifestSnapshot::load(root.path(), &SnapshotOptions::production()).unwrap();
	// Assert
	assert!(ids.contains(&loaded.manifest().build_id));
	for id in ids {
		assert!(
			root.path()
				.join(format!("builds/{id}/manifest.json"))
				.is_file()
		);
	}
}

#[rstest]
fn conflicting_existing_generation_cannot_replace_the_active_pointer() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let publisher = AssetPublisher::new(root.path().into());
	let a = publisher
		.publish(prepare(b"one", AssetMode::Production))
		.unwrap();
	let b = prepare(b"two", AssetMode::Production);
	let target = root.path().join(&b.manifest().paths["a.txt"]);
	std::fs::create_dir_all(target.parent().unwrap()).unwrap();
	std::fs::write(target, b"wrong").unwrap();
	std::fs::write(
		root.path()
			.join(format!("builds/{}/manifest.json", b.manifest().build_id)),
		b.manifest_bytes(),
	)
	.unwrap();
	// Act
	let result = publisher.publish(b);
	// Assert
	assert!(result.unwrap_err().to_string().contains("a.txt"));
	assert_eq!(
		ManifestSnapshot::load(root.path(), &SnapshotOptions::production())
			.unwrap()
			.manifest()
			.build_id,
		a.manifest().build_id
	);
}

#[rstest]
fn publication_uses_captured_bytes_when_source_overlaps_output_root() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	std::fs::write(root.path().join("a.txt"), b"captured").unwrap();
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::from_directory(
			root.path().into(),
			"a.txt",
			"a.txt",
		))
		.unwrap();
	let prepared = pipeline.prepare(AssetMode::Production).unwrap();
	std::fs::write(root.path().join("a.txt"), b"changed after capture").unwrap();
	// Act
	let snapshot = AssetPublisher::new(root.path().into())
		.publish(prepared)
		.unwrap();
	// Assert
	assert_eq!(snapshot.read_asset("a.txt").unwrap(), b"captured");
	assert_eq!(
		std::fs::read(root.path().join("a.txt")).unwrap(),
		b"changed after capture"
	);
}

#[rstest]
#[case(false)]
#[case(true)]
fn corrupt_retained_generation_cannot_activate_new_output(#[case] missing: bool) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let publisher = AssetPublisher::new(root.path().into());
	let retained = publisher
		.publish(prepare(b"old", AssetMode::Production))
		.unwrap();
	let active = publisher
		.publish(prepare(b"current", AssetMode::Production))
		.unwrap();
	let pointer = std::fs::read(root.path().join("manifest.json")).unwrap();
	let path = root.path().join(&retained.manifest().paths["a.txt"]);
	if missing {
		std::fs::remove_file(path).unwrap();
	} else {
		std::fs::write(path, b"bad").unwrap();
	}
	let next = prepare(b"next", AssetMode::Production);
	let next_dir = root
		.path()
		.join(format!("builds/{}", next.manifest().build_id));
	// Act
	let result = publisher.publish(next);
	// Assert
	assert!(result.is_err());
	assert_eq!(
		std::fs::read(root.path().join("manifest.json")).unwrap(),
		pointer
	);
	assert_eq!(
		ManifestSnapshot::load(root.path(), &SnapshotOptions::production())
			.unwrap()
			.manifest()
			.build_id,
		active.manifest().build_id
	);
	assert!(!next_dir.exists());
}

#[rstest]
#[case(false)]
#[case(true)]
fn competing_legacy_manifest_is_rejected_before_activation(#[case] existing: bool) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let publisher = AssetPublisher::new(root.path().into());
	let previous = existing.then(|| {
		publisher
			.publish(prepare(b"old", AssetMode::Production))
			.unwrap()
	});
	let legacy = br#"{"paths":{}}"#;
	std::fs::write(root.path().join("staticfiles.json"), legacy).unwrap();
	let next = prepare(b"new", AssetMode::Production);
	let next_id = next.manifest().build_id.clone();
	// Act
	let error = publisher.publish(next).unwrap_err();
	// Assert
	assert!(
		error
			.to_string()
			.contains("competing legacy staticfiles.json"),
		"{error}"
	);
	assert_eq!(
		std::fs::read(root.path().join("staticfiles.json")).unwrap(),
		legacy
	);
	assert!(!root.path().join("builds").join(next_id).exists());
	if let Some(previous) = previous {
		assert_eq!(
			std::fs::read(root.path().join("manifest.json")).unwrap(),
			previous.manifest_bytes()
		);
	} else {
		assert!(!root.path().join("manifest.json").exists());
		assert!(!root.path().join("builds").exists());
	}
}
