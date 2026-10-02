use reinhardt_utils::staticfiles::publication::{
	DecodedAssetManifest, decode_manifest, discover_manifest,
};
use rstest::rstest;

#[rstest]
#[case(br#"{"version":"1.0","paths":{"x.css":"x.123.css"}}"#)]
#[case(br#"{"paths":{"x.css":"x.123.css"}}"#)]
#[case(br#"{"version":"1.0","files":{"x.css":"x.123.css"}}"#)]
#[case(br#"{"x.css":"x.123.css"}"#)]
fn legacy_paths_are_preserved_without_invented_identity(#[case] input: &[u8]) {
	// Arrange & Act
	let decoded = decode_manifest(input).unwrap();
	// Assert
	let DecodedAssetManifest::Legacy(legacy) = decoded else {
		panic!("legacy input must not acquire generation guarantees");
	};
	assert_eq!(
		legacy.paths.get("x.css").map(String::as_str),
		Some("x.123.css")
	);
}

#[rstest]
#[case(br#"{"version":"9.0","paths":{}}"#)]
#[case(br#"{"paths":{"x.css":3}}"#)]
#[case(br#"{"paths":{"x.css":"one.css","x.css":"two.css"}}"#)]
#[case(br#"{"paths":{"x.css":"../outside.css"}}"#)]
#[case(br#"{"paths":{},"files":{}}"#)]
fn malformed_or_ambiguous_manifests_fail(#[case] input: &[u8]) {
	// Arrange & Act
	let decoded = decode_manifest(input);
	// Assert
	assert!(
		decoded.is_err(),
		"must reject {}",
		String::from_utf8_lossy(input)
	);
}

#[rstest]
fn competing_default_manifests_require_explicit_selection() {
	// Arrange
	let root = tempfile::tempdir().unwrap();
	let current = root.path().join("manifest.json");
	let legacy = root.path().join("staticfiles.json");
	std::fs::write(&current, b"{}").unwrap();
	std::fs::write(&legacy, b"{}").unwrap();
	// Act
	let error = discover_manifest(root.path(), None).unwrap_err();
	// Assert
	let diagnostic = error.to_string();
	assert!(diagnostic.contains("manifest.json"));
	assert!(diagnostic.contains("staticfiles.json"));
	assert_eq!(
		discover_manifest(root.path(), Some(&legacy)).unwrap(),
		legacy
	);
}

fn manifest_v2() -> serde_json::Value {
	let id = "a".repeat(64);
	serde_json::json!({
		"version": "2.0", "mode": "production", "build_id": id,
		"paths": {"app.js": format!("builds/{id}/pages/app.js"), "app_bg.wasm": format!("builds/{id}/pages/app_bg.wasm")},
		"assets": {
			"app.js": {"category":"pages", "role":"asset", "mime":"text/javascript", "size":10, "sha256":"b".repeat(64), "dependencies":["app_bg.wasm"], "variants":[]},
			"app_bg.wasm": {"category":"pages", "role":"asset", "mime":"application/wasm", "size":8, "sha256":"c".repeat(64), "dependencies":[], "variants":[]}
		},
		"entrypoints": {"app": {"javascript":"app.js", "wasm":"app_bg.wasm", "styles":[], "document":null}},
		"pipeline": [{"id":"classifier", "version":"1", "options":{}}]
	})
}

#[rstest]
fn complete_v2_metadata_is_decoded_without_losing_entrypoint_relationships() {
	// Arrange
	let bytes = serde_json::to_vec(&manifest_v2()).unwrap();
	// Act
	let decoded = decode_manifest(&bytes).unwrap();
	// Assert
	let DecodedAssetManifest::V2(manifest) = decoded else {
		panic!("expected v2")
	};
	assert_eq!(manifest.entrypoints["app"].wasm, "app_bg.wasm");
	assert_eq!(manifest.assets["app.js"].dependencies, ["app_bg.wasm"]);
}

#[rstest]
#[case("/version", serde_json::json!("3.0"))]
#[case("/mode", serde_json::json!("automatic"))]
#[case("/build_id", serde_json::json!("short"))]
#[case("/paths/app.js", serde_json::json!("../app.js"))]
#[case("/assets/app.js/sha256", serde_json::json!("wrong"))]
#[case("/assets/app.js/size", serde_json::json!(-1))]
#[case("/assets/app.js/dependencies", serde_json::json!(["missing.wasm"]))]
#[case("/entrypoints/app/wasm", serde_json::json!("app.js"))]
#[case("/entrypoints/app/styles", serde_json::json!(["app.js"]))]
#[case("/entrypoints/app/document", serde_json::json!("app.js"))]
#[case("/assets/app.js/variants", serde_json::json!(["app_bg.wasm"]))]
fn invalid_v2_metadata_is_rejected(#[case] pointer: &str, #[case] replacement: serde_json::Value) {
	// Arrange
	let mut value = manifest_v2();
	*value.pointer_mut(pointer).unwrap() = replacement;
	// Act
	let result = decode_manifest(&serde_json::to_vec(&value).unwrap());
	// Assert
	assert!(result.is_err(), "must reject mutation of {pointer}");
}

#[rstest]
#[case("text/")]
#[case("text/\0plain")]
#[case("invalid type/plain")]
fn ordinary_asset_mime_must_be_a_valid_media_type(#[case] mime: &str) {
	// Arrange
	let mut value = manifest_v2();
	value["entrypoints"] = serde_json::json!({});
	value["assets"]["app.js"]["mime"] = mime.into();
	// Act
	let result = decode_manifest(&serde_json::to_vec(&value).unwrap());
	// Assert
	assert!(result.is_err());
}

#[rstest]
#[tokio::test]
async fn existing_storage_and_template_readers_reject_invalid_mapping_values() {
	// Arrange
	use reinhardt_utils::staticfiles::{ManifestStaticFilesStorage, TemplateStaticConfig};
	let root = tempfile::tempdir().unwrap();
	std::fs::write(
		root.path().join("staticfiles.json"),
		br#"{"paths":{"valid.css":"valid.hash.css","broken.css":123}}"#,
	)
	.unwrap();
	let storage = ManifestStaticFilesStorage::new(root.path(), "/static/");
	// Act
	let loaded = storage.load_manifest().await;
	let template = TemplateStaticConfig::from_storage(&storage).await;
	// Assert
	assert!(
		loaded.is_err(),
		"a malformed entry must not silently disappear"
	);
	assert!(
		template.is_err(),
		"template and storage must share strict decoding"
	);
}
