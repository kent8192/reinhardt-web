use std::{borrow::Cow, fmt::Write as _};

use reinhardt_pages::{CssColor, CssLength, CssNumber, style_def};
use rstest::rstest;
use sha2::{Digest, Sha256};

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

fn expected_scope_suffix(style_type_name: &str) -> String {
	let identity = format!(
		"rstyle-v2\0{}\0{}\0{style_type_name}",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION")
	);
	let digest = Sha256::digest(identity.as_bytes());
	let mut suffix = String::with_capacity(12);
	for byte in &digest[..6] {
		write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
	}
	suffix
}

#[rstest]
fn generated_accessors_and_variables_use_locked_scope_names() {
	// Arrange
	let accent = CssColor::parse("blue").expect("blue should be a checked color");
	let scope_suffix = expected_scope_suffix("RuntimeStyles");
	let expected_class = format!("card--rs-{scope_suffix}");
	let expected_vars =
		format!("--rs-{scope_suffix}-accent: blue;--rs-{scope_suffix}-padding: 8px;");

	// Act
	let class = STYLES.card();
	let vars: Cow<'static, str> = STYLES
		.vars()
		.padding(CssLength::px(8.0))
		.accent(accent)
		.into();

	// Assert
	assert_eq!(class.as_str(), expected_class);
	assert_eq!(vars.as_ref(), expected_vars);
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
	let expected_class = format!("card--rs-{}", expected_scope_suffix("RuntimeRangeStyles"));

	// Act
	let class = RANGE_STYLES.card();
	let result = std::panic::catch_unwind(|| {
		let _: Cow<'static, str> = RANGE_STYLES.vars().weight(too_heavy).into();
	});

	// Assert
	assert_eq!(class.as_str(), expected_class);
	assert!(result.is_err());
}
