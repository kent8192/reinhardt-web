//! Shared manifest selection for both runserver entrypoints.

use reinhardt_utils::staticfiles::publication::{
	AssetBuildError, DecodedAssetManifest, ManifestStore, SnapshotOptions, decode_manifest,
};
use std::path::{Path, PathBuf};

pub(crate) fn load_store(
	static_root: Option<&Path>,
	selected: Option<&Path>,
	mode: &str,
	expected: Option<&str>,
) -> Result<Option<ManifestStore>, AssetBuildError> {
	let invalid = |reason: &str| AssetBuildError::Input {
		asset: "runserver assets".into(),
		reason: reason.into(),
	};
	let mut options = match mode {
		"production" => SnapshotOptions::production(),
		"development" => SnapshotOptions::development(),
		_ => {
			return Err(invalid(
				"invalid --asset-mode; expected production or development",
			));
		}
	};
	let selected_path = selected
		.map(Path::to_path_buf)
		.or_else(|| static_root.map(|root| root.join("manifest.json")));
	let Some(path) = selected_path else {
		return if expected.is_some() {
			Err(invalid(
				"--expected-asset-build-id requires a version 2 asset manifest",
			))
		} else {
			Ok(None)
		};
	};
	let path = std::path::absolute(&path).map_err(|e| invalid(&e.to_string()))?;
	let bytes = match std::fs::read(&path) {
		Ok(bytes) => bytes,
		Err(error)
			if error.kind() == std::io::ErrorKind::NotFound
				&& expected.is_none()
				&& selected.is_none() =>
		{
			return Ok(None);
		}
		Err(error) => {
			return Err(invalid(&format!(
				"cannot read asset manifest {}: {error}",
				path.display()
			)));
		}
	};
	let DecodedAssetManifest::V2(manifest) = decode_manifest(&bytes)? else {
		return if expected.is_some() {
			Err(invalid(
				"--expected-asset-build-id requires a version 2 asset manifest",
			))
		} else {
			Ok(None)
		};
	};
	let parent = path.parent().expect("absolute manifest path has a parent");
	let inferred_root = if parent.file_name().and_then(|name| name.to_str())
		== Some(manifest.build_id.as_str())
		&& parent
			.parent()
			.and_then(Path::file_name)
			.and_then(|name| name.to_str())
			== Some("builds")
	{
		parent
			.parent()
			.and_then(Path::parent)
			.expect("generation has a publication root")
	} else {
		parent
	};
	let configured_root = static_root
		.map(std::path::absolute)
		.transpose()
		.map_err(|e| invalid(&e.to_string()))?;
	let root: PathBuf = configured_root
		.filter(|root| path.starts_with(root))
		.unwrap_or_else(|| inferred_root.into());
	if selected.is_some() {
		options = options.manifest_path(path);
	}
	if let Some(expected) = expected {
		options = options.expected_build_id(expected.into());
	}
	ManifestStore::open(root, options).map(Some)
}

