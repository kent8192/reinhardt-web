#![cfg(feature = "types")]

use reinhardt_core::types::static_assets::{AssetUrlError, AssetUrlSnapshot};
use rstest::rstest;
use std::collections::BTreeMap;

fn snapshot(prefix: &str, path: &str) -> Result<AssetUrlSnapshot, AssetUrlError> {
	let id = "a".repeat(64);
	AssetUrlSnapshot::new(
		id.clone(),
		prefix.into(),
		BTreeMap::from([("app.js".into(), format!("builds/{id}/{path}"))]),
	)
}

#[rstest]
#[case("/console/static/", "/console/static/")]
#[case("/", "/")]
#[case("/static", "/static/")]
#[case(
	"https://cdn.example.test/console/static/",
	"https://cdn.example.test/console/static/"
)]
fn generation_urls_preserve_the_configured_prefix(#[case] prefix: &str, #[case] expected: &str) {
	// Arrange
	let urls = snapshot(prefix, "pages/app.js").unwrap();
	// Act
	let url = urls.resolve("app.js").unwrap();
	// Assert
	assert_eq!(
		url,
		format!("{expected}builds/{}/pages/app.js", "a".repeat(64))
	);
	assert!(matches!(
		urls.resolve("unknown.js"),
		Err(AssetUrlError::UnknownLogicalPath { .. })
	));
}

#[rstest]
fn decoded_names_are_encoded_exactly_once() {
	// Arrange
	let urls = snapshot("/static/", "js/日本 語%20#.js").unwrap();
	// Act
	let url = urls.resolve("app.js").unwrap();
	// Assert
	assert!(url.ends_with("/js/%E6%97%A5%E6%9C%AC%20%E8%AA%9E%2520%23.js"));
}

#[rstest]
#[case("../outside.js")]
#[case("pages/../outside.js")]
#[case("pages//app.js")]
#[case("pages/./app.js")]
#[case("pages\\app.js")]
#[case("pages/app.js\0")]
fn traversal_or_ambiguous_paths_are_rejected(#[case] path: &str) {
	// Arrange & Act
	let result = snapshot("/static/", path);
	// Assert
	assert!(matches!(result, Err(AssetUrlError::InvalidPath { .. })));
}

#[rstest]
#[case("//evil.test/static/")]
#[case("static/")]
#[case("https://user:pass@example.test/static/")]
#[case("/static/?query=1")]
#[case("/static/#fragment")]
#[case("javascript:alert(1)")]
#[case("/console/%2e%2e/static/")]
#[case("/console/%2fstatic/")]
fn invalid_static_prefixes_are_rejected(#[case] prefix: &str) {
	// Arrange & Act
	let result = snapshot(prefix, "pages/app.js");
	// Assert
	assert!(matches!(result, Err(AssetUrlError::InvalidPrefix)));
}

#[rstest]
fn projection_roundtrip_preserves_generation_and_revalidates_input() {
	// Arrange
	let urls = snapshot("/console/static/", "pages/app.js").unwrap();
	let json = urls.to_json().unwrap();
	// Act
	let decoded = AssetUrlSnapshot::from_json(&json).unwrap();
	let malformed = json.replace(&"a".repeat(64), "BAD");
	// Assert
	assert_eq!(decoded.resolve("app.js"), urls.resolve("app.js"));
	assert!(AssetUrlSnapshot::from_json(&malformed).is_err());
}

#[rstest]
fn paths_cannot_escape_the_selected_generation() {
	// Arrange
	let paths = BTreeMap::from([(
		"app.js".into(),
		format!("builds/{}/pages/app.js", "b".repeat(64)),
	)]);
	// Act
	let result = AssetUrlSnapshot::new("a".repeat(64), "/static/".into(), paths);
	// Assert
	assert!(matches!(result, Err(AssetUrlError::InvalidPath { .. })));
}
