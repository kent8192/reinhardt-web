#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetProducer, PreparedGeneration,
};
use rstest::rstest;

fn pack(pairs: &[(&str, &[u8])]) -> PreparedGeneration {
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in pairs {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()))
			.unwrap();
	}
	pipeline.prepare(AssetMode::Production).unwrap()
}

#[rstest]
fn discovery_order_does_not_change_published_identity() {
	// Arrange
	let a = pack(&[("a.txt", b"A"), ("b.txt", b"B")]);
	// Act
	let b = pack(&[("b.txt", b"B"), ("a.txt", b"A")]);
	let changed = pack(&[("a.txt", b"changed"), ("b.txt", b"B")]);
	// Assert
	assert_eq!(a.manifest_bytes(), b.manifest_bytes());
	assert_ne!(a.manifest().build_id, changed.manifest().build_id);
	assert_eq!(a.read_asset("a.txt").unwrap(), b"A");
}

#[rstest]
fn pages_producer_keeps_glue_wasm_and_nested_companions_together() {
	// Arrange
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in [
		("app.js", &b"export default 1;"[..]),
		("app_bg.wasm", b"\0asm\x01\0\0\0"),
		("snippets/nested.js", b"export default 2;"),
	] {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()).with_producer(AssetProducer::Pages))
			.unwrap();
	}
	// Act
	let result = pipeline.prepare(AssetMode::Production).unwrap();
	// Assert
	assert!(result.manifest().paths["app.js"].ends_with("/pages/app.js"));
	assert!(result.manifest().paths["app_bg.wasm"].ends_with("/pages/app_bg.wasm"));
	assert!(result.manifest().paths["snippets/nested.js"].ends_with("/pages/snippets/nested.js"));
}

#[rstest]
fn duplicate_logical_names_cannot_silently_choose_a_winner() {
	// Arrange
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes("a.txt", b"one".to_vec()))
		.unwrap();
	// Act
	let duplicate = pipeline.add_input(AssetInput::bytes("a.txt", b"two".to_vec()));
	// Assert
	assert!(duplicate.is_err());
}

use reinhardt_utils::staticfiles::publication::{
	AnalyzedAsset, AssetBuildError, AssetProcessor, AssetReference, PreparedAsset,
	ProcessorIdentity, RewriteOutput,
};
use std::collections::BTreeMap;

struct ReferenceProcessor;

impl AssetProcessor for ReferenceProcessor {
	fn identity(&self) -> ProcessorIdentity {
		ProcessorIdentity {
			id: "test.reference".into(),
			version: "1".into(),
			options: serde_json::json!({}),
		}
	}
	fn matches(&self, asset: &PreparedAsset) -> bool {
		asset.logical_path().ends_with(".assetref")
	}
	fn prepare(&self, _: &PreparedAsset) -> Result<Vec<AssetInput>, AssetBuildError> {
		Ok(vec![AssetInput::bytes(
			"generated.txt",
			b"generated".to_vec(),
		)])
	}
	fn analyze(&self, asset: &PreparedAsset) -> Result<Vec<AssetReference>, AssetBuildError> {
		Ok(vec![AssetReference {
			target: String::from_utf8(asset.read()?).unwrap(),
			suffix: String::new(),
			site: serde_json::Value::Null,
		}])
	}
	fn rewrite(
		&self,
		asset: &AnalyzedAsset,
		paths: &BTreeMap<String, String>,
	) -> Result<RewriteOutput, AssetBuildError> {
		Ok(RewriteOutput {
			bytes: paths[&asset.references[0].target].as_bytes().to_vec(),
			source_map: None,
		})
	}
}

#[rstest]
fn custom_processor_declares_generated_assets_before_rewriting() {
	// Arrange
	let mut pipeline = AssetPipeline::new();
	pipeline
		.register_processor(Box::new(ReferenceProcessor))
		.unwrap();
	pipeline
		.add_input(AssetInput::bytes(
			"reference.assetref",
			b"generated.txt".to_vec(),
		))
		.unwrap();
	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	// Assert
	assert_eq!(
		packed.read_asset("reference.assetref").unwrap(),
		b"other/generated.txt"
	);
	assert_eq!(packed.read_asset("generated.txt").unwrap(), b"generated");
	assert_eq!(
		packed.manifest().assets["reference.assetref"].dependencies,
		["generated.txt"]
	);
}

