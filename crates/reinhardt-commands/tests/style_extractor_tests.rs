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

fn write_workspace_with_duplicate_package_name(root: &Path) -> PathBuf {
	let workspace = root.join("workspace");
	fs::create_dir_all(&workspace).expect("create workspace directory");
	fs::write(
		workspace.join("Cargo.toml"),
		"[workspace]\nmembers = [\"app\"]\nresolver = \"3\"\n",
	)
	.expect("write workspace manifest");
	fs::create_dir_all(workspace.join("app/src"))
		.expect("create workspace member source directory");
	fs::write(
		workspace.join("app/Cargo.toml"),
		"[package]\nname = \"foo\"\nversion = \"0.4.0\"\nedition = \"2024\"\n\n[dependencies]\nforeign_foo = { package = \"foo\", path = \"../../foreign\" }\n",
	)
	.expect("write workspace member manifest");
	fs::write(workspace.join("app/src/lib.rs"), "pub fn app() {}\n")
		.expect("write workspace member source");
	fs::create_dir_all(root.join("foreign/src")).expect("create foreign package source directory");
	fs::write(
		root.join("foreign/Cargo.toml"),
		"[package]\nname = \"foo\"\nversion = \"0.5.0\"\nedition = \"2024\"\n",
	)
	.expect("write foreign package manifest");
	fs::write(root.join("foreign/src/lib.rs"), "pub fn foreign() {}\n")
		.expect("write foreign package source");
	workspace.join("Cargo.toml")
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
fn package_selection_ignores_transitive_dependencies_with_the_same_name() {
	let directory = tempfile::tempdir().expect("create temporary workspace");
	let manifest = write_workspace_with_duplicate_package_name(directory.path());

	let context =
		StylePackageContext::resolve(&manifest, Some("foo")).expect("select workspace member");

	assert_eq!(
		context.package_manifest_path,
		directory.path().join("workspace/app/Cargo.toml")
	);
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
fn scanner_ignores_cfg_disabled_style_definitions() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(
		directory.path(),
		r#"
#[cfg(any())]
#[style_def]
static DISABLED: DisabledStyles = style! { .disabled { color: red; } };

#[style_def]
static ENABLED: EnabledStyles = style! { .enabled { color: blue; } };
"#,
	);
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("extract only active definitions");

	assert_eq!(bundle.definitions.len(), 1);
	assert_eq!(bundle.definitions[0].style_type_name, "EnabledStyles");
}

#[rstest]
fn scanner_includes_style_definitions_enabled_only_for_the_wasm_target() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(
		directory.path(),
		r#"
#[cfg(target_family = "wasm")]
#[style_def]
static CLIENT: ClientStyles = style! { .client { color: blue; } };
"#,
	);
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("extract styles compiled for the WASM Pages target");

	assert_eq!(bundle.definitions.len(), 1);
	assert_eq!(bundle.definitions[0].style_type_name, "ClientStyles");
}

#[rstest]
fn scanner_follows_compiled_modules_and_ignores_unreferenced_source_files() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(directory.path(), "mod styles;\n");
	fs::write(
		directory.path().join("src/styles.rs"),
		"#[style_def] static STYLES: Styles = style! { .card { color: red; } };\n",
	)
	.expect("write compiled module");
	fs::write(
		directory.path().join("src/old.rs"),
		"static STALE: StaleStyles = style! { .stale { color: red; } };\n",
	)
	.expect("write uncompiled source");
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("ignore uncompiled scratch source");

	assert_eq!(bundle.definitions.len(), 1);
	assert_eq!(bundle.definitions[0].style_type_name, "Styles");
}

#[rstest]
fn scanner_resolves_path_modules_inside_inline_modules_from_the_module_directory() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(
		directory.path(),
		r#"
mod ui {
    #[path = "styles.rs"]
    mod styles;
}
"#,
	);
	fs::create_dir_all(directory.path().join("src/ui")).expect("create inline module directory");
	fs::write(
		directory.path().join("src/ui/styles.rs"),
		"#[style_def] static STYLES: NestedStyles = style! { .card { color: red; } };\n",
	)
	.expect("write nested path module");
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("extract nested path module from the inline module directory");

	assert_eq!(bundle.definitions.len(), 1);
	assert_eq!(bundle.definitions[0].style_type_name, "NestedStyles");
}

#[rstest]
fn scanner_extracts_only_the_library_target_served_by_pages() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(directory.path(), &canonical_style("red", ""));
	fs::create_dir_all(directory.path().join("src/bin")).expect("create binary source directory");
	fs::write(
		directory.path().join("src/bin/admin.rs"),
		canonical_style("blue", ""),
	)
	.expect("write binary-only style definition");
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("extract only styles compiled for the library target");

	assert_eq!(bundle.definitions.len(), 1);
	assert_eq!(bundle.definitions[0].style_type_name, "CardStyles");
	assert!(bundle.definitions[0].source_path.ends_with("src/lib.rs"));
}

#[rstest]
fn scanner_ignores_qualified_foreign_style_macros() {
	let directory = tempfile::tempdir().expect("create temporary package");
	let manifest = write_package(
		directory.path(),
		"static THEME: Theme = other_crate::style! { .theme { color: red; } };\n",
	);
	let context = StylePackageContext::resolve(&manifest, None).expect("select root package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("ignore a foreign style macro");

	assert!(bundle.definitions.is_empty());
	assert!(bundle.css.is_empty());
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
