//! Property-based tests for watch syntax in page! macro
//!
//! Uses proptest to verify:
//! 1. Arbitrary boolean conditions produce valid Views
//! 2. Arbitrary string content is properly escaped
//! 3. Arbitrary nesting depths produce valid Views
//! 4. View structure invariants are maintained
#![cfg(not(target_arch = "wasm32"))]
use proptest::prelude::*;
use reinhardt_pages::component::Page;
use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;
use serial_test::serial;
proptest! {
	#![proptest_config(ProptestConfig::with_cases(50))] #[doc =
	" Property: Any boolean condition should produce a valid View"] #[test]
	#[serial(reactive)] fn test_watch_arbitrary_boolean_condition(condition in any::<
	bool > ()) { let signal = Signal::new(condition); let view = page!(| signal : Signal
	< bool >| { div { watch { if signal.get() { span { "True" } } else { span { "False" }
	} } } }) (signal.clone()); match & view { Page::Element(el) => { prop_assert_eq!(el
	.tag_name(), "div"); prop_assert!(! el.child_views().is_empty(),
	"Watch should produce child"); } _ => prop_assert!(false, "Expected Page::Element"),
	} let html = view.render_to_string(); prop_assert!(! html.is_empty());
	prop_assert!(html.contains("<div>")); prop_assert!(html.contains("</div>")); if
	condition { prop_assert!(html.contains("True"),
	"True branch should render when condition is true"); prop_assert!(! html
	.contains("False"), "False branch should not render when condition is true"); } else
	{ prop_assert!(html.contains("False"),
	"False branch should render when condition is false"); prop_assert!(! html
	.contains("True"), "True branch should not render when condition is false"); } }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(50))] #[doc =
	" Property: Alphanumeric content should be rendered correctly"] #[test]
	#[serial(reactive)] fn test_watch_arbitrary_alphanumeric_content(content in
	"[a-zA-Z0-9 ]{1,100}") { let signal = Signal::new(content.clone()); let view =
	page!(| signal : Signal < String >| { div { watch { span { { signal.get() } } } } })
	(signal.clone()); let html = view.render_to_string(); prop_assert!(html.contains(&
	content), "Content should be present in HTML"); prop_assert!(html.contains("<span>"),
	"Should have span opening tag"); prop_assert!(html.contains("</span>"),
	"Should have span closing tag"); }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(30))] #[doc =
	" Property: Content with special HTML characters should be properly escaped"] #[test]
	#[serial(reactive)] fn test_watch_content_escaping_property(content in ".*") { let
	signal = Signal::new(content.clone()); let view = page!(| signal : Signal < String >|
	{ div { watch { { signal.get() } } } }) (signal.clone()); let html = view
	.render_to_string(); if content.contains('<') { prop_assert!(html.contains("&lt;"),
	"< should be escaped to &lt; in content"); } if content.contains('>') {
	prop_assert!(html.contains("&gt;"), "> should be escaped to &gt; in content"); } }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(20))] #[doc =
	" Property: Nesting depth should not affect validity"] #[test] #[serial(reactive)] fn
	test_watch_arbitrary_nesting_depth(depth in 0u8..5) { let signal = Signal::new(true);
	let view = match depth { 0 => page!(| signal : Signal < bool >| { div { watch { if
	signal.get() { "Depth 0" } } } }) (signal.clone()), 1 => page!(| signal : Signal <
	bool >| { div { div { watch { if signal.get() { "Depth 1" } } } } }) (signal
	.clone()), 2 => page!(| signal : Signal < bool >| { div { div { div { watch { if
	signal.get() { "Depth 2" } } } } } }) (signal.clone()), 3 => page!(| signal : Signal
	< bool >| { div { div { div { div { watch { if signal.get() { "Depth 3" } } } } } }
	}) (signal.clone()), _ => page!(| signal : Signal < bool >| { div { div { div { div {
	div { watch { if signal.get() { "Depth 4" } } } } } } } }) (signal.clone()), }; match
	& view { Page::Element(el) => { prop_assert_eq!(el.tag_name(), "div"); } _ =>
	prop_assert!(false, "Expected Page::Element at any depth"), } let html = view
	.render_to_string(); prop_assert!(! html.is_empty()); let expected =
	format!("Depth {}", depth.min(4)); prop_assert!(html.contains(& expected),
	"Should contain depth marker"); }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(50))] #[doc =
	" Property: Integer values should be correctly formatted"] #[test]
	#[serial(reactive)] fn test_watch_integer_expression(value in any::< i32 > ()) { let
	signal = Signal::new(value); let view = page!(| signal : Signal < i32 >| { div {
	watch { { format!("Value: {}", signal.get()) } } } }) (signal.clone()); let html =
	view.render_to_string(); prop_assert!(html.contains(& format!("Value: {}", value)),
	"Integer value should be rendered correctly"); }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(30))] #[doc =
	" Property: Lists of any size should render correctly"] #[test] #[serial(reactive)]
	fn test_watch_list_size_property(size in 0usize..20) { let items : Vec < String > =
	(0..size).map(| i | format!("item-{}", i)).collect(); let signal = Signal::new(items
	.clone()); let view = page!(| signal : Signal < Vec < String >>| { ul { watch { for
	item in signal.get().iter() { li { { item.clone() } } } } } }) (signal.clone()); let
	html = view.render_to_string(); prop_assert!(html.contains("<ul>"));
	prop_assert!(html.contains("</ul>")); let li_count = html.matches("<li>").count();
	prop_assert_eq!(li_count, size, "Should have {} li elements", size); for item in &
	items { prop_assert!(html.contains(item), "Item {} should be present", item); } }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(30))] #[doc =
	" Property: Multiple signals should all be tracked correctly"] #[test]
	#[serial(reactive)] fn test_watch_multiple_signals_property(loading in any::< bool >
	(), error_text in prop::option::of("[a-zA-Z ]{1,20}")) { let loading_signal =
	Signal::new(loading); let error_signal = Signal::new(error_text.clone()); let view =
	page!(| loading : Signal < bool >, error : Signal < Option < String >>| { div { watch
	{ if loading.get() { span { class : "loading", "Loading..." } } } watch { if error
	.get().is_some() { span { class : "error", { error.get().unwrap_or_default() } } } }
	} }) (loading_signal.clone(), error_signal.clone()); let html = view
	.render_to_string(); if loading { prop_assert!(html.contains("Loading..."),
	"Loading should be shown when true"); } else { prop_assert!(! html
	.contains("Loading..."), "Loading should not be shown when false"); } if let Some(ref
	error) = error_text { prop_assert!(html.contains(error),
	"Error message should be shown"); } else { prop_assert!(! html
	.contains("class=\"error\""), "Error container should not exist when None"); } }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(50))] #[doc =
	" Property: Both branches of if-else should be mutually exclusive"] #[test]
	#[serial(reactive)] fn test_watch_condition_toggle_invariant(condition in any::< bool
	> ()) { let signal = Signal::new(condition); let view = page!(| signal : Signal <
	bool >| { div { watch { if signal.get() { span { id : "true-branch", "TRUE" } } else
	{ span { id : "false-branch", "FALSE" } } } } }) (signal.clone()); let html = view
	.render_to_string(); let has_true_branch = html.contains("id=\"true-branch\""); let
	has_false_branch = html.contains("id=\"false-branch\""); prop_assert!(has_true_branch
	^ has_false_branch, "Exactly one branch should be rendered, not both or neither");
	prop_assert_eq!(has_true_branch, condition, "True branch should match condition");
	prop_assert_eq!(has_false_branch, ! condition,
	"False branch should match inverted condition"); }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(30))] #[doc =
	" Property: Empty and non-empty content should both render valid HTML"] #[test]
	#[serial(reactive)] fn test_watch_empty_vs_nonempty_content(content in
	prop::option::of("[a-zA-Z0-9 ]{1,50}")) { let signal = Signal::new(content.clone()
	.unwrap_or_default()); let view = page!(| signal : Signal < String >| { div { class :
	"container", watch { { signal.get() } } } }) (signal.clone()); let html = view
	.render_to_string(); prop_assert!(html.contains("class=\"container\""));
	prop_assert!(html.starts_with("<div")); prop_assert!(html.ends_with("</div>")); if
	let Some(ref c) = content { prop_assert!(html.contains(c),
	"Non-empty content should be present"); } }
}
proptest! {
	#![proptest_config(ProptestConfig::with_cases(30))] #[doc =
	" Property: page! macro always produces Page::Element at top level"] #[test]
	#[serial(reactive)] fn test_watch_view_variant_consistency(show_content in any::<
	bool > (), content in "[a-zA-Z ]{0,20}") { let show = Signal::new(show_content); let
	text = Signal::new(content.clone()); let view = page!(| show : Signal < bool >, text
	: Signal < String >| { div { watch { if show.get() { span { { text.get() } } } } } })
	(show.clone(), text.clone()); prop_assert!(matches!(view, Page::Element(_)),
	"page! macro should always produce Page::Element at top level"); let html = view
	.render_to_string(); prop_assert!(html.starts_with("<div>"),
	"Should start with opening tag"); prop_assert!(html.ends_with("</div>"),
	"Should end with closing tag"); }
}