#[rstest]
fn missing_custom_dependency_is_an_actionable_error() {
	// Arrange
	let mut pipeline = AssetPipeline::new();
	pipeline
		.register_processor(Box::new(ReferenceProcessor))
		.unwrap();
	pipeline
		.add_input(AssetInput::bytes(
			"reference.assetref",
			b"missing.txt".to_vec(),
		))
		.unwrap();
	// Act
	let error = pipeline.prepare(AssetMode::Production).unwrap_err();
	// Assert
	assert!(error.to_string().contains("reference.assetref"));
	assert!(error.to_string().contains("missing.txt"));
}

struct LegacyUppercase;

#[async_trait::async_trait]
impl reinhardt_utils::staticfiles::processing::Processor for LegacyUppercase {
	async fn process(&self, input: &[u8], _: &std::path::Path) -> std::io::Result<Vec<u8>> {
		Ok(input.to_ascii_uppercase())
	}
	fn can_process(&self, path: &std::path::Path) -> bool {
		path.extension().is_some_and(|ext| ext == "txt")
	}
	fn name(&self) -> &str {
		"legacy-uppercase"
	}
}

#[rstest]
#[tokio::test]
async fn existing_async_byte_processor_runs_inside_an_async_caller() {
	// Arrange
	let mut pipeline = AssetPipeline::new();
	pipeline
		.add_input(AssetInput::bytes("a.txt", b"content".to_vec()))
		.unwrap();
	pipeline
		.register_byte_processor(
			Box::new(LegacyUppercase),
			ProcessorIdentity {
				id: "test.uppercase".into(),
				version: "1".into(),
				options: serde_json::json!({}),
			},
			false,
		)
		.unwrap();
	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	// Assert
	assert_eq!(packed.read_asset("a.txt").unwrap(), b"CONTENT");
	assert!(
		packed
			.manifest()
			.pipeline
			.iter()
			.any(|p| p.id == "test.uppercase")
	);
}

#[rstest]
#[case("movie.mp4")]
#[case("icon.svg")]
#[case("logo.png")]
#[case("style.css")]
#[case("script.js")]
#[case("font.woff2")]
#[case("sound.mp3")]
#[case("opaque.bin")]
fn every_asset_category_participates_in_generation_identity(#[case] name: &str) {
	// Arrange & Act
	let first = pack(&[(name, b"/* one */")]);
	let second = pack(&[(name, b"/* two */")]);
	// Assert
	assert_ne!(first.manifest().build_id, second.manifest().build_id);
}

#[rstest]
fn mode_and_classifier_options_participate_in_identity() {
	// Arrange
	let build = |mode, custom| {
		let mut pipeline = AssetPipeline::new();
		pipeline
			.add_input(AssetInput::bytes("a.txt", b"same".to_vec()))
			.unwrap();
		if custom {
			pipeline
				.classifier_mut()
				.register_extension(
					"txt",
					reinhardt_utils::staticfiles::publication::AssetCategory::Js,
				)
				.unwrap();
		}
		pipeline.prepare(mode).unwrap()
	};
	// Act
	let production = build(AssetMode::Production, false);
	let development = build(AssetMode::Development, false);
	let classified = build(AssetMode::Production, true);
	// Assert
	assert_ne!(
		production.manifest().build_id,
		development.manifest().build_id
	);
	assert_ne!(
		production.manifest().build_id,
		classified.manifest().build_id
	);
}

#[cfg(unix)]
#[rstest]
fn escaping_source_symlinks_are_rejected_and_relocated_roots_are_identical() {
	// Arrange
	let first = tempfile::tempdir().unwrap();
	let second = tempfile::tempdir().unwrap();
	std::fs::write(first.path().join("a.txt"), b"same").unwrap();
	std::fs::write(second.path().join("a.txt"), b"same").unwrap();
	let build = |root: &std::path::Path| {
		let mut pipeline = AssetPipeline::new();
		pipeline
			.add_input(AssetInput::from_directory(root.into(), "a.txt", "a.txt"))
			.unwrap();
		pipeline.prepare(AssetMode::Production)
	};
	// Act
	let a = build(first.path()).unwrap();
	let b = build(second.path()).unwrap();
	std::fs::remove_file(second.path().join("a.txt")).unwrap();
	std::os::unix::fs::symlink(first.path().join("a.txt"), second.path().join("a.txt")).unwrap();
	let escaped = build(second.path());
	// Assert
	assert_eq!(a.manifest_bytes(), b.manifest_bytes());
	assert!(escaped.is_err());
}
