use reinhardt_manouche::{StyleCompileContext, compile_style, serialize_css};
use rstest::rstest;

const REPRESENTATIVE_SOURCE: &str = r#"
	globals {
		border: Color;
		surface_secondary: Color;
	}
	vars {
		padding: Length = 1rem;
		gutter: LengthPercentage = 1rem;
		accent: Color = globals.surface_secondary;
		offset: LengthPercentage = 0px;
	}
	.card {
		padding: vars.padding;
		border: (1px, solid, globals.border);
		width: max(0, 100% - vars.gutter * 2);
		color: vars.accent.mix(white, 15%);
		background-image: linear_gradient(Direction::Right, [
			stop(vars.accent, 0%),
			stop(vars.accent.mix(black, 20%), 100%),
		]);
		transform: (translate_x(vars.offset), rotate(6deg), scale(1.05));
		border-radius: slash(0.5rem, 1rem);
	}
	@media (max-width: 640px) {
		.card { padding: 0.75rem; }
	}
"#;

const CASCADE_ORDER_SOURCE: &str = r#"
	.card {
		color: red;
		&:hover { color: blue; }
		background-color: white;
		@media (max-width: 640px) { width: 100%; }
		opacity: 1;
	}
"#;

const VARIABLES_SOURCE: &str = r#"
	vars {
		base: Length = 1rem;
		gap: Length = vars.base;
		inset: Length = vars.gap;
	}
	.card {
		padding: vars.inset;
		margin: vars.gap;
	}
"#;

const SELECTOR_LIST_SOURCE: &str = r#"
	.card, .panel {
		&:hover, &.featured { color: red; }
		.label, > button { color: blue; }
		&:is(.active, [aria-current = "page" i]) { color: green; }
		&:nth-child(2n + 1 of .row) { color: black; }
	}
"#;

fn context() -> StyleCompileContext<'static> {
	StyleCompileContext {
		package_name: "poll-app",
		package_version: "0.4.0",
		style_type_name: "PollCardStyles",
	}
}

fn compile_css(source: &str) -> String {
	let input = source
		.parse()
		.expect("style fixture source should tokenize");
	let compiled = compile_style(input, &context()).expect("style fixture should compile");
	serialize_css(&compiled.css)
}

#[rstest]
#[case(REPRESENTATIVE_SOURCE, include_str!("fixtures/style/representative.css"))]
#[case(CASCADE_ORDER_SOURCE, include_str!("fixtures/style/cascade-order.css"))]
#[case(VARIABLES_SOURCE, include_str!("fixtures/style/variables.css"))]
#[case(SELECTOR_LIST_SOURCE, include_str!("fixtures/style/selector-list.css"))]
fn compiled_css_matches_checked_in_fixture(#[case] source: &str, #[case] expected: &str) {
	// Arrange
	let first = compile_css(source);

	// Act
	let second = compile_css(source);

	// Assert
	assert_eq!(first, expected);
	assert_eq!(second, expected);
	assert!(first.ends_with('\n'));
	assert!(!first.ends_with("\n\n"));
}

#[rstest]
fn representative_pipeline_publishes_metadata_and_canonical_css() {
	// Arrange
	let input = REPRESENTATIVE_SOURCE
		.parse()
		.expect("representative source should tokenize");

	// Act
	let compiled = compile_style(input, &context()).expect("representative source should compile");
	let css = serialize_css(&compiled.css);

	// Assert
	assert_eq!(compiled.classes.len(), 1);
	assert_eq!(compiled.globals.len(), 2);
	assert_eq!(compiled.variables.len(), 4);
	assert!(css.contains("linear-gradient("));
	assert!(css.contains("translateX("));
	assert_eq!(css.matches("width: max(0, calc(").count(), 1);
	assert!(css.find("padding:").unwrap() < css.find("border:").unwrap());
}

#[rstest]
fn empty_style_serializes_to_zero_bytes() {
	// Arrange
	let input = "".parse().expect("empty style should tokenize");

	// Act
	let compiled = compile_style(input, &context()).expect("empty style should compile");
	let css = serialize_css(&compiled.css);

	// Assert
	assert_eq!(css, "");
}
