//! Component System integration tests
//!
//! Success Criteria:
//! 1. Components can render to View correctly
//! 2. Component composition works properly
//! 3. Props are handled correctly
//! 4. IntoView trait works with Components
//! 5. ElementView builder pattern works as expected
//!
//! Test Categories:
//! - Happy Path: 3 tests
//! - Error Path: 2 tests
//! - Edge Cases: 3 tests
//! - State Transitions: 1 test
//! - Use Cases: 3 tests
//! - Property-based: 1 test
//! - Combination: 2 tests
//! - Sanity: 1 test
//! - Equivalence Partitioning: 3 tests
//! - Boundary Analysis: 4 tests
//! - Decision Table: 7 tests
//!
//! Total: 30 tests

use proptest::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use reinhardt_pages::component::DummyEvent;
use reinhardt_pages::component::{Component, ElementView, IntoView, View};
use rstest::*;

// ============================================================================
// Test Components
// ============================================================================

struct SimpleComponent {
	message: String,
}

impl Component for SimpleComponent {
	fn render(&self) -> View {
		ElementView::new("div")
			.attr("class", "simple")
			.child(self.message.clone())
			.into_view()
	}

	fn name() -> &'static str {
		"SimpleComponent"
	}
}

struct ComposedComponent {
	title: String,
	content: String,
}

impl Component for ComposedComponent {
	fn render(&self) -> View {
		ElementView::new("article")
			.child(ElementView::new("h1").child(self.title.clone()))
			.child(ElementView::new("p").child(self.content.clone()))
			.into_view()
	}

	fn name() -> &'static str {
		"ComposedComponent"
	}
}

struct PropsComponent<T: Clone + std::fmt::Display + 'static> {
	value: T,
}

impl<T: Clone + std::fmt::Display + 'static> Component for PropsComponent<T> {
	fn render(&self) -> View {
		ElementView::new("span")
			.child(format!("Value: {}", self.value))
			.into_view()
	}

	fn name() -> &'static str {
		"PropsComponent"
	}
}

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
fn simple_message() -> String {
	"Hello, World!".to_string()
}

// ============================================================================
// Happy Path Tests (3 tests)
// ============================================================================

/// Tests basic Component::render functionality
#[rstest]
fn test_component_basic_render(simple_message: String) {
	let component = SimpleComponent {
		message: simple_message.clone(),
	};
	let view = component.render();
	let html = view.render_to_string();

	assert!(html.contains("class=\"simple\""));
	assert!(html.contains(&simple_message));
}

/// Tests component composition
#[rstest]
fn test_component_composition() {
	let component = ComposedComponent {
		title: "Test Title".to_string(),
		content: "Test content here.".to_string(),
	};
	let view = component.render();
	let html = view.render_to_string();

	assert!(html.contains("<article>"));
	assert!(html.contains("<h1>Test Title</h1>"));
	assert!(html.contains("<p>Test content here.</p>"));
}

/// Tests component with generic props
#[rstest]
fn test_component_with_props() {
	let int_comp = PropsComponent { value: 42 };
	let str_comp = PropsComponent {
		value: "test".to_string(),
	};

	assert!(int_comp.render().render_to_string().contains("Value: 42"));
	assert!(str_comp.render().render_to_string().contains("Value: test"));
}

// ============================================================================
// Error Path Tests (2 tests)
// ============================================================================

/// Tests XSS protection through HTML escaping
#[rstest]
fn test_component_xss_protection() {
	let component = SimpleComponent {
		message: "<script>alert('xss')</script>".to_string(),
	};
	let html = component.render().render_to_string();

	// Should be escaped
	assert!(!html.contains("<script>"));
	assert!(html.contains("&lt;script&gt;"));
}

/// Tests rendering with special characters
#[rstest]
fn test_component_special_characters() {
	let component = SimpleComponent {
		message: "Quote: \"test\" & Ampersand: &amp;".to_string(),
	};
	let html = component.render().render_to_string();

	// Should be properly escaped
	assert!(html.contains("&quot;"));
	assert!(html.contains("&amp;"));
}

// ============================================================================
// Edge Cases Tests (3 tests)
// ============================================================================

/// Tests empty component rendering
#[rstest]
fn test_component_empty() {
	let component = SimpleComponent {
		message: String::new(),
	};
	let html = component.render().render_to_string();

	assert_eq!(html, "<div class=\"simple\"></div>");
}

/// Tests large component tree
#[rstest]
fn test_component_large_tree() {
	let children: Vec<View> = (0..100)
		.map(|i| {
			ElementView::new("div")
				.child(format!("Item {}", i))
				.into_view()
		})
		.collect();

	let view = ElementView::new("div").children(children).into_view();
	let html = view.render_to_string();

	assert!(html.contains("Item 0"));
	assert!(html.contains("Item 99"));
}

