use std::borrow::Cow;

use reinhardt_pages::{CssColor, CssLength, style_def};
use rstest::rstest;

#[style_def]
pub(crate) static STYLES: RuntimeStyles = style! {
	vars {
		accent: Color = red;
		padding: Length = 1rem;
	}
	.card {
		color: vars.accent;
		padding: vars.padding;
	}
};

#[rstest]
fn generated_accessors_and_variables_use_locked_scope_names() {
	// Arrange
	let accent = CssColor::parse("blue").expect("blue should be a checked color");

	// Act
	let class = STYLES.card();
	let vars: Cow<'static, str> = STYLES
		.vars()
		.padding(CssLength::px(8.0))
		.accent(accent)
		.into();

	// Assert
	assert_eq!(class.as_str(), "card--rs-18609b3d54af");
	assert_eq!(
		vars,
		"--rs-18609b3d54af-accent: blue;--rs-18609b3d54af-padding: 8px;"
	);
}
