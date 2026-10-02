use reinhardt_commands::StaticAssetSettings;
use reinhardt_commands::buildstatic::{BuildStaticCommand, BuildStaticRequest, BuildStaticResult};
use rstest::rstest;

#[rstest]
fn publication_names_in_independent_sources_remain_publishable() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let source = root.path().join("source");
	std::fs::create_dir_all(source.join("builds")).unwrap();
	for (name, content) in [
		("manifest.json", r#"{"name":"Example PWA"}"#),
		("staticfiles.json", r#"{"example":"application data"}"#),
		("builds/history.txt", "application build history"),
		(
			"index.html",
			r#"<html><head><link rel="manifest" href="manifest.json"></head></html>"#,
		),
	] {
		std::fs::write(source.join(name), content).unwrap();
	}
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: root.path().join("output"),
		staticfiles_dirs: vec![source],
	});

	// Act
	let BuildStaticResult::Published(snapshot) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected");
	};

	// Assert
	for logical in ["manifest.json", "staticfiles.json", "builds/history.txt"] {
		assert!(snapshot.manifest().paths.contains_key(logical));
	}
	assert_eq!(
		snapshot.manifest().assets["index.html"].dependencies,
		["manifest.json"]
	);
	assert_eq!(
		snapshot.read_asset("manifest.json").unwrap(),
		br#"{"name":"Example PWA"}"#
	);
}

#[rstest]
#[case(false)]
#[case(true)]
fn repeated_builds_do_not_collect_their_own_publications(#[case] nested: bool) {
	// Arrange: the output is either the input root or a subtree of it.
	let root = tempfile::tempdir().unwrap();
	let source = root.path().join("source");
	std::fs::create_dir_all(&source).unwrap();
	std::fs::write(source.join("logo.svg"), b"<svg/>").unwrap();
	let output = if nested {
		source.join("published")
	} else {
		source.clone()
	};
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: output.clone(),
		staticfiles_dirs: vec![source],
	});
	let BuildStaticResult::Published(first) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected");
	};
	std::fs::create_dir_all(output.join(".asset-abandoned")).unwrap();
	std::fs::write(output.join(".asset-abandoned/partial.txt"), "incomplete").unwrap();
	std::fs::create_dir_all(output.join(".manifest-abandoned")).unwrap();
	std::fs::write(output.join(".manifest-abandoned/partial.txt"), "incomplete").unwrap();

	// Act
	let BuildStaticResult::Published(second) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected");
	};

	// Assert
	assert_eq!(second.manifest().build_id, first.manifest().build_id);
	assert_eq!(second.manifest().paths, first.manifest().paths);
	assert!(second.manifest().paths.contains_key("logo.svg"));
}
