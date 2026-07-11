use std::fs;

use reinhardt_commands::{CollectStaticCommand, CollectStaticOptions, VirtualStaticAsset};
use reinhardt_utils::staticfiles::StaticFilesConfig;
use rstest::rstest;

#[rstest]
#[case(false)]
#[case(true)]
fn virtual_component_styles_are_hashed_and_written_as_regular_files(#[case] link: bool) {
	let directory = tempfile::tempdir().expect("create destination");
	let destination = directory.path().join("static-root");
	let config = StaticFilesConfig {
		static_root: destination.clone(),
		static_url: "/static/".to_string(),
		staticfiles_dirs: Vec::new(),
		media_url: None,
	};
	let options = CollectStaticOptions {
		link,
		enable_hashing: true,
		verbosity: 0,
		..CollectStaticOptions::default()
	};
	let mut command = CollectStaticCommand::new(config, options);
	command.add_virtual_asset(VirtualStaticAsset {
		logical_path: "__reinhardt__/components.css".to_string(),
		bytes: b".card { color: red; }\n".to_vec(),
	});

	let stats = command.execute().expect("collect virtual component CSS");

	assert_eq!(stats.copied, 1);
	let manifest: serde_json::Value =
		serde_json::from_slice(&fs::read(destination.join("manifest.json")).unwrap()).unwrap();
	let hashed = manifest["paths"]["__reinhardt__/components.css"]
		.as_str()
		.expect("hashed mapping");
	let output = destination.join(hashed);
	assert_eq!(fs::read(&output).unwrap(), b".card { color: red; }\n");
	assert!(!output.symlink_metadata().unwrap().file_type().is_symlink());
	assert!(!destination.join("__reinhardt__/components.css").exists());
}

#[rstest]
fn physical_sources_cannot_claim_the_reserved_namespace() {
	let directory = tempfile::tempdir().expect("create source tree");
	let source = directory.path().join("source");
	fs::create_dir_all(source.join("__reinhardt__")).unwrap();
	fs::write(source.join("__reinhardt__/components.css"), "stale").unwrap();
	let destination = directory.path().join("destination");
	let config = StaticFilesConfig {
		static_root: destination.clone(),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![source],
		media_url: None,
	};
	let mut command = CollectStaticCommand::new(config, CollectStaticOptions::default());

	let error = command
		.execute()
		.expect_err("reserved source must fail preflight");

	assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
	assert!(!destination.exists());
}
