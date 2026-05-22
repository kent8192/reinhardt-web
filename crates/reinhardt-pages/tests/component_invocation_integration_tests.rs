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
	#[builder(default)]
	children: Option<Page>,
}

fn card(props: CardProps) -> Page {
	page!(|p: CardProps| {
		article {
			h2 { {p.item.clone()} }
			{p.children.unwrap_or_else(Page::empty)}
		}
	})(props)
}

#[rstest]
fn brace_invocation_compiles_and_renders() {
	// Arrange + Act
	let v = page!(|| {
		div {
			Card { item: "hello".to_string() }
		}
	})();

	// Assert
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
				p { "one" }
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
