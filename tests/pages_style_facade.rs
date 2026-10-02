#![cfg(feature = "pages")]

use reinhardt::pages::{ClassToken, CssColor, style_def};
use rstest::rstest;

#[style_def]
pub(crate) static STYLES: FacadeStyles = style! {
	.card {
		color: red;
	}
};

#[style_def]
pub(crate) static SECOND_STYLES: SecondFacadeStyles = style! {
	.label {
		color: blue;
	}
};

#[rstest]
fn style_api_resolves_through_the_root_pages_facade() {
	let token: ClassToken = STYLES.card();
	let second_token: ClassToken = SECOND_STYLES.label();
	let color = CssColor::parse("red").expect("red is a checked color");
	assert!(!token.as_str().is_empty());
	assert!(!second_token.as_str().is_empty());
	assert_eq!(color.to_string(), "red");
}
