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
	storage.load_manifest().await.unwrap();

	// Act
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// Assert
	assert_eq!(
		config.resolve_url(logical),
		format!("/static/{parent}/{encoded}")
	);
	assert_eq!(storage.url(logical), format!("/static/{parent}/{encoded}"));
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
	storage.load_manifest().await.unwrap();

	// Act
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// Assert
	assert_eq!(
		config.resolve_url("logo.svg"),
		"/static/images/logo%20%231.svg"
	);
	assert_eq!(storage.url("logo.svg"), "/static/images/logo%20%231.svg");
}

#[rstest]
#[tokio::test]
async fn legacy_storage_rejects_writes_after_loading_a_v2_manifest() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes("logo.svg", b"<svg/>".to_vec()))
		.unwrap();
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	let storage = ManifestStaticFilesStorage::new(root.path(), "/static/");
	let manifest_path = root.path().join(&storage.manifest_name);
	let original = encode_manifest(packed.manifest()).unwrap();
	std::fs::write(&manifest_path, &original).unwrap();
	storage.load_manifest().await.unwrap();
	let mut files = std::collections::HashMap::new();
	files.insert("new.css".into(), b"body{}".to_vec());
	// Act
	let error = storage.save_with_dependencies(files).await.unwrap_err();
	// Assert
	assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
	assert_eq!(std::fs::read(&manifest_path).unwrap(), original);
	assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
