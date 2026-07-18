use std::fs;

use reinhardt_commands::{StyleExtractor, StylePackageContext};
use reinhardt_pages::style_def;
use rstest::rstest;

#[style_def]
static PARITY_STYLES: ParityStyles = style! {
	.card {
		color: red;
	}
};

#[rstest]
fn macro_and_extractor_share_scoped_class_names() {
	let directory = tempfile::tempdir().expect("create temporary package");
	fs::create_dir(directory.path().join("src")).unwrap();
	fs::write(
		directory.path().join("Cargo.toml"),
		format!(
			"[package]\nname = {:?}\nversion = {:?}\nedition = \"2024\"\n",
			env!("CARGO_PKG_NAME"),
			env!("CARGO_PKG_VERSION")
		),
	)
	.unwrap();
	fs::write(
		directory.path().join("src/lib.rs"),
		"#[style_def]\nstatic PARITY_STYLES: ParityStyles = style! { .card { color: red; } };\n",
	)
	.unwrap();
	let context = StylePackageContext::resolve(directory.path().join("Cargo.toml"), None)
		.expect("select parity package");

	let bundle = StyleExtractor::new(context)
		.extract()
		.expect("extract parity style");

	assert_eq!(bundle.definitions.len(), 1);
	assert_eq!(bundle.definitions[0].compiled.classes.len(), 1);
	assert_eq!(
		PARITY_STYLES.card().as_str(),
		bundle.definitions[0].compiled.classes[0].css_name
	);
}
