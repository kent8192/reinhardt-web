use std::fs;
use std::path::{Path, PathBuf};

use reinhardt_commands::{StyleExtractor, StylePackageContext};
use rstest::rstest;

fn write_package(root: &Path, source: &str) -> PathBuf {
	fs::create_dir_all(root.join("src")).expect("create source directory");
	fs::write(
		root.join("Cargo.toml"),
		"[package]\nname = \"poll-app\"\nversion = \"0.4.0\"\nedition = \"2024\"\n",
	)
	.expect("write package manifest");
	fs::write(root.join("src/lib.rs"), source).expect("write package source");
	root.join("Cargo.toml")
}

fn canonical_style(color: &str, extra_class: &str) -> String {
	format!(
		"#[style_def]\nstatic STYLES: CardStyles = style! {{\n\t.card {{ color: {color}; }}\n\t{extra_class}\n}};\n"
	)
}

#[rstest]
fn package_selection_uses_the_metadata_root_package() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(directory.path(), "pub fn value() -> usize { 1 }\n");

	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	assert_eq!(context.package_name, "poll-app");
	assert_eq!(context.package_version, "0.4.0");
	assert_eq!(context.package_manifest_path, manifest);
}

#[rstest]
fn scanner_builds_a_deterministic_bundle_and_three_fingerprints() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(directory.path(), &canonical_style("red", ""));
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");
	let extractor = StyleExtractor::new(context.clone());

	let first = extractor.extract().expect("extract first bundle");
	let repeated = extractor.extract().expect("extract repeated bundle");
	assert_eq!(first.css, repeated.css);
	assert_eq!(first.fingerprints, repeated.fingerprints);
	assert_eq!(first.definitions.len(), 1);
	assert!(
		String::from_utf8(first.css.clone())
			.unwrap()
			.contains(".card--rs-")
	);

	fs::write(
		directory.path().join("src/lib.rs"),
		canonical_style("blue", ""),
	)
	.expect("change declaration");
	let declaration_change = StyleExtractor::new(context.clone())
		.extract()
		.expect("extract declaration change");
	assert_eq!(
		first.fingerprints.non_style_rust,
		declaration_change.fingerprints.non_style_rust
	);
	assert_eq!(
		first.fingerprints.generated_api,
		declaration_change.fingerprints.generated_api
	);
	assert_ne!(first.fingerprints.css, declaration_change.fingerprints.css);

	fs::write(
		directory.path().join("src/lib.rs"),
		canonical_style("blue", ".label { color: blue; }"),
	)
	.expect("add class");
	let api_change = StyleExtractor::new(context)
		.extract()
		.expect("extract API change");
	assert_ne!(
		declaration_change.fingerprints.generated_api,
		api_change.fingerprints.generated_api
	);
}

#[rstest]
fn scanner_rejects_noncanonical_style_envelopes() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(
		directory.path(),
		"#[crate::style_def]\nstatic STYLES: CardStyles = style! { .card { color: red; } };\n",
	);
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let error = StyleExtractor::new(context)
		.extract()
		.expect_err("qualified attribute must fail");

	assert!(error.contains("canonical envelope"));
}

#[rstest]
fn scanner_returns_empty_css_after_the_final_definition_is_removed() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(directory.path(), "pub fn value() -> usize { 1 }\n");
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("extract empty bundle");

	assert!(bundle.css.is_empty());
	assert!(bundle.definitions.is_empty());
}
