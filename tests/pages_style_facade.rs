#![cfg(feature = "pages")]

use reinhardt::pages::{ClassToken, CssColor, style_def};
use rstest::rstest;

#[style_def]
pub(crate) static STYLES: FacadeStyles = style! {
	.card {
		color: red;
	}
};

#[rstest]
fn style_api_resolves_through_the_root_pages_facade() {
	let token: ClassToken = STYLES.card();
	let color = CssColor::parse("red").expect("red is a checked color");
	assert!(!token.as_str().is_empty());
	assert_eq!(color.to_string(), "red");
}
