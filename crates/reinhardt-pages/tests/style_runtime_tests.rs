use std::borrow::Cow;

use reinhardt_pages::{
	ClassList, ClassToken, CssAngle, CssColor, CssInteger, CssLength, CssLengthPercentage,
	CssNumber, CssPercentage, CssTime, StyleVars,
};
use rstest::rstest;

#[rstest]
#[case(CssLength::px(12.5).to_string(), "12.5px")]
#[case(CssLength::rem(0.75).to_string(), "0.75rem")]
#[case(CssPercentage::new(15.0).to_string(), "15%")]
#[case(CssAngle::deg(6.0).to_string(), "6deg")]
#[case(CssAngle::grad(6.0).to_string(), "6grad")]
#[case(CssAngle::rad(6.0).to_string(), "6rad")]
#[case(CssAngle::turn(0.5).to_string(), "0.5turn")]
#[case(CssTime::ms(150.0).to_string(), "150ms")]
#[case(CssTime::s(1.5).to_string(), "1.5s")]
#[case(CssNumber::new(1.0).to_string(), "1")]
#[case(CssNumber::new(-1.25).to_string(), "-1.25")]
#[case(CssInteger::new(-2).to_string(), "-2")]
fn typed_values_serialize_exactly(#[case] actual: String, #[case] expected: &str) {
	// Arrange and Act are supplied by the parameterized constructors.

	// Assert
	assert_eq!(actual, expected);
}

#[rstest]
#[case("CssLength", || { CssLength::px(f64::NAN); })]
#[case("CssLength", || { CssLength::px(f64::INFINITY); })]
#[case("CssPercentage", || { CssPercentage::new(f64::NEG_INFINITY); })]
#[case("CssAngle", || { CssAngle::deg(f64::NAN); })]
#[case("CssTime", || { CssTime::ms(f64::INFINITY); })]
#[case("CssNumber", || { CssNumber::new(f64::NEG_INFINITY); })]
fn floating_wrappers_reject_non_finite_values(#[case] wrapper: &str, #[case] construct: fn()) {
	// Arrange
	let expected = format!("{wrapper} rejects non-finite values");

	// Act
	let panic = std::panic::catch_unwind(construct).expect_err("non-finite input should panic");
	let message = panic
		.downcast_ref::<String>()
		.map(String::as_str)
		.or_else(|| panic.downcast_ref::<&str>().copied())
		.expect("panic should contain a string message");

	// Assert
	assert_eq!(message, expected);
}

#[rstest]
#[case("#abc", "#abc")]
#[case("#AbCd", "#abcd")]
#[case("#AABBCC", "#aabbcc")]
#[case("#AABBCCDD", "#aabbccdd")]
#[case("rebeccapurple", "rebeccapurple")]
#[case("TRANSPARENT", "transparent")]
fn css_color_parse_accepts_checked_values(#[case] input: &str, #[case] expected: &str) {
	// Arrange and Act
	let color = CssColor::parse(input).expect("checked color should parse");

	// Assert
	assert_eq!(color.to_string(), expected);
}