/// Tests deeply nested component structure
#[rstest]
fn test_component_deep_nesting() {
	let mut view = ElementView::new("span").child("Core").into_view();

	for i in 0..20 {
		view = ElementView::new("div")
			.attr("level", i.to_string())
			.child(view)
			.into_view();
	}

	let html = view.render_to_string();
	assert!(html.contains("level=\"0\""));
	assert!(html.contains("level=\"19\""));
	assert!(html.contains("Core"));
}

// ============================================================================
// State Transitions Tests (1 test)
// ============================================================================

/// Tests props change triggering re-render
#[rstest]
fn test_component_state_transitions() {
	let component1 = SimpleComponent {
		message: "Initial".to_string(),
	};
	let html1 = component1.render().render_to_string();
	assert!(html1.contains("Initial"));

	// Simulate props change
	let component2 = SimpleComponent {
		message: "Updated".to_string(),
	};
	let html2 = component2.render().render_to_string();
	assert!(html2.contains("Updated"));
	assert!(!html2.contains("Initial"));
}

// ============================================================================
// Use Cases Tests (3 tests)
// ============================================================================

/// Tests list rendering use case
#[rstest]
fn test_component_use_case_list() {
	let items = vec!["Apple", "Banana", "Cherry"];
	let view = ElementView::new("ul")
		.children(items.iter().map(|item| {
			ElementView::new("li")
				.attr("class", "item")
				.child(*item)
				.into_view()
		}))
		.into_view();

	let html = view.render_to_string();
	assert!(html.contains("<ul>"));
	assert!(html.contains("<li class=\"item\">Apple</li>"));
	assert!(html.contains("<li class=\"item\">Banana</li>"));
	assert!(html.contains("<li class=\"item\">Cherry</li>"));
}

/// Tests form rendering use case
#[rstest]
fn test_component_use_case_form() {
	let view = ElementView::new("form")
		.attr("method", "post")
		.child(
			ElementView::new("input")
				.attr("type", "text")
				.attr("name", "username")
				.attr("placeholder", "Enter username"),
		)
		.child(
			ElementView::new("input")
				.attr("type", "password")
				.attr("name", "password")
				.attr("placeholder", "Enter password"),
		)
		.child(
			ElementView::new("button")
				.attr("type", "submit")
				.child("Login"),
		)
		.into_view();

	let html = view.render_to_string();
	assert!(html.contains("<form method=\"post\">"));
	assert!(html.contains("type=\"text\""));
	assert!(html.contains("type=\"password\""));
	assert!(html.contains("type=\"submit\""));
}

/// Tests nested component use case
#[rstest]
fn test_component_use_case_nested() {
	struct HeaderComponent;
	impl Component for HeaderComponent {
		fn render(&self) -> View {
			ElementView::new("header")
				.child(ElementView::new("h1").child("Site Title"))
				.into_view()
		}
		fn name() -> &'static str {
			"HeaderComponent"
		}
	}

	struct MainComponent;
	impl Component for MainComponent {
		fn render(&self) -> View {
			ElementView::new("main")
				.child(ElementView::new("p").child("Main content"))
				.into_view()
		}
		fn name() -> &'static str {
			"MainComponent"
		}
	}

	let view = ElementView::new("div")
		.attr("class", "app")
		.child(HeaderComponent.render())
		.child(MainComponent.render())
		.into_view();

	let html = view.render_to_string();
	assert!(html.contains("<header>"));
	assert!(html.contains("<main>"));
	assert!(html.contains("Site Title"));
	assert!(html.contains("Main content"));
}

// ============================================================================
// Property-based Tests (1 test)
// ============================================================================

/// Tests props immutability and consistency
#[rstest]
fn test_component_property_props_immutability() {
	proptest!(|(value in -1000i32..1000i32)| {
		let comp1 = PropsComponent { value };
		let comp2 = PropsComponent { value };

		let html1 = comp1.render().render_to_string();
		let html2 = comp2.render().render_to_string();

		// Verify content before comparing equality
		let expected = format!("Value: {}", value);
		prop_assert!(html1.contains(&expected));
		// Same props should produce same output
		prop_assert_eq!(html1, html2);
	});
}

// ============================================================================
// Combination Tests (2 tests)
// ============================================================================

/// Tests Component with IntoView trait
#[rstest]
fn test_component_combination_into_view() {
	let component = SimpleComponent {
		message: "Test".to_string(),
	};

	// Component implements IntoView through blanket implementation
	let view: View = component.into_view();
	let html = view.render_to_string();

	assert!(html.contains("Test"));
}

/// Tests Component with ElementView
#[rstest]
fn test_component_combination_element_view() {
	let inner = SimpleComponent {
		message: "Inner".to_string(),
	};

	let outer = ElementView::new("section")
		.attr("id", "wrapper")
		.child(inner.render())
		.into_view();

	let html = outer.render_to_string();
	assert!(html.contains("<section id=\"wrapper\">"));
	assert!(html.contains("Inner"));
}

// ============================================================================
// Sanity Tests (1 test)
// ============================================================================

