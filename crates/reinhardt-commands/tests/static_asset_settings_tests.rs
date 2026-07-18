use std::path::PathBuf;

use reinhardt_commands::StaticAssetSettings;
use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::sources::DefaultSource;
use rstest::rstest;
use serde_json::json;

#[rstest]
#[case("static", "/assets/", "dist")]
#[case("static_files", "/typed/", "typed-static")]
fn modern_static_sections_are_resolved(
	#[case] section: &str,
	#[case] url: &str,
	#[case] root: &str,
) {
	let settings = SettingsBuilder::new()
		.add_source(DefaultSource::new().with_value(section, json!({ "url": url, "root": root })))
		.build()
		.unwrap();

	let resolved = StaticAssetSettings::from_merged(&settings, PathBuf::from("/project").as_path());

	assert_eq!(resolved.static_url, url);
	assert_eq!(resolved.static_root, PathBuf::from("/project").join(root));
}

#[rstest]
fn legacy_flat_settings_and_directories_are_preserved() {
	let settings = SettingsBuilder::new()
		.add_source(
			DefaultSource::new()
				.with_value("static_url", json!("/legacy/"))
				.with_value("static_root", json!("/srv/static"))
				.with_value("staticfiles_dirs", json!(["assets", "vendor"])),
		)
		.build()
		.unwrap();

	let resolved = StaticAssetSettings::from_merged(&settings, PathBuf::from("/project").as_path());

	assert_eq!(resolved.static_url, "/legacy/");
	assert_eq!(resolved.static_root, PathBuf::from("/srv/static"));
	assert_eq!(
		resolved.staticfiles_dirs,
		vec![PathBuf::from("assets"), PathBuf::from("vendor")]
	);
}

#[rstest]
fn defaults_use_static_url_and_project_staticfiles() {
	let settings = SettingsBuilder::new().build().unwrap();

	let resolved = StaticAssetSettings::from_merged(&settings, PathBuf::from("/project").as_path());

	assert_eq!(resolved.static_url, "/static/");
	assert_eq!(resolved.static_root, PathBuf::from("/project/staticfiles"));
	assert!(resolved.staticfiles_dirs.is_empty());
}
