#![cfg(feature = "asset-publication")]

use reinhardt_utils::staticfiles::publication::{
	AssetInput, AssetMode, AssetPipeline, AssetProducer,
};
use rstest::rstest;

fn pipeline(inputs: &[(&str, &[u8])]) -> AssetPipeline {
	let mut pipeline = AssetPipeline::new();
	for (name, bytes) in inputs {
		pipeline
			.add_input(AssetInput::bytes(name, bytes.to_vec()))
			.unwrap();
	}
	pipeline
}

#[rstest]
fn css_references_follow_category_moves_and_keep_suffixes() {
	// Arrange
	let inputs: &[(&str, &[u8])] = &[
		("styles/main.css", b"@import './other.css?v=2'; .icon{background:url('../images/logo.svg#mark')} @font-face{src:url('../assets/font.woff2')}"),
		("styles/other.css", b".other{color:red}"),
		("images/logo.svg", br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#),
		("assets/font.woff2", b"font fixture"),
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let css = String::from_utf8(packed.read_asset("styles/main.css").unwrap()).unwrap();
	// Assert
	assert!(css.contains("../../vectors/logo.svg#mark"), "{css}");
	assert!(css.contains("../../fonts/assets/font.woff2"), "{css}");
	assert!(css.contains("./other.css?v=2"), "{css}");
	assert_eq!(
		packed.manifest().assets["styles/main.css"].dependencies,
		["assets/font.woff2", "images/logo.svg", "styles/other.css"]
	);
}

#[rstest]
fn literal_import_cycles_keep_every_node_and_rewrite_only_syntax() {
	// Arrange
	let inputs: &[(&str, &[u8])] = &[
		("scripts/a.js", b"import './b.js'; export { x } from '../shared/x.mjs'; const untouched='../images/logo.svg'; // import '../absent.js'\n export const image = new URL('../images/logo.svg', import.meta.url);"),
		("scripts/b.js", b"export * from './a.js'; export const later = () => import('../shared/x.mjs?q=1#chunk');"),
		("shared/x.mjs", b"export const x=1;"),
		("images/logo.svg", b"<svg/>")
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let js = String::from_utf8(packed.read_asset("scripts/a.js").unwrap()).unwrap();
	// Assert
	assert_eq!(packed.manifest().assets.len(), 4);
	assert!(js.contains("../../vectors/logo.svg"), "{js}");
	assert!(js.contains("const untouched='../images/logo.svg'"), "{js}");
	assert!(js.contains("// import '../absent.js'"), "{js}");
	assert!(
		packed.manifest().assets["scripts/b.js"]
			.dependencies
			.contains(&"scripts/a.js".into())
	);
}

#[rstest]
fn wasm_bindgen_fallback_and_snippets_resolve_inside_pages() {
	// Arrange
	let mut inputs = AssetPipeline::new();
	for (name, bytes) in [
		("app.js", &b"import './snippets/tool.js'; export const wasm = new URL('app_bg.wasm', import.meta.url);"[..]),
		("app_bg.wasm", b"\0asm\x01\0\0\0"),
		("snippets/tool.js", b"export const tool=1;")
	] { inputs.add_input(AssetInput::bytes(name, bytes.to_vec()).with_producer(AssetProducer::Pages)).unwrap(); }
	// Act
	let packed = inputs.prepare(AssetMode::Production).unwrap();
	let glue = String::from_utf8(packed.read_asset("app.js").unwrap()).unwrap();
	// Assert
	assert!(glue.contains("./app_bg.wasm"), "{glue}");
	assert!(glue.contains("./snippets/tool.js"), "{glue}");
	assert_eq!(
		packed.manifest().assets["app.js"].dependencies,
		["app_bg.wasm", "snippets/tool.js"]
	);
}

#[rstest]
#[case("app.js", "import './absent.js';")]
#[case("app.js", "const name='x'; import(name);")]
#[case("app.js", "new URL(name, import.meta.url);")]
#[case("app.js", "import 'unbundled-package';")]
#[case("app.css", "body{background:url('./absent.svg')}")]
fn unsupported_or_missing_references_fail_with_the_source_name(
	#[case] name: &str,
	#[case] text: &str,
) {
	// Arrange
	let inputs = [(name, text.as_bytes())];
	// Act
	let result = pipeline(&inputs).prepare(AssetMode::Production);
	// Assert
	assert!(result.is_err());
	assert!(result.unwrap_err().to_string().contains(name));
}

#[rstest]
fn external_data_and_fragment_urls_are_preserved() {
	// Arrange
	let css = b".a{background:url(data:image/svg+xml;base64,PHN2Zy8+)} .b{mask:url(#local)} @import 'https://example.test/theme.css';";
	// Act
	let packed = pipeline(&[("app.css", css)])
		.prepare(AssetMode::Production)
		.unwrap();
	// Assert
	assert_eq!(packed.read_asset("app.css").unwrap(), css);
	assert!(packed.manifest().assets["app.css"].dependencies.is_empty());
}

#[rstest]
fn html_asset_attributes_move_without_rewriting_navigation() {
	// Arrange
	let html = br#"<!doctype html><html><head><link rel="stylesheet" href="styles/site.css"></head><body><a href="account">Account</a><img src="images/logo.svg" srcset="images/logo.svg 1x, images/logo.svg?large 2x"></body></html>"#;
	let inputs: &[(&str, &[u8])] = &[
		("index.html", html),
		("styles/site.css", b"body{color:red}"),
		("images/logo.svg", b"<svg/>"),
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let output = String::from_utf8(packed.read_asset("index.html").unwrap()).unwrap();
	// Assert
	assert!(output.contains("../css/styles/site.css"), "{output}");
	assert!(output.contains("../vectors/logo.svg"), "{output}");
	assert!(output.contains("href=\"account\""), "{output}");
}

#[rstest]
fn inline_html_and_srcset_data_urls_keep_their_syntax() {
	// Arrange
	let html = br#"<html><head><style>.a{mask:url('images/logo.svg#icon')}</style></head><body style="background:url('images/logo.svg')"><img srcset="data:image/png;base64,YQ== 1x, images/logo.svg 2x"><script type="module">import './app.js'; const label = '<unrelated>';</script></body></html>"#;
	let inputs: &[(&str, &[u8])] = &[
		("index.html", html),
		("app.js", b"export const x=1;"),
		("images/logo.svg", b"<svg/>"),
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let output = String::from_utf8(packed.read_asset("index.html").unwrap()).unwrap();
	// Assert
	assert!(output.contains("../vectors/logo.svg#icon"), "{output}");
	assert!(
		output.contains("data:image/png;base64,YQ== 1x, ../vectors/logo.svg 2x"),
		"{output}"
	);
	assert!(
		output.contains("import \"../js/app.js\"; const label = '<unrelated>';"),
		"{output}"
	);
	assert_eq!(
		packed.manifest().assets["index.html"].dependencies,
		["app.js", "images/logo.svg"]
	);
}

#[rstest]
fn entry_document_retains_logical_template_until_rendering() {
	// Arrange
	let template = br#"<img src="{{ static_url('images/logo.svg') }}">"#;
	let mut pipeline = pipeline(&[("images/logo.svg", b"<svg/>")]);
	pipeline
		.add_input(
			AssetInput::bytes("views/index.html", template.to_vec())
				.with_role(reinhardt_utils::staticfiles::publication::AssetRole::EntryDocument),
		)
		.unwrap();
	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	// Assert
	assert_eq!(packed.read_asset("views/index.html").unwrap(), template);
	assert_eq!(
		packed.manifest().assets["views/index.html"].dependencies,
		["images/logo.svg"]
	);
}

#[rstest]
fn imported_entry_document_aliases_are_normalized_to_logical_names() {
	// Arrange
	let mut pipeline = pipeline(&[("styles/main.css", b"body{}")]);
	pipeline
		.add_input(
			AssetInput::bytes(
				"views/index.html",
				br#"<link rel="stylesheet" href="/console/static/styles/main.hash.css">"#.to_vec(),
			)
			.with_role(reinhardt_utils::staticfiles::publication::AssetRole::EntryDocument),
		)
		.unwrap();
	pipeline
		.add_alias("console/static/styles/main.hash.css", "styles/main.css")
		.unwrap();
	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	let bytes = packed.read_asset("views/index.html").unwrap();
	let references = reinhardt_utils::staticfiles::publication::analyze_asset_references(
		"views/index.html",
		"text/html",
		&bytes,
	)
	.unwrap();
	// Assert: the request renderer needs only the logical manifest, never discarded input aliases.
	assert_eq!(references[0].target, "styles/main.css");
	assert!(!String::from_utf8(bytes).unwrap().contains("main.hash.css"));
}

#[rstest]
fn escaped_syntax_and_percent_encoded_names_resolve_exactly_once() {
	// Arrange
	let inputs: &[(&str, &[u8])] = &[
		("main.js", br#"import './hello\u0020world.js'; const u=new URL('./images/100%25.svg?q=1#mark',import.meta.url);"#),
		("hello world.js", b"export const x=1;"),
		("main.css", br#"a{mask:url('images/100%25.svg')} b{mask:url(images/lo\67 o.svg)}"#),
		("images/100%.svg", b"<svg/>"), ("images/logo.svg", b"<svg/>")
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let js = String::from_utf8(packed.read_asset("main.js").unwrap()).unwrap();
	let css = String::from_utf8(packed.read_asset("main.css").unwrap()).unwrap();
	// Assert
	assert!(js.contains("./hello%20world.js"), "{js}");
	assert!(js.contains("../vectors/100%25.svg?q=1#mark"), "{js}");
	assert!(css.contains("../vectors/logo.svg"), "{css}");
}

#[rstest]
#[case("<script type=importmap>{\"imports\":{}}</script>")]
#[case("<base href='/elsewhere/'><img src='x.svg'>")]
fn html_rejects_unsupported_base_and_import_maps(#[case] html: &str) {
	// Arrange
	let inputs = pipeline(&[("index.html", html.as_bytes())]);
	// Act
	let result = inputs.prepare(AssetMode::Production);
	// Assert
	assert!(result.unwrap_err().to_string().contains("index.html"));
}

#[rstest]
fn parameterized_css_mime_still_rewrites_dependencies() {
	// Arrange
	let mut pipeline = pipeline(&[("logo.svg", b"<svg/>")]);
	pipeline
		.add_input(
			AssetInput::bytes("theme.css", b"body{mask:url('logo.svg')}".to_vec())
				.with_mime("text/css; charset=utf-8".into()),
		)
		.unwrap();
	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	// Assert
	let css = String::from_utf8(packed.read_asset("theme.css").unwrap()).unwrap();
	assert!(css.contains("../vectors/logo.svg"), "{css}");
}

#[rstest]
fn explicit_document_relative_runtime_import_survives_relocation() {
	// Arrange
	let source = b"const entry = document.querySelector('script').dataset.wasmEntry; await import(new URL(entry, document.baseURI).href);";
	let pipeline = pipeline(&[("runtime.js", source)]);
	// Act
	let packed = pipeline.prepare(AssetMode::Production).unwrap();
	// Assert
	assert_eq!(packed.read_asset("runtime.js").unwrap(), source);
	assert_eq!(
		packed.manifest().assets["runtime.js"].dependencies,
		Vec::<String>::new()
	);
}

#[rstest]
#[case("await import(entry);")]
#[case("await import(new URL(entry, import.meta.url).href);")]
#[case("await import(new URL(entry, base).href);")]
fn computed_module_relative_imports_still_fail(#[case] source: &str) {
	// Arrange
	let pipeline = pipeline(&[("runtime.js", source.as_bytes())]);
	// Act
	let result = pipeline.prepare(AssetMode::Production);
	// Assert
	assert!(matches!(
		result,
		Err(reinhardt_utils::staticfiles::publication::AssetBuildError::Input { .. })
	));
}

#[rstest]
#[case("prefetch")]
#[case("PREFETCH")]
#[case("alternate prefetch")]
fn html_prefetch_declares_and_rewrites_asset_dependencies(#[case] relation: &str) {
	// Arrange
	let html = format!(r#"<link rel="{relation}" href="chunks/next.js?v=2#module">"#);
	let inputs: &[(&str, &[u8])] = &[
		("index.html", html.as_bytes()),
		("chunks/next.js", b"export const next=1;"),
	];
	// Act
	let prepared = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	// Assert
	assert_eq!(
		prepared.manifest().assets["index.html"].dependencies,
		["chunks/next.js"]
	);
	let rewritten = String::from_utf8(prepared.read_asset("index.html").unwrap()).unwrap();
	assert!(
		rewritten.contains("../js/chunks/next.js?v=2#module"),
		"{rewritten}"
	);
}

#[rstest]
fn image_preload_candidates_and_iframe_sources_follow_published_paths() {
	// Arrange
	let html = br#"<link rel="preload" as="image" imagesrcset="images/small.png 1x, images/large.png 2x"><iframe src="docs/frame.html"></iframe>"#;
	let inputs: &[(&str, &[u8])] = &[
		("index.html", html),
		("images/small.png", b"small"),
		("images/large.png", b"large"),
		("docs/frame.html", b"<p>frame</p>"),
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let output = String::from_utf8(packed.read_asset("index.html").unwrap()).unwrap();
	// Assert
	assert!(
		output.contains("imagesrcset=\"../images/small.png 1x, ../images/large.png 2x\""),
		"{output}"
	);
	assert!(
		output.contains("<iframe src=\"./docs/frame.html\"></iframe>"),
		"{output}"
	);
	assert_eq!(
		packed.manifest().assets["index.html"].dependencies,
		["docs/frame.html", "images/large.png", "images/small.png"]
	);
}

#[rstest]
fn classic_inline_scripts_rewrite_dynamic_imports() {
	// Arrange
	let html = br#"<script>import('./chunks/widget.js').then(({ mount }) => mount());</script>"#;
	let inputs: &[(&str, &[u8])] = &[
		("index.html", html),
		("chunks/widget.js", b"export function mount(){}"),
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let output = String::from_utf8(packed.read_asset("index.html").unwrap()).unwrap();
	// Assert
	assert!(
		output.contains("import(\"../js/chunks/widget.js\")"),
		"{output}"
	);
	assert_eq!(
		packed.manifest().assets["index.html"].dependencies,
		["chunks/widget.js"]
	);
}

#[rstest]
fn web_manifest_asset_urls_follow_published_paths() {
	// Arrange
	let inputs: &[(&str, &[u8])] = &[
		("manifest.webmanifest", br#"{"name":"Demo","icons":[{"src":"icons/app icon.png","sizes":"192x192"}],"screenshots":[{"src":"screenshots/home.png"}],"shortcuts":[{"name":"Open","url":"/open","icons":[{"src":"icons/shortcut.png"}]}],"file_handlers":[{"action":"/open-file/","icons":[{"src":"icons/file.png"}]}]}"#),
		("icons/app icon.png", b"app icon"),
		("icons/shortcut.png", b"shortcut icon"),
		("icons/file.png", b"file icon"),
		("screenshots/home.png", b"screenshot"),
	];
	// Act
	let packed = pipeline(inputs).prepare(AssetMode::Production).unwrap();
	let output = String::from_utf8(packed.read_asset("manifest.webmanifest").unwrap()).unwrap();
	let manifest: serde_json::Value = serde_json::from_str(&output).unwrap();
	// Assert
	assert_eq!(
		manifest["icons"][0]["src"],
		"../images/icons/app%20icon.png"
	);
	assert_eq!(
		manifest["screenshots"][0]["src"],
		"../images/screenshots/home.png"
	);
	assert_eq!(
		manifest["shortcuts"][0]["icons"][0]["src"],
		"../images/icons/shortcut.png"
	);
	assert_eq!(
		manifest["file_handlers"][0]["icons"][0]["src"],
		"../images/icons/file.png"
	);
	assert_eq!(
		packed.manifest().assets["manifest.webmanifest"].dependencies,
		[
			"icons/app icon.png",
			"icons/file.png",
			"icons/shortcut.png",
			"screenshots/home.png"
		]
	);
}

#[rstest]
fn web_manifest_preserves_origin_relative_app_ids() {
	// Arrange
	let manifest = br#"{"name":"Demo","id":"demo-app","start_url":"/app/"}"#;
	// Act
	let packed = pipeline(&[("manifest.webmanifest", manifest)])
		.prepare(AssetMode::Production)
		.unwrap();
	let output = String::from_utf8(packed.read_asset("manifest.webmanifest").unwrap()).unwrap();
	// Assert
	let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
	assert_eq!(parsed["id"], "demo-app");
	assert_eq!(parsed["start_url"], "/app/");
}

#[rstest]
fn web_manifest_rejects_relative_file_and_protocol_handler_urls() {
	// Arrange
	let file_handler = br#"{"name":"Demo","start_url":"/","file_handlers":[{"action":"open/"}]}"#;
	let protocol_handler = br#"{"name":"Demo","start_url":"/","protocol_handlers":[{"protocol":"web+tea","url":"handlers/?url=%s"}]}"#;
	// Act
	let file_error = pipeline(&[("manifest.webmanifest", file_handler)])
		.prepare(AssetMode::Production)
		.unwrap_err();
	let protocol_error = pipeline(&[("manifest.webmanifest", protocol_handler)])
		.prepare(AssetMode::Production)
		.unwrap_err();
	// Assert
	assert!(file_error.to_string().contains("/file_handlers/0/action"));
	assert!(
		protocol_error
			.to_string()
			.contains("/protocol_handlers/0/url")
	);
}

#[rstest]
fn web_manifest_rejects_relative_navigation_urls_that_change_when_relocated() {
	// Arrange
	let manifest = br#"{"name":"Demo","start_url":"./app/"}"#;
	// Act
	let result = pipeline(&[("manifest.webmanifest", manifest)]).prepare(AssetMode::Production);
	// Assert
	assert!(
		result
			.unwrap_err()
			.to_string()
			.contains("relative Web App Manifest URL /start_url")
	);
}
