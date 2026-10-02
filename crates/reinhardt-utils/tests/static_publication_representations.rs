#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::publication::{
	AssetEncoding, AssetInput, AssetMode, AssetPipeline, PreparedGeneration,
};
use rstest::rstest;
use std::io::{Read, Write};

fn gzip(bytes: &[u8]) -> Vec<u8> {
	let mut encoder = flate2::GzBuilder::new()
		.mtime(0)
		.write(Vec::new(), flate2::Compression::default());
	encoder.write_all(bytes).unwrap();
	encoder.finish().unwrap()
}

fn pack(
	inputs: &[(&str, &[u8])],
) -> Result<PreparedGeneration, reinhardt_utils::staticfiles::publication::AssetBuildError> {
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in inputs {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()))
			.unwrap();
	}
	pipeline.prepare(AssetMode::Production)
}

#[rstest]
fn gzip_is_regenerated_from_the_rewritten_canonical_css() {
	// Arrange
	let css = b"body{background:url('images/icon.svg')}";
	let encoded = gzip(css);
	let inputs: &[(&str, &[u8])] = &[
		("main.css", css),
		("main.css.gz", &encoded),
		("images/icon.svg", b"<svg/>"),
	];
	// Act
	let first = pack(inputs).unwrap();
	let second = pack(inputs).unwrap();
	let bytes = first.read_asset("main.css.gz").unwrap();
	let mut decoded = Vec::new();
	flate2::read::GzDecoder::new(bytes.as_slice())
		.read_to_end(&mut decoded)
		.unwrap();
	// Assert
	assert_eq!(decoded, first.read_asset("main.css").unwrap());
	assert_ne!(decoded, css);
	assert_eq!(first.manifest_bytes(), second.manifest_bytes());
	assert_eq!(
		first.manifest().assets["main.css.gz"].encoding,
		Some(AssetEncoding::Gzip)
	);
	assert_eq!(
		first.manifest().assets["main.css.gz"].parent.as_deref(),
		Some("main.css")
	);
	assert_eq!(
		first.manifest().assets["main.css"].variants,
		["main.css.gz"]
	);
}

#[rstest]
#[case("icon.svgz", "icon.svg", &b"<svg/>"[..])]
#[case("main.css.gz", "main.css", &b"body{color:red}"[..])]
fn encoded_only_inputs_create_a_canonical_parent(
	#[case] name: &str,
	#[case] parent: &str,
	#[case] original: &[u8],
) {
	// Arrange
	let bytes = gzip(original);
	// Act
	let packed = pack(&[(name, &bytes)]).unwrap();
	// Assert
	assert_eq!(packed.read_asset(parent).unwrap(), original);
	assert_eq!(
		packed.manifest().assets[name].mime,
		packed.manifest().assets[parent].mime
	);
}

#[rstest]
fn conflicting_or_malformed_encoded_inputs_fail() {
	// Arrange
	let conflict = gzip(b"body{color:blue}");
	// Act
	let mismatch = pack(&[("a.css", b"body{color:red}"), ("a.css.gz", &conflict)]);
	let malformed = pack(&[("a.css.gz", b"invalid gzip")]);
	// Assert
	assert!(mismatch.unwrap_err().to_string().contains("a.css.gz"));
	assert!(malformed.unwrap_err().to_string().contains("a.css.gz"));
}

#[rstest]
fn archive_gzip_is_opaque_and_brotli_maps_keep_their_parent_role() {
	// Arrange
	let map = br#"{"version":3,"sources":[],"names":[],"mappings":""}"#;
	let mut encoded = Vec::new();
	{
		let mut writer = brotli::CompressorWriter::new(&mut encoded, 4096, 6, 22);
		writer.write_all(map).unwrap();
	}
	// Act
	let packed = pack(&[
		("backup.tar.gz", b"opaque archive"),
		("a.css.map.br", &encoded),
	])
	.unwrap();
	// Assert
	assert_eq!(
		packed.read_asset("backup.tar.gz").unwrap(),
		b"opaque archive"
	);
	assert_eq!(packed.read_asset("a.css.map").unwrap(), map);
	assert_eq!(
		packed.manifest().assets["a.css.map.br"].role,
		reinhardt_utils::staticfiles::publication::AssetRole::SourceMap
	);
}

#[rstest]
fn source_mapping_tracks_a_length_changing_url_edit() {
	// Arrange
	let original = "const url=new URL('images/icon.svg',import.meta.url); const marker=1;\n//# sourceMappingURL=main.js.map";
	let old_column = original.find("const marker").unwrap() as u32;
	let mut builder = sourcemap::SourceMapBuilder::new(Some("main.js"));
	builder.add(
		0,
		old_column,
		7,
		4,
		Some("source.ts"),
		Some("marker"),
		false,
	);
	let source_id = builder.add_source("source.ts");
	builder.set_source_contents(source_id, Some("// source fixture"));
	let mut bytes = Vec::new();
	builder.into_sourcemap().to_writer(&mut bytes).unwrap();
	// Act
	let packed = pack(&[
		("main.js", original.as_bytes()),
		("main.js.map", &bytes),
		("images/icon.svg", b"<svg/>"),
	])
	.unwrap();
	let rewritten = String::from_utf8(packed.read_asset("main.js").unwrap()).unwrap();
	let map = sourcemap::SourceMap::from_slice(&packed.read_asset("main.js.map").unwrap()).unwrap();
	let token = map
		.lookup_token(0, rewritten.find("const marker").unwrap() as u32)
		.unwrap();
	// Assert
	assert_eq!(
		token.get_dst_col(),
		rewritten.find("const marker").unwrap() as u32
	);
	assert_eq!(token.get_src(), (7, 4));
	assert_eq!(token.get_source(), Some("source.ts"));
	assert_eq!(map.get_source_contents(0), Some("// source fixture"));
}

#[rstest]
#[case("../../original.ts", None)]
#[case("original.ts", Some("https://sources.example.test/project/"))]
fn embedded_and_external_source_roots_preserve_original_source_identity(
	#[case] source: &str,
	#[case] root: Option<&str>,
) {
	// Arrange
	let map = serde_json::json!({"version":3,"file":"app.js","sources":[source],"sourceRoot":root,"sourcesContent":["const x=1;"],"names":[],"mappings":"AAAA"});
	let bytes = serde_json::to_vec(&map).unwrap();
	// Act
	let packed = pack(&[
		("app.js", b"const x=1;\n//# sourceMappingURL=app.js.map"),
		("app.js.map", &bytes),
	])
	.unwrap();
	let actual: serde_json::Value =
		serde_json::from_slice(&packed.read_asset("app.js.map").unwrap()).unwrap();
	// Assert
	assert_eq!(actual["sources"], map["sources"]);
	assert_eq!(actual["sourcesContent"], map["sourcesContent"]);
	if root.is_some() {
		assert_eq!(actual["sourceRoot"], map["sourceRoot"]);
	}
}

#[rstest]
fn duplicate_gzip_forms_for_one_parent_are_ambiguous() {
	// Arrange
	let encoded = gzip(b"<svg/>");
	// Act
	let result = pack(&[("icon.svgz", &encoded), ("icon.svg.gz", &encoded)]);
	// Assert
	assert!(result.unwrap_err().to_string().contains("duplicate"));
}