#[rstest]
#[case("red; background: blue")]
#[case("unknown-color")]
#[case("#12")]
#[case("var(--color)")]
fn css_color_parse_rejects_unchecked_text(#[case] input: &str) {
	// Arrange and Act
	let result = CssColor::parse(input);

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn length_percentage_converts_without_reparsing() {
	// Arrange
	let length = CssLength::px(8.0);
	let percentage = CssPercentage::new(25.0);

	// Act
	let length_value = CssLengthPercentage::from(length);
	let percentage_value = CssLengthPercentage::from(percentage);

	// Assert
	assert_eq!(length_value.to_string(), "8px");
	assert_eq!(percentage_value.to_string(), "25%");
}

#[rstest]
fn every_registered_length_unit_has_a_fixed_constructor() {
	// Arrange
	let values = [
		CssLength::px(1.0).to_string(),
		CssLength::cm(1.0).to_string(),
		CssLength::mm(1.0).to_string(),
		CssLength::q(1.0).to_string(),
		CssLength::inches(1.0).to_string(),
		CssLength::pc(1.0).to_string(),
		CssLength::pt(1.0).to_string(),
		CssLength::em(1.0).to_string(),
		CssLength::rem(1.0).to_string(),
		CssLength::ex(1.0).to_string(),
		CssLength::rex(1.0).to_string(),
		CssLength::cap(1.0).to_string(),
		CssLength::rcap(1.0).to_string(),
		CssLength::ch(1.0).to_string(),
		CssLength::rch(1.0).to_string(),
		CssLength::ic(1.0).to_string(),
		CssLength::ric(1.0).to_string(),
		CssLength::lh(1.0).to_string(),
		CssLength::rlh(1.0).to_string(),
		CssLength::vw(1.0).to_string(),
		CssLength::vh(1.0).to_string(),
		CssLength::vi(1.0).to_string(),
		CssLength::vb(1.0).to_string(),
		CssLength::vmin(1.0).to_string(),
		CssLength::vmax(1.0).to_string(),
		CssLength::svw(1.0).to_string(),
		CssLength::svh(1.0).to_string(),
		CssLength::svi(1.0).to_string(),
		CssLength::svb(1.0).to_string(),
		CssLength::svmin(1.0).to_string(),
		CssLength::svmax(1.0).to_string(),
		CssLength::lvw(1.0).to_string(),
		CssLength::lvh(1.0).to_string(),
		CssLength::lvi(1.0).to_string(),
		CssLength::lvb(1.0).to_string(),
		CssLength::lvmin(1.0).to_string(),
		CssLength::lvmax(1.0).to_string(),
		CssLength::dvw(1.0).to_string(),
		CssLength::dvh(1.0).to_string(),
		CssLength::dvi(1.0).to_string(),
		CssLength::dvb(1.0).to_string(),
		CssLength::dvmin(1.0).to_string(),
		CssLength::dvmax(1.0).to_string(),
		CssLength::cqw(1.0).to_string(),
		CssLength::cqh(1.0).to_string(),
		CssLength::cqi(1.0).to_string(),
		CssLength::cqb(1.0).to_string(),
		CssLength::cqmin(1.0).to_string(),
		CssLength::cqmax(1.0).to_string(),
	];
	let expected = [
		"1px", "1cm", "1mm", "1q", "1in", "1pc", "1pt", "1em", "1rem", "1ex", "1rex", "1cap",
		"1rcap", "1ch", "1rch", "1ic", "1ric", "1lh", "1rlh", "1vw", "1vh", "1vi", "1vb", "1vmin",
		"1vmax", "1svw", "1svh", "1svi", "1svb", "1svmin", "1svmax", "1lvw", "1lvh", "1lvi",
		"1lvb", "1lvmin", "1lvmax", "1dvw", "1dvh", "1dvi", "1dvb", "1dvmin", "1dvmax", "1cqw",
		"1cqh", "1cqi", "1cqb", "1cqmin", "1cqmax",
	];

	// Act
	let rendered: Vec<&str> = values.iter().map(String::as_str).collect();

	// Assert
	assert_eq!(rendered, expected);
}

#[rstest]
fn class_composition_preserves_order_duplicates_and_legacy_bytes() {
	// Arrange
	let card = ClassToken::new("card--rs-scope");
	let featured = ClassToken::new("featured--rs-scope");

	// Act
	let classes = card + featured + card + "Legacy_Class" + "";

	// Assert
	assert_eq!(
		classes.as_str(),
		"card--rs-scope featured--rs-scope card--rs-scope Legacy_Class"
	);
}

#[rstest]
fn class_tokens_borrow_and_class_lists_own_cow_values() {
	// Arrange
	let token = ClassToken::new("card--rs-scope");
	let list: ClassList = token + "legacy";

	// Act
	let token_cow: Cow<'static, str> = token.into();
	let list_cow: Cow<'static, str> = list.into();

	// Assert
	assert_eq!(token_cow, Cow::Borrowed("card--rs-scope"));
	assert_eq!(list_cow, Cow::Owned::<str>("card--rs-scope legacy".into()));
}

#[rstest]
fn style_vars_replace_slots_and_serialize_in_source_order() {
	// Arrange
	let mut vars = StyleVars::with_slots(3);

	// Act
	vars.set(2, "--rs-scope-third", CssInteger::new(3));
	vars.set(0, "--rs-scope-first", CssInteger::new(1));
	vars.set(2, "--rs-scope-third", CssInteger::new(4));
	let css: Cow<'static, str> = vars.into();

	// Assert
	assert_eq!(css, "--rs-scope-first: 1;--rs-scope-third: 4;");
}

#[rstest]
fn empty_style_vars_borrow_an_empty_value() {
	// Arrange
	let vars = StyleVars::with_slots(2);

	// Act
	let css: Cow<'static, str> = vars.into();

	// Assert
	assert_eq!(css, Cow::Borrowed(""));
}
