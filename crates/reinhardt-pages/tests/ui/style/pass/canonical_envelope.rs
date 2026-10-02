use std::borrow::Cow;

use reinhardt_pages::{CssColor, CssLength, style_def};

#[style_def]
static STYLES: CardStyles = style! {
	vars {
		accent: Color = red;
		padding: Length = 1rem;
	}
	.card {
		color: vars.accent;
	}
	.featured {
		color: vars.accent;
	}
};

fn main() {
	let classes: Cow<'static, str> = (STYLES.card() + STYLES.featured() + "legacy").into();
	let variables: Cow<'static, str> = STYLES
		.vars()
		.padding(CssLength::rem(0.75))
		.accent(CssColor::parse("blue").unwrap())
		.into();
	assert!(!classes.is_empty());
	assert!(!variables.is_empty());
}
