use std::borrow::Cow;

use reinhardt_pages::{CssColor, CssLength, CssNumber, style_def};
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

#[style_def]
pub(crate) static RANGE_STYLES: RuntimeRangeStyles = style! {
	vars {
		weight: Number = 400;
	}
	.card {
		font-weight: vars.weight;
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
	assert_eq!(class.as_str(), "card--rs-cde6a1fbe15e");
	assert_eq!(
		vars,
		"--rs-cde6a1fbe15e-accent: blue;--rs-cde6a1fbe15e-padding: 8px;"
	);
}

#[rstest]
fn generated_variable_setters_preserve_nonnegative_property_constraints() {
	// Arrange
	let negative_padding = CssLength::px(-1.0);

	// Act
	let result = std::panic::catch_unwind(|| {
		let _: Cow<'static, str> = STYLES.vars().padding(negative_padding).into();
	});

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn generated_variable_setters_preserve_numeric_range_property_constraints() {
	// Arrange
	let too_heavy = CssNumber::new(1001.0);

	// Act
	let class = RANGE_STYLES.card();
	let result = std::panic::catch_unwind(|| {
		let _: Cow<'static, str> = RANGE_STYLES.vars().weight(too_heavy).into();
	});

	// Assert
	assert_eq!(class.as_str(), "card--rs-e97db4180022");
	assert!(result.is_err());
}