/// Tests minimal component implementation
#[rstest]
fn test_component_sanity_minimal() {
	struct MinimalComponent;
	impl Component for MinimalComponent {
		fn render(&self) -> View {
			View::text("Minimal")
		}
		fn name() -> &'static str {
			"MinimalComponent"
		}
	}

	let comp = MinimalComponent;
	assert_eq!(comp.render().render_to_string(), "Minimal");
	assert_eq!(MinimalComponent::name(), "MinimalComponent");
}

// ============================================================================
// Equivalence Partitioning Tests (3 tests)
// ============================================================================

/// Tests with string props
#[rstest]
#[case::string_props("Hello".to_string())]
fn test_component_equivalence_string_props(#[case] value: String) {
	let comp = PropsComponent { value };
	let html = comp.render().render_to_string();
	assert!(html.contains("Hello"));
}

/// Tests with integer props
#[rstest]
#[case::int_props(42)]
fn test_component_equivalence_int_props(#[case] value: i32) {
	let comp = PropsComponent { value };
	let html = comp.render().render_to_string();
	assert!(html.contains("42"));
}

/// Tests with float props
#[rstest]
#[case::float_props(3.14)]
fn test_component_equivalence_float_props(#[case] value: f64) {
	let comp = PropsComponent { value };
	let html = comp.render().render_to_string();
	assert!(html.contains("3.14"));
}

// ============================================================================
// Boundary Analysis Tests (4 tests)
// ============================================================================

/// Tests with zero children
#[rstest]
#[case::zero_children()]
fn test_component_boundary_zero_children() {
	let view = ElementView::new("div").into_view();
	assert_eq!(view.render_to_string(), "<div></div>");
}

/// Tests with one child
#[rstest]
#[case::one_child()]
fn test_component_boundary_one_child() {
	let view = ElementView::new("div").child("Single").into_view();
	assert_eq!(view.render_to_string(), "<div>Single</div>");
}

/// Tests with multiple children (10)
#[rstest]
#[case::multiple_children()]
fn test_component_boundary_multiple_children() {
	let children: Vec<_> = (0..10).map(|i| format!("Child{}", i)).collect();
	let view = ElementView::new("div").children(children).into_view();
	let html = view.render_to_string();

	assert!(html.contains("Child0"));
	assert!(html.contains("Child9"));
}

/// Tests with many children (100)
#[rstest]
#[case::many_children()]
fn test_component_boundary_many_children() {
	let children: Vec<_> = (0..100).map(|i| format!("Item{}", i)).collect();
	let view = ElementView::new("div").children(children).into_view();
	let html = view.render_to_string();

	assert!(html.contains("Item0"));
	assert!(html.contains("Item99"));
}

// ============================================================================
// Decision Table Tests (7 tests)
// ============================================================================

/// Decision Table: No props × No children × No events
#[rstest]
#[case::no_props_no_children_no_events()]
fn test_component_decision_case1_minimal() {
	let view = ElementView::new("div").into_view();
	assert_eq!(view.render_to_string(), "<div></div>");
}

/// Decision Table: String props × No children × No events
#[rstest]
#[case::string_props_no_children()]
fn test_component_decision_case2_string_props() {
	let comp = PropsComponent {
		value: "Test".to_string(),
	};
	let html = comp.render().render_to_string();
	assert!(html.contains("Test"));
}

/// Decision Table: No props × Has children × No events
#[rstest]
#[case::no_props_with_children()]
fn test_component_decision_case3_with_children() {
	let view = ElementView::new("div")
		.child("Child1")
		.child("Child2")
		.into_view();
	let html = view.render_to_string();
	assert!(html.contains("Child1"));
	assert!(html.contains("Child2"));
}

/// Decision Table: String props × Has children × No events
#[rstest]
#[case::props_with_children()]
fn test_component_decision_case4_props_and_children() {
	let view = ElementView::new("div")
		.attr("class", "container")
		.child("Content")
		.into_view();
	let html = view.render_to_string();
	assert!(html.contains("class=\"container\""));
	assert!(html.contains("Content"));
}

/// Decision Table: No props × No children × Has event listener
#[rstest]
#[case::no_props_with_listener()]
fn test_component_decision_case5_with_listener() {
	#[cfg(not(target_arch = "wasm32"))]
	let view = ElementView::new("button")
		.listener("click", |_event: DummyEvent| {
			// Handler logic
		})
		.into_view();

	#[cfg(target_arch = "wasm32")]
	let view = ElementView::new("button")
		.listener("click", |_event| {
			// Handler logic
		})
		.into_view();

	// Event handlers don't affect HTML output
	assert_eq!(view.render_to_string(), "<button></button>");
}

/// Decision Table: Fragment with multiple views
#[rstest]
#[case::fragment_multiple()]
fn test_component_decision_case6_fragment() {
	let view = View::fragment(vec!["First", "Second", "Third"]);
	assert_eq!(view.render_to_string(), "FirstSecondThird");
}

/// Decision Table: Void element with attributes
#[rstest]
#[case::void_element()]
fn test_component_decision_case7_void_element() {
	let view = ElementView::new("br").attr("class", "break").into_view();
	assert_eq!(view.render_to_string(), "<br class=\"break\" />");
}
