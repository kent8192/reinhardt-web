use clap::Parser;
use reinhardt_commands::StaticAssetSettings;
use reinhardt_commands::buildstatic::{
	BuildStaticArgs, BuildStaticCommand, BuildStaticRequest, BuildStaticResult,
};
use rstest::rstest;

#[rstest]
#[case(&["buildstatic", "--pages", "--package", "dashboard", "--release"])]
#[case(&["buildstatic", "--static-manifest", "/build/collected/manifest.json", "--pages-dir", "/build/wasm-dist", "--pages-entry", "dashboard.js"])]
#[case(&["buildstatic", "--mode", "development", "--pages", "--package", "browser-fixture"])]
#[case(&["buildstatic", "--dry-run"])]
#[case(&["buildstatic", "--pages", "--profile", "production"])]
fn supported_buildstatic_cli_forms_parse(#[case] args: &[&str]) {
	// Arrange & Act
	let parsed = BuildStaticArgs::try_parse_from(args);
	// Assert
	assert!(parsed.is_ok(), "{parsed:?}");
}

#[rstest]
#[case(&["buildstatic", "--pages", "--pages-dir", "/bundle", "--pages-entry", "app.js"])]
#[case(&["buildstatic", "--pages-dir", "/bundle"])]
#[case(&["buildstatic", "--pages-entry", "app.js"])]
#[case(&["buildstatic", "--mode", "automatic"])]
#[case(&["buildstatic", "--release", "--profile", "production"])]
fn incompatible_buildstatic_cli_forms_fail(#[case] args: &[&str]) {
	// Arrange & Act
	let parsed = BuildStaticArgs::try_parse_from(args);
	// Assert
	assert!(parsed.is_err());
}

#[rstest]
fn ordinary_assets_use_configured_static_root_without_a_pages_build() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let input = root.path().join("sources");
	std::fs::create_dir_all(&input).unwrap();
	std::fs::write(input.join("logo.svg"), b"<svg/>").unwrap();
	std::fs::write(input.join("main.css"), b"body{mask:url('logo.svg')}").unwrap();
	let output = root.path().join("configured-static");
	let settings = StaticAssetSettings {
		static_url: "/console/static/".into(),
		static_root: output.clone(),
		staticfiles_dirs: vec![input],
	};
	let command = BuildStaticCommand::new(settings);
	// Act
	let result = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap();
	// Assert
	let BuildStaticResult::Published(snapshot) = result else {
		panic!("publication expected")
	};
	assert!(output.join("manifest.json").is_file());
	assert!(snapshot.manifest().paths["logo.svg"].ends_with("/vectors/logo.svg"));
	assert!(
		snapshot
			.url_snapshot("/console/static/")
			.unwrap()
			.resolve("main.css")
			.unwrap()
			.starts_with("/console/static/builds/")
	);
}

#[rstest]
fn dry_run_does_not_create_output_or_invoke_a_compiler() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let output = root.path().join("configured-static");
	let settings = StaticAssetSettings {
		static_url: "/static/".into(),
		static_root: output.clone(),
		staticfiles_dirs: Vec::new(),
	};
	let command = BuildStaticCommand::new(settings);
	let mut request = BuildStaticRequest::new(root.path().into());
	request.dry_run = true;
	request.pages = Some(reinhardt_commands::buildstatic::PagesSource::Build);
	// Act
	let result = command.execute(request).unwrap();
	// Assert
	let BuildStaticResult::DryRun(preview) = result else {
		panic!("preview expected")
	};
	assert!(!preview.pending_checks.is_empty());
	assert!(!output.exists());
	assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[rstest]
fn encoded_static_prefix_is_decoded_once_for_source_reference_lookup() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let source = root.path().join("source");
	std::fs::create_dir(&source).unwrap();
	std::fs::write(source.join("logo.svg"), b"<svg/>").unwrap();
	std::fs::write(
		source.join("main.css"),
		b"body{mask:url('/console/%E6%97%A5/static/logo.svg')}",
	)
	.unwrap();
	let command = BuildStaticCommand::new(StaticAssetSettings {
		static_root: root.path().join("output"),
		static_url: "/console/%E6%97%A5/static/".into(),
		staticfiles_dirs: vec![source],
	});
	// Act
	let BuildStaticResult::Published(snapshot) = command
		.execute(BuildStaticRequest::new(root.path().into()))
		.unwrap()
	else {
		panic!("publication expected")
	};
	// Assert
	assert_eq!(
		snapshot.manifest().assets["main.css"].dependencies,
		["logo.svg"]
	);
	assert!(
		snapshot
			.url_snapshot("/console/%E6%97%A5/static/")
			.unwrap()
			.resolve("logo.svg")
			.unwrap()
			.starts_with("/console/%E6%97%A5/static/builds/")
	);
}
