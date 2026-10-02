use reinhardt_utils::staticfiles::publication::{
	AssetCategory, AssetClassifier, AssetProducer, validate_assignments,
};
use rstest::rstest;

#[rstest]
#[case(
	"images/brand/logo.SVG",
	AssetProducer::Static,
	"vectors/brand/logo.SVG"
)]
#[case("css/admin/theme.css", AssetProducer::Static, "css/admin/theme.css")]
#[case(
	"styles/theme.css.map.br",
	AssetProducer::Static,
	"css/styles/theme.css.map.br"
)]
#[case("app.js.gz", AssetProducer::Static, "js/app.js.gz")]
#[case("backup.tar.gz", AssetProducer::Static, "other/backup.tar.gz")]
#[case("README", AssetProducer::Static, "other/README")]
#[case("film.MP4", AssetProducer::Static, "videos/film.MP4")]
#[case("logo.svgz", AssetProducer::Static, "vectors/logo.svgz")]
#[case("photo.avif", AssetProducer::Static, "images/photo.avif")]
#[case("sound.opus", AssetProducer::Static, "audio/sound.opus")]
#[case("body.woff2", AssetProducer::Static, "fonts/body.woff2")]
#[case("app_bg.wasm", AssetProducer::Pages, "pages/app_bg.wasm")]
#[case("app.js", AssetProducer::Pages, "pages/app.js")]
#[case(
	"snippets/app/util.js",
	AssetProducer::Pages,
	"pages/snippets/app/util.js"
)]
#[case("snippets/logo.svg", AssetProducer::Pages, "pages/snippets/logo.svg")]
#[case("app.js.map", AssetProducer::Pages, "pages/app.js.map")]
#[case("app_bg.wasm", AssetProducer::Static, "other/app_bg.wasm")]
#[case(
	"__reinhardt__/components.css",
	AssetProducer::Static,
	"css/__reinhardt__/components.css"
)]
fn classification_keeps_namespaces_and_pages_companions(
	#[case] logical: &str,
	#[case] producer: AssetProducer,
	#[case] expected: &str,
) {
	// Arrange
	let classifier = AssetClassifier::default();
	// Act
	let (_, path) = classifier.classify(logical, producer).unwrap();
	// Assert
	assert_eq!(path, expected);
}

#[rstest]
fn explicit_extension_mapping_overrides_builtin_mapping_but_not_pages() {
	// Arrange
	let mut classifier = AssetClassifier::default();
	classifier
		.register_extension("json", AssetCategory::Vectors)
		.unwrap();
	// Act
	let normal = classifier
		.classify("lottie.JSON", AssetProducer::Static)
		.unwrap();
	let pages = classifier
		.classify("lottie.JSON", AssetProducer::Pages)
		.unwrap();
	// Assert
	assert_eq!(normal.1, "vectors/lottie.JSON");
	assert_eq!(pages.1, "pages/lottie.JSON");
	assert!(
		classifier
			.register_extension("json", AssetCategory::Images)
			.is_err()
	);
	assert!(
		classifier
			.register_extension("custom", AssetCategory::Pages)
			.is_err()
	);
}

#[rstest]
#[case(vec![("a".into(), "images/logo.png".into()), ("b".into(), "images/logo.png".into())])]
#[case(vec![("a".into(), "images/Logo.png".into()), ("b".into(), "images/logo.PNG".into())])]
#[case(vec![("a".into(), "images/a.png".into()), ("a".into(), "images/b.png".into())])]
fn collisions_report_both_assignments(#[case] assignments: Vec<(String, String)>) {
	// Arrange & Act
	let error = validate_assignments(&assignments).unwrap_err();
	// Assert
	assert!(error.to_string().contains(&assignments[0].0));
	assert!(error.to_string().contains(&assignments[1].0));
}

#[rstest]
#[case("/absolute.png")]
#[case("C:/drive.png")]
#[case("../outside.png")]
#[case("images/../../outside.png")]
#[case("images\\outside.png")]
#[case("images//logo.png")]
fn unsafe_source_names_fail_before_output_assignment(#[case] logical: &str) {
	// Arrange & Act
	let result = AssetClassifier::default().classify(logical, AssetProducer::Static);
	// Assert
	assert!(result.is_err());
}