// Keep framework-owned routes and Pages selection identical in both servers.
pub(crate) fn serving_config(
	store: std::sync::Arc<ManifestStore>,
	static_url: String,
	entrypoint: Option<&str>,
	navigation: bool,
) -> Result<reinhardt_utils::staticfiles::publication::ManifestServingConfig, AssetBuildError> {
	use reinhardt_utils::staticfiles::publication::ManifestServingConfig;

	let mut config =
		ManifestServingConfig::new(store, static_url.clone())?.with_navigation_fallback(navigation);
	if let Some(entrypoint) = entrypoint {
		config = config.with_pages(entrypoint.into());
	}
	let uri: hyper::Uri = static_url
		.parse()
		.map_err(
			|error: hyper::http::uri::InvalidUri| AssetBuildError::Input {
				asset: static_url.clone(),
				reason: error.to_string(),
			},
		)?;
	let config = config.with_passthrough_prefixes(vec![
		"/api".into(),
		"/docs".into(),
		"/openapi.json".into(),
		"/static/admin".into(),
		format!("{}/admin", uri.path().trim_end_matches('/')),
	])?;
	config.validate()?;
	Ok(config)
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_utils::staticfiles::publication::{
		AssetInput, AssetMode, AssetPipeline, AssetPublisher,
	};
	use rstest::rstest;

	fn publish(root: &Path, content: &[u8]) -> String {
		let mut pipeline = AssetPipeline::new();
		pipeline
			.add_input(AssetInput::bytes("a.txt", content.to_vec()))
			.unwrap();
		AssetPublisher::new(root.into())
			.publish(pipeline.prepare(AssetMode::Production).unwrap())
			.unwrap()
			.manifest()
			.build_id
			.clone()
	}

	#[rstest]
	#[case(Some("second"), true, Some("second"))]
	#[case(Some("first"), true, Some("first"))]
	#[case(Some("second"), false, None)]
	#[case(None, true, None)]
	#[case(None, false, None)]
	#[case(Some("missing"), true, None)]
	#[case(Some("missing"), false, None)]
	#[tokio::test]
	async fn named_entrypoint_controls_navigation(
		#[case] selected: Option<&str>,
		#[case] navigation: bool,
		#[case] expected_document: Option<&str>,
	) {
		use reinhardt_http::{Handler, Middleware, Request, Response};
		use reinhardt_utils::staticfiles::publication::{
			AssetProducer, AssetRole, ManifestStaticMiddleware, PagesEntrypoint,
		};
		use std::sync::Arc;
		struct Missing;
		#[async_trait::async_trait]
		impl Handler for Missing {
			async fn handle(&self, _: Request) -> reinhardt_core::exception::Result<Response> {
				Ok(Response::not_found())
			}
		}
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let mut pipeline = AssetPipeline::new();
		pipeline
			.add_input(
				AssetInput::bytes("app.js", b"export default function init(){};export const wasm=new URL('app.wasm',import.meta.url);".to_vec())
					.with_producer(AssetProducer::Pages),
			)
			.unwrap();
		pipeline
			.add_input(
				AssetInput::bytes("app.wasm", b"\0asm\x01\0\0\0".to_vec())
					.with_producer(AssetProducer::Pages),
			)
			.unwrap();
		for name in ["first", "second"] {
			let document = format!("{name}.html");
			pipeline
				.add_input(
					AssetInput::bytes(
						&document,
						format!("<html><head></head><body>{name}</body></html>").into_bytes(),
					)
					.with_role(AssetRole::EntryDocument),
				)
				.unwrap();
			pipeline
				.set_entrypoint(
					name,
					PagesEntrypoint {
						javascript: "app.js".into(),
						wasm: "app.wasm".into(),
						styles: vec![],
						document: Some(document),
					},
				)
				.unwrap();
		}
		AssetPublisher::new(root.path().into())
			.publish(pipeline.prepare(AssetMode::Production).unwrap())
			.unwrap();
		let store = Arc::new(
			load_store(Some(root.path()), None, "production", None)
				.unwrap()
				.unwrap(),
		);
		// Act
		let config = serving_config(store.clone(), "/static/".into(), selected, navigation);
		// Assert
		if selected.is_none() || selected == Some("missing") {
			assert_eq!(
				config.unwrap_err().to_string(),
				if selected.is_none() {
					"invalid static asset manifest: select a Pages entrypoint explicitly for navigation or a multi-entry publication"
				} else {
					"invalid asset \"missing\": unknown Pages entrypoint; select a manifest entrypoint name"
				}
			);
			return;
		}
		let response = ManifestStaticMiddleware::new(config.unwrap())
			.process(
				Request::builder()
					.uri("/dashboard")
					.header("accept", "text/html")
					.build()
					.unwrap(),
				Arc::new(Missing),
			)
			.await
			.unwrap();
		if let Some(name) = expected_document {
			assert_eq!(response.status, hyper::StatusCode::OK);
			let snapshot = store.active();
			let template = snapshot.read_asset(&format!("{name}.html")).unwrap();
			let expected = reinhardt_utils::staticfiles::publication::render_entry_document(
				&snapshot,
				"/static/",
				name,
				std::str::from_utf8(&template).unwrap(),
			)
			.unwrap();
			assert_eq!(response.body.as_ref(), expected.as_bytes());
		} else {
			assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
		}
	}

	#[rstest]
	#[case(false, false)]
	#[case(false, true)]
	#[case(true, false)]
	#[case(true, true)]
	fn selected_manifest_pins_its_generation(#[case] retained: bool, #[case] configured: bool) {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let expected = publish(root.path(), b"first");
		let selected = if retained {
			root.path().join(format!("builds/{expected}/manifest.json"))
		} else {
			let custom = root.path().join("custom.json");
			std::fs::copy(root.path().join("manifest.json"), &custom).unwrap();
			custom
		};
		publish(root.path(), b"second");
		// Act
		let store = load_store(
			configured.then_some(root.path()),
			Some(&selected),
			"production",
			Some(&expected),
		)
		.unwrap()
		.unwrap();
		// Assert
		assert_eq!(store.active().manifest().build_id, expected);
		assert_eq!(store.active().read_asset("a.txt").unwrap(), b"first");
	}

	#[rstest]
	fn custom_manifest_works_without_canonical_pointer() {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let id = publish(root.path(), b"first");
		let selected = root.path().join("custom.json");
		std::fs::rename(root.path().join("manifest.json"), &selected).unwrap();
		// Act
		let store = load_store(Some(root.path()), Some(&selected), "production", None)
			.unwrap()
			.unwrap();
		// Assert
		assert_eq!(store.active().manifest().build_id, id);
	}

	#[rstest]
	#[case(false)]
	#[case(true)]
	fn expected_build_rejects_missing_or_legacy_manifest(#[case] legacy: bool) {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		if legacy {
			std::fs::write(root.path().join("manifest.json"), br#"{"paths":{}}"#).unwrap();
		}
		// Act
		let result = load_store(Some(root.path()), None, "production", Some("required"));
		// Assert
		assert!(matches!(result, Err(AssetBuildError::Input { .. })));
		assert!(
			load_store(Some(root.path()), None, "production", None)
				.unwrap()
				.is_none()
		);
	}

	#[rstest]
	fn explicitly_selected_missing_manifest_is_an_error() {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let selected = root.path().join("missing.json");
		// Act
		let result = load_store(Some(root.path()), Some(&selected), "production", None);
		// Assert
		assert!(matches!(result, Err(AssetBuildError::Input { .. })));
	}

	#[rstest]
	fn expected_build_requires_a_manifest_location() {
		assert!(matches!(
			load_store(None, None, "production", Some("required")),
			Err(AssetBuildError::Input { .. })
		));
	}
}
