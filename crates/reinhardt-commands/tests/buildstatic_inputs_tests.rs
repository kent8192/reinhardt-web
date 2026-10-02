use reinhardt_commands::buildstatic::{
	BuildStaticCommand, BuildStaticRequest, BuildStaticResult, PagesSource,
};
use reinhardt_commands::{CollectStaticCommand, CollectStaticOptions, StaticAssetSettings};
use reinhardt_utils::staticfiles::StaticFilesConfig;
use rstest::rstest;

#[rstest]
#[case("/static/")]
#[case("/console/static/")]
#[case("https://cdn.example.test/console/static/")]
fn legacy_hashed_references_and_configured_prefixes_resolve(#[case] prefix: &str) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let source = root.path().join("source");
	std::fs::create_dir_all(source.join("styles")).unwrap();
	std::fs::create_dir_all(source.join("images")).unwrap();
	std::fs::write(
		source.join("styles/main.aaa.css"),
		b"body{mask:url('../images/logo.bbb.svg#icon')}",
	)
	.unwrap();
	std::fs::write(source.join("images/logo.bbb.svg"), b"<svg/>").unwrap();
	std::fs::write(source.join("manifest.json"), br#"{"paths":{"styles/main.css":"styles/main.aaa.css","images/logo.svg":"images/logo.bbb.svg"}}"#).unwrap();
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: prefix.into(),
		static_root: root.path().join("out"),
		staticfiles_dirs: Vec::new(),
	});
	let mut request = BuildStaticRequest::new(root.path().into());
	request.static_manifest = Some("source/manifest.json".into());
	// Act
	let BuildStaticResult::Published(snapshot) = command.execute(request).unwrap() else {
		panic!("publication expected")
	};
	// Assert
	assert_eq!(
		snapshot.manifest().assets["styles/main.css"].dependencies,
		["images/logo.svg"]
	);
	assert!(
		String::from_utf8(snapshot.read_asset("styles/main.css").unwrap())
			.unwrap()
			.contains("../../vectors/logo.svg#icon")
	);
	assert!(
		snapshot
			.url_snapshot(prefix)
			.unwrap()
			.resolve("styles/main.css")
			.unwrap()
			.starts_with(prefix)
	);
}

#[rstest]
fn overlapping_source_and_destination_excludes_publication_metadata_on_rebuild() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	std::fs::write(root.path().join("logo.svg"), b"<svg/>").unwrap();
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: root.path().into(),
		staticfiles_dirs: vec![root.path().into()],
	});
	let BuildStaticResult::Published(first) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected")
	};
	// Act
	let BuildStaticResult::Published(second) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected")
	};
	// Assert
	assert_eq!(second.manifest().build_id, first.manifest().build_id);
	assert_eq!(second.manifest().assets, first.manifest().assets);
	assert_eq!(second.read_asset("logo.svg").unwrap(), b"<svg/>");
	// Registered applications may contribute assets in addition to this fixture.
	for logical in second.manifest().assets.keys() {
		assert!(!logical.starts_with("builds/"), "{logical}");
		assert!(
			!["manifest.json", "staticfiles.json", ".publication.lock"].contains(&logical.as_str()),
			"{logical}"
		);
	}
}

#[rstest]
#[case("export const wasm=new URL('absent.wasm',import.meta.url);", "missing")]
#[case("export const wasm=new URL(file,import.meta.url);", "computed")]
#[case("export const wasm=42;", "exactly one")]
#[case(
	"import './missing.js'; export const wasm=new URL('app.wasm',import.meta.url);",
	"missing logical dependency"
)]
fn invalid_prebuilt_bundles_do_not_activate(#[case] javascript: &str, #[case] diagnostic: &str) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let source = root.path().join("pages");
	std::fs::create_dir(&source).unwrap();
	std::fs::write(source.join("app.js"), javascript).unwrap();
	std::fs::write(source.join("app.wasm"), b"\0asm\x01\0\0\0").unwrap();
	let output = root.path().join("out");
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: output.clone(),
		staticfiles_dirs: Vec::new(),
	});
	let mut request = BuildStaticRequest::new(root.path().into());
	request.pages = Some(PagesSource::Directory {
		directory: source,
		entry: "app.js".into(),
	});
	// Act
	let error = command.execute(request).unwrap_err();
	// Assert
	assert!(error.to_string().contains(diagnostic), "{error}");
	assert!(!output.exists());
}

#[rstest]
fn conflicting_sources_are_reported_by_preview_and_rejected_before_activation() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let a = root.path().join("a");
	let b = root.path().join("b");
	for source in [&a, &b] {
		std::fs::create_dir(source).unwrap();
		std::fs::write(source.join("main.css"), b"body{}").unwrap();
	}
	let output = root.path().join("out");
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: output.clone(),
		staticfiles_dirs: vec![a, b],
	});
	let mut request = BuildStaticRequest::new(root.path().into());
	request.dry_run = true;
	// Act
	let BuildStaticResult::DryRun(preview) = command.execute(request.clone()).unwrap() else {
		panic!("preview expected")
	};
	request.dry_run = false;
	let error = command.execute(request).unwrap_err();
	// Assert
	assert_eq!(preview.conflicts.len(), 1);
	assert!(error.to_string().contains("collision"));
	assert!(!output.exists());
}

