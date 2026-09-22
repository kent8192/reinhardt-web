#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::publication::{DecodedAssetManifest, decode_manifest};
use rstest::rstest;

#[rstest]
#[case("version", "version.abc")]
#[case("version", "1.0")]
#[case("version", "2.0")]
#[case("paths", "paths.abc")]
#[case("files", "files.abc")]
fn flat_manifest_preserves_metadata_named_assets(#[case] name: &str, #[case] published: &str) {
	// Arrange
	let mut expected = std::collections::BTreeMap::new();
	expected.insert(name.to_owned(), published.to_owned());
	expected.insert("app.js".to_owned(), "app.abc.js".to_owned());
	let bytes = serde_json::to_vec(&expected).unwrap();

	// Act
	let decoded = decode_manifest(&bytes).unwrap();

	// Assert
	let DecodedAssetManifest::Legacy(legacy) = decoded else {
		panic!("a flat string map must remain a legacy manifest");
	};
	assert_eq!(legacy.paths, expected);
}

#[rstest]
fn flat_manifest_can_contain_all_structured_key_names() {
	// Arrange
	let expected = std::collections::BTreeMap::from([
		("version".to_owned(), "version.abc".to_owned()),
		("paths".to_owned(), "paths.abc".to_owned()),
		("files".to_owned(), "files.abc".to_owned()),
	]);

	// Act
	let decoded = decode_manifest(&serde_json::to_vec(&expected).unwrap()).unwrap();

	// Assert
	let DecodedAssetManifest::Legacy(legacy) = decoded else {
		panic!("metadata-like names are ordinary assets in a flat map");
	};
	assert_eq!(legacy.paths, expected);
}

#[rstest]
#[case(br#"{"version":"9.0","paths":{}}"#)]
#[case(br#"{"version":2,"paths":{}}"#)]
#[case(br#"{"paths":{},"files":{}}"#)]
#[case(br#"{"paths":[],"app.js":"app.abc.js"}"#)]
#[case(br#"{"version":"../outside","app.js":"app.abc.js"}"#)]
#[case(br#"{"version":"one","version":"two"}"#)]
fn structured_metadata_and_flat_paths_remain_strict(#[case] input: &[u8]) {
	// Act
	let result = decode_manifest(input);

	// Assert
	assert!(result.is_err(), "must reject {}", String::from_utf8_lossy(input));
}
