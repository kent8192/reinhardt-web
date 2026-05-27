#![cfg(not(target_arch = "wasm32"))]
//! End-to-end integration tests for the React-style brace-form component
//! invocation introduced by spec §3.5.
//!
//! These tests exercise the full pipeline: parse → validate → codegen →
//! runtime render. The brace form expands to a `bon::Builder` chain on a
//! `<Name>Props` struct, dispatched into `fn <name>(props: <Name>Props) -> Page`.
//!
//! Refs #4668 (P7) #4524.

use reinhardt_pages::component::Page;
use reinhardt_pages::page;
use rstest::rstest;

#[derive(bon::Builder)]
struct CardProps {
	item: String,
	// `Option<_>` is implicitly optional under `bon::Builder`; an explicit
	// `#[builder(default)]` would be redundant and is rejected by bon.
	children: Option<Page>,
}

fn card(props: CardProps) -> Page {
	page!(|p: CardProps| {
		article {
			h2 { {
				p.item.clone()
			} }
			{
				p.children.clone().unwrap_or_else(Page::empty)
			}
		}
	})(props)
}

#[rstest]
fn brace_invocation_compiles_and_renders() {
	// Arrange + Act
	let v = page!(|| {
		div {
			Card {
				item: "hello".to_string()
			}
		}
	})();

	// Assert — uses Debug formatting with substring assertions intentionally;
	// Debug output is not stable, so these must not be converted to exact-string
	// checks. The same pattern applies throughout this file.
	let s = format!("{v:?}");
	assert!(
		s.contains("hello"),
		"render output should contain `hello`, got: {s}"
	);
}

#[rstest]
fn brace_invocation_with_single_child() {
	// Arrange + Act
	let v = page!(|| {
		div {
			Card {
				item: "outer".to_string(),
				p { "inner" }
			}
		}
	})();

	// Assert
	let s = format!("{v:?}");
	assert!(
		s.contains("outer") && s.contains("inner"),
		"render output should contain both `outer` and `inner`, got: {s}"
	);
}

#[rstest]
fn brace_invocation_with_multiple_children() {
	// Arrange + Act
	let v = page!(|| {
		div {
			Card {
				item: "outer".to_string(),
				p { "one" },
				p { "two" }
			}
		}
	})();

	// Assert: spec §3.5.3 — ≥2 children are wrapped in Page::fragment.
	let s = format!("{v:?}");
	assert!(
		s.contains("outer") && s.contains("one") && s.contains("two"),
		"render output should contain `outer`, `one`, and `two`, got: {s}"
	);
}

#[rstest]
fn nested_component_inside_for_loop() {
	// Spec §3.5.4: page!-in-page! nesting via brace-form components inside a
	// `for` loop. Each loop iteration invokes a component whose body itself
	// is a `page!` macro.
	//
	// This test depends on PR1 (#4527 — auto-wrap behavior for `for`
	// loop bodies). If PR1 has not yet merged into develop/0.2.0, the test
	// may fail at compile or assert time; this is documented in the PR body
	// and will go green once PR #4727 lands.

	#[derive(bon::Builder)]
	struct ItemCardProps {
		title: String,
	}

	fn item_card(p: ItemCardProps) -> Page {
		page!(|p: ItemCardProps| {
			article {
				h2 { {
					p.title.clone()
				} }
			}
		})(p)
	}

	let titles = vec!["a".to_string(), "b".to_string(), "c".to_string()];
	let v = page!(|titles: Vec<String>| {
		div {
			for t in titles.iter() {
				ItemCard {
					title: t.clone()
				}
			}
		}
	})(titles);

	let s = format!("{v:?}");
	for t in ["a", "b", "c"] {
		assert!(s.contains(t), "missing {t} in render output: {s}");
	}
}