#[rstest]
fn reimported_v2_assets_keep_the_original_namespace_and_identity() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let source = root.path().join("source");
	std::fs::create_dir_all(source.join("styles")).unwrap();
	std::fs::create_dir_all(source.join("images")).unwrap();
	std::fs::write(
		source.join("styles/main.css"),
		b"body{mask:url('../images/logo.svg')}\n/*# sourceMappingURL=main.css.map */",
	)
	.unwrap();
	std::fs::write(source.join("images/logo.svg"), b"<svg/>").unwrap();
	std::fs::write(source.join("styles/main.css.map"), br#"{"version":3,"file":"main.css","sources":["main.scss"],"sourcesContent":["body{}"],"names":[],"mappings":"AAAA"}"#).unwrap();
	let settings = StaticAssetSettings {
		static_url: "/console/static/".into(),
		static_root: root.path().join("published"),
		staticfiles_dirs: vec![source],
	};
	let command = BuildStaticCommand::new(settings.clone());
	let BuildStaticResult::Published(first) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected")
	};
	let mut request = BuildStaticRequest::new(root.path().into());
	request.static_manifest = Some(settings.static_root.join("manifest.json"));
	// Act: import the current publication into the same root.
	let BuildStaticResult::Published(second) = command.execute(request).unwrap() else {
		panic!("publication expected")
	};
	// Assert
	assert_eq!(first.manifest().build_id, second.manifest().build_id);
	assert_eq!(
		first.read_asset("styles/main.css").unwrap(),
		second.read_asset("styles/main.css").unwrap()
	);
	assert_eq!(
		first.read_asset("styles/main.css.map").unwrap(),
		second.read_asset("styles/main.css.map").unwrap()
	);
}

#[rstest]
fn competing_legacy_manifest_is_diagnosed_without_being_changed() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let legacy = root.path().join("staticfiles.json");
	std::fs::write(&legacy, br#"{"paths":{}}"#).unwrap();
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: root.path().into(),
		staticfiles_dirs: Vec::new(),
	});
	// Act
	let result = command.execute(BuildStaticRequest::new(root.path().into()));
	// Assert
	assert!(result.unwrap_err().to_string().contains("staticfiles.json"));
	assert_eq!(std::fs::read(&legacy).unwrap(), br#"{"paths":{}}"#);
	assert!(!root.path().join("manifest.json").exists());
}

#[rstest]
fn materialized_pages_and_legacy_styles_publish_without_project_compilation() {
	// Arrange: no Cargo.toml or compiler configuration exists in this project.
	let root = tempfile::tempdir().unwrap();
	let collected = root.path().join("collected");
	let pages = root.path().join("wasm-dist");
	std::fs::create_dir_all(collected.join("__reinhardt__")).unwrap();
	std::fs::create_dir_all(pages.join("snippets")).unwrap();
	std::fs::write(
		collected.join("__reinhardt__/components.hash.css"),
		b".component{color:rgb(1,2,3)}",
	)
	.unwrap();
	std::fs::write(collected.join("manifest.json"), br#"{"version":"1.0","paths":{"__reinhardt__/components.css":"__reinhardt__/components.hash.css"}}"#).unwrap();
	std::fs::write(pages.join("app.js"), b"import './snippets/tool.js'; export const wasm=new URL('actual-target.wasm',import.meta.url);").unwrap();
	std::fs::write(pages.join("actual-target.wasm"), b"\0asm\x01\0\0\0").unwrap();
	std::fs::write(pages.join("snippets/tool.js"), b"export const value=1;").unwrap();
	let output = root.path().join("configured-static");
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_url: "/console/static/".into(),
		static_root: output.clone(),
		staticfiles_dirs: Vec::new(),
	});
	let mut request = BuildStaticRequest::new(root.path().into());
	request.static_manifest = Some(collected.join("manifest.json"));
	request.pages = Some(PagesSource::Directory {
		directory: pages,
		entry: "app.js".into(),
	});
	// Act
	let BuildStaticResult::Published(snapshot) = command.execute(request).unwrap() else {
		panic!("publication expected")
	};
	// Assert
	assert!(snapshot.manifest().paths["app.js"].ends_with("/pages/app.js"));
	assert!(snapshot.manifest().paths["actual-target.wasm"].ends_with("/pages/actual-target.wasm"));
	assert!(snapshot.manifest().paths["snippets/tool.js"].ends_with("/pages/snippets/tool.js"));
	assert!(
		snapshot.manifest().paths["__reinhardt__/components.css"]
			.ends_with("/css/__reinhardt__/components.css")
	);
	assert_eq!(
		snapshot.manifest().entrypoints["default"].wasm,
		"actual-target.wasm"
	);
	assert_eq!(
		snapshot.manifest().entrypoints["default"].styles,
		["__reinhardt__/components.css"]
	);
	let mut reimport = BuildStaticRequest::new(root.path().into());
	reimport.static_manifest = Some(output.join("manifest.json"));
	let BuildStaticResult::Published(republished) = command.execute(reimport).unwrap() else {
		panic!("republication expected")
	};
	assert_eq!(
		republished.manifest().build_id,
		snapshot.manifest().build_id
	);
	assert_eq!(
		republished
			.read_asset("__reinhardt__/pages-loader.js")
			.unwrap(),
		snapshot
			.read_asset("__reinhardt__/pages-loader.js")
			.unwrap()
	);
}

#[rstest]
fn legacy_collectstatic_refuses_to_clear_a_v2_publication() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let settings = StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: root.path().into(),
		staticfiles_dirs: Vec::new(),
	};
	BuildStaticCommand::new(settings)
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap();
	let before = std::fs::read(root.path().join("manifest.json")).unwrap();
	let config = StaticFilesConfig {
		static_root: root.path().into(),
		..Default::default()
	};
	let options = CollectStaticOptions {
		clear: true,
		no_input: true,
		interactive: false,
		..Default::default()
	};
	// Act
	let result = CollectStaticCommand::new(config, options).execute();
	// Assert
	assert!(result.unwrap_err().to_string().contains("buildstatic"));
	assert_eq!(
		std::fs::read(root.path().join("manifest.json")).unwrap(),
		before
	);
}
