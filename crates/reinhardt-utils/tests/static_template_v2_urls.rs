#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::ManifestStaticFilesStorage;
use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, encode_manifest,
};
use reinhardt_utils::staticfiles::template_integration::TemplateStaticConfig;
use rstest::rstest;

#[rstest]
#[case("logo #1.svg", "logo%20%231.svg")]
#[case("100%.svg", "100%25.svg")]
#[case("日本.svg", "%E6%97%A5%E6%9C%AC.svg")]
#[tokio::test]
async fn v2_template_paths_are_encoded_once(#[case] logical: &str, #[case] encoded: &str) {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes(logical, b"<svg/>".to_vec()))
		.unwrap();
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	let manifest = packed.manifest();
	let (parent, _) = manifest.paths[logical].rsplit_once('/').unwrap();
	let storage = ManifestStaticFilesStorage::new(root.path().to_path_buf(), "/static/");
	std::fs::write(
		root.path().join(&storage.manifest_name),
		encode_manifest(manifest).unwrap(),
	)
	.unwrap();

	// Act
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// Assert
	assert_eq!(
		config.resolve_url(logical),
		format!("/static/{parent}/{encoded}")
	);
}

#[rstest]
#[tokio::test]
async fn legacy_template_urls_are_not_double_encoded() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let storage = ManifestStaticFilesStorage::new(root.path().to_path_buf(), "/static/");
	std::fs::write(
		root.path().join(&storage.manifest_name),
		br#"{"version":"1.0","paths":{"logo.svg":"images/logo%20%231.svg"}}"#,
	)
	.unwrap();

	// Act
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// Assert
	assert_eq!(
		config.resolve_url("logo.svg"),
		"/static/images/logo%20%231.svg"
	);
}
