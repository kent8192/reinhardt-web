//! Hydration markers for SSR.
//!
//! These markers are embedded in the SSR-rendered HTML to enable
//! client-side hydration. The hydration process uses these markers
//! to reconnect reactive state with the existing DOM.
//!
//! ## Island Architecture (Phase 2-B)
//!
//! Supports selective hydration using the Island Architecture pattern:
//! - **Full**: Traditional full hydration (default)
//! - **Island**: Interactive components that require hydration
//! - **Static**: Non-interactive content (no hydration needed)

use std::sync::atomic::{AtomicU64, Ordering};

/// The attribute name for hydration IDs.
pub const HYDRATION_ATTR_ID: &str = "data-rh-id";

/// The attribute name for serialized props.
pub const HYDRATION_ATTR_PROPS: &str = "data-rh-props";

/// Global counter for generating unique hydration IDs.
static HYDRATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generates a unique hydration ID.
pub(super) fn generate_hydration_id() -> String {
	let id = HYDRATION_COUNTER.fetch_add(1, Ordering::SeqCst);
	format!("rh-{}", id)
}

/// Resets the hydration counter (for testing).
#[cfg(test)]
pub(crate) fn reset_hydration_counter() {
	HYDRATION_COUNTER.store(0, Ordering::SeqCst);
}

/// Hydration strategy for a component (Phase 2-B).
///
/// Defines how a component should be hydrated on the client side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HydrationStrategy {
	/// Full hydration (default) - entire component tree is hydrated.
	#[default]
	Full,
	/// Island hydration - only this component is hydrated (interactive island).
	Island,
	/// Static content - no hydration needed (non-interactive).
	Static,
}

/// Represents a hydration marker embedded in SSR output.
#[derive(Debug, Clone)]
pub struct HydrationMarker {
	/// The unique ID of this marker.
	pub id: String,
	/// The component name for debugging.
	pub component_name: Option<String>,
	/// Serialized props (JSON).
	pub props: Option<String>,
	/// Hydration strategy (Phase 2-B).
	pub strategy: HydrationStrategy,
}

impl HydrationMarker {
	/// Creates a new hydration marker with an auto-generated ID.
	pub fn new() -> Self {
		Self {
			id: generate_hydration_id(),
			component_name: None,
			props: None,
			strategy: HydrationStrategy::default(),
		}
	}

	/// Creates a marker with a specific component name.
	pub fn with_component(name: impl Into<String>) -> Self {
		Self {
			id: generate_hydration_id(),
			component_name: Some(name.into()),
			props: None,
			strategy: HydrationStrategy::default(),
		}
	}

	/// Sets the serialized props.
	pub fn with_props(mut self, props: impl Into<String>) -> Self {
		self.props = Some(props.into());
		self
	}

	/// Sets the hydration strategy (Phase 2-B).
	pub fn with_strategy(mut self, strategy: HydrationStrategy) -> Self {
		self.strategy = strategy;
		self
	}

	/// Creates an island marker (interactive component).
	pub fn island() -> Self {
		Self::new().with_strategy(HydrationStrategy::Island)
	}

	/// Creates a static marker (non-interactive component).
	pub fn static_content() -> Self {
		Self::new().with_strategy(HydrationStrategy::Static)
	}

	/// Generates the HTML attributes for this marker.
	pub fn to_attrs(&self) -> Vec<(String, String)> {
		let mut attrs = vec![(HYDRATION_ATTR_ID.to_string(), self.id.clone())];

		// Add strategy-specific attributes (Phase 2-B)
		match self.strategy {
			HydrationStrategy::Island => {
				attrs.push(("data-rh-island".to_string(), "true".to_string()));
			}
			HydrationStrategy::Static => {
				attrs.push(("data-rh-static".to_string(), "true".to_string()));
			}
			HydrationStrategy::Full => {
				// Default, no special marker
			}
		}

		if let Some(ref props) = self.props {
			attrs.push((HYDRATION_ATTR_PROPS.to_string(), props.clone()));
		}

		if let Some(ref name) = self.component_name {
			attrs.push(("data-rh-component".to_string(), name.clone()));
		}

		attrs
	}

	/// Generates the HTML attribute string for this marker.
	pub fn to_attr_string(&self) -> String {
		self.to_attrs()
			.iter()
			.map(|(k, v)| format!("{}=\"{}\"", k, html_escape_attr(v)))
			.collect::<Vec<_>>()
			.join(" ")
	}
}

impl Default for HydrationMarker {
	fn default() -> Self {
		Self::new()
	}
}

/// Escapes a string for use in an HTML attribute value.
fn html_escape_attr(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('"', "&quot;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
}

/// Generates a hydration boundary comment.
#[cfg(test)]
pub(crate) fn hydration_boundary_start(id: &str) -> String {
	format!("<!--rh-start:{}-->", id)
}

/// Generates a hydration boundary end comment.
#[cfg(test)]
pub(crate) fn hydration_boundary_end(id: &str) -> String {
	format!("<!--rh-end:{}-->", id)
}

/// Builder for creating HydrationMarker instances (Phase 2-B).
///
/// Provides a fluent API for constructing markers with various options.
///
/// # Example
///
/// ```no_run
/// let marker = HydrationMarkerBuilder::new()
///     .component_name("Counter")
///     .strategy(HydrationStrategy::Island)
///     .props(r#"{"count": 0}"#)
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct HydrationMarkerBuilder {
	component_name: Option<String>,
	props: Option<String>,
	strategy: Option<HydrationStrategy>,
}

impl HydrationMarkerBuilder {
	/// Creates a new builder.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the component name.
	pub fn component_name(mut self, name: impl Into<String>) -> Self {
		self.component_name = Some(name.into());
		self
	}

	/// Sets the serialized props.
	pub fn props(mut self, props: impl Into<String>) -> Self {
		self.props = Some(props.into());
		self
	}

	/// Sets the hydration strategy.
	pub fn strategy(mut self, strategy: HydrationStrategy) -> Self {
		self.strategy = Some(strategy);
		self
	}

	/// Marks as an island (interactive component).
	pub fn island(self) -> Self {
		self.strategy(HydrationStrategy::Island)
	}

	/// Marks as static (non-interactive component).
	pub fn static_content(self) -> Self {
		self.strategy(HydrationStrategy::Static)
	}

	/// Builds the HydrationMarker.
	pub fn build(self) -> HydrationMarker {
		let mut marker = HydrationMarker::new();

		if let Some(name) = self.component_name {
			marker.component_name = Some(name);
		}

		if let Some(props) = self.props {
			marker.props = Some(props);
		}

		if let Some(strategy) = self.strategy {
			marker.strategy = strategy;
		}

		marker
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_hydration_id() {
		reset_hydration_counter();
		let id1 = generate_hydration_id();
		let id2 = generate_hydration_id();
		assert_eq!(id1, "rh-0");
		assert_eq!(id2, "rh-1");
	}

	#[test]
	fn test_hydration_marker_new() {
		reset_hydration_counter();
		let marker = HydrationMarker::new();
		assert_eq!(marker.id, "rh-0");
		assert!(marker.component_name.is_none());
		assert!(marker.props.is_none());
	}

	#[test]
	fn test_hydration_marker_with_component() {
		reset_hydration_counter();
		let marker = HydrationMarker::with_component("MyComponent");
		assert_eq!(marker.component_name, Some("MyComponent".to_string()));
	}

	#[test]
	fn test_hydration_marker_with_props() {
		reset_hydration_counter();
		let marker = HydrationMarker::new().with_props(r#"{"count":42}"#);
		assert_eq!(marker.props, Some(r#"{"count":42}"#.to_string()));
	}

	#[test]
	fn test_hydration_marker_to_attrs() {
		reset_hydration_counter();
		let marker = HydrationMarker::with_component("Test").with_props(r#"{"x":1}"#);
		let attrs = marker.to_attrs();
		assert!(attrs.contains(&("data-rh-id".to_string(), "rh-0".to_string())));
		assert!(attrs.contains(&("data-rh-props".to_string(), r#"{"x":1}"#.to_string())));
		assert!(attrs.contains(&("data-rh-component".to_string(), "Test".to_string())));
	}

	#[test]
	fn test_hydration_marker_to_attr_string() {
		reset_hydration_counter();
		let marker = HydrationMarker::new();
		let attr_str = marker.to_attr_string();
		assert!(attr_str.contains("data-rh-id=\"rh-0\""));
	}

	#[test]
	fn test_html_escape_attr() {
		assert_eq!(html_escape_attr("hello"), "hello");
		assert_eq!(html_escape_attr("a&b"), "a&amp;b");
		assert_eq!(html_escape_attr("a\"b"), "a&quot;b");
		assert_eq!(html_escape_attr("<script>"), "&lt;script&gt;");
	}

	#[test]
	fn test_hydration_boundaries() {
		assert_eq!(hydration_boundary_start("rh-42"), "<!--rh-start:rh-42-->");
		assert_eq!(hydration_boundary_end("rh-42"), "<!--rh-end:rh-42-->");
	}

	// Phase 2-B Tests

	#[test]
	fn test_hydration_strategy_default() {
		assert_eq!(HydrationStrategy::default(), HydrationStrategy::Full);
	}

	#[test]
	fn test_hydration_marker_with_strategy() {
		reset_hydration_counter();
		let marker = HydrationMarker::new().with_strategy(HydrationStrategy::Island);
		assert_eq!(marker.strategy, HydrationStrategy::Island);
	}

	#[test]
	fn test_hydration_marker_island() {
		reset_hydration_counter();
		let marker = HydrationMarker::island();
		assert_eq!(marker.strategy, HydrationStrategy::Island);
		let attrs = marker.to_attrs();
		assert!(attrs.contains(&("data-rh-island".to_string(), "true".to_string())));
	}

	#[test]
	fn test_hydration_marker_static() {
		reset_hydration_counter();
		let marker = HydrationMarker::static_content();
		assert_eq!(marker.strategy, HydrationStrategy::Static);
		let attrs = marker.to_attrs();
		assert!(attrs.contains(&("data-rh-static".to_string(), "true".to_string())));
	}

	#[test]
	fn test_hydration_marker_full_no_special_attr() {
		reset_hydration_counter();
		let marker = HydrationMarker::new(); // Default is Full
		let attrs = marker.to_attrs();
		assert!(!attrs.iter().any(|(k, _)| k == "data-rh-island"));
		assert!(!attrs.iter().any(|(k, _)| k == "data-rh-static"));
	}

	#[test]
	fn test_hydration_marker_builder_basic() {
		reset_hydration_counter();
		let marker = HydrationMarkerBuilder::new().build();
		assert_eq!(marker.id, "rh-0");
		assert_eq!(marker.strategy, HydrationStrategy::Full);
	}

	#[test]
	fn test_hydration_marker_builder_with_component() {
		reset_hydration_counter();
		let marker = HydrationMarkerBuilder::new()
			.component_name("Counter")
			.build();
		assert_eq!(marker.component_name, Some("Counter".to_string()));
	}

	#[test]
	fn test_hydration_marker_builder_with_props() {
		reset_hydration_counter();
		let marker = HydrationMarkerBuilder::new()
			.props(r#"{"count": 5}"#)
			.build();
		assert_eq!(marker.props, Some(r#"{"count": 5}"#.to_string()));
	}

	#[test]
	fn test_hydration_marker_builder_island() {
		reset_hydration_counter();
		let marker = HydrationMarkerBuilder::new()
			.island()
			.component_name("Button")
			.build();
		assert_eq!(marker.strategy, HydrationStrategy::Island);
		assert_eq!(marker.component_name, Some("Button".to_string()));
	}

	#[test]
	fn test_hydration_marker_builder_static() {
		reset_hydration_counter();
		let marker = HydrationMarkerBuilder::new()
			.static_content()
			.component_name("Header")
			.build();
		assert_eq!(marker.strategy, HydrationStrategy::Static);
	}

	#[test]
	fn test_hydration_marker_builder_complete() {
		reset_hydration_counter();
		let marker = HydrationMarkerBuilder::new()
			.component_name("TodoList")
			.props(r#"{"items": []}"#)
			.strategy(HydrationStrategy::Island)
			.build();

		assert_eq!(marker.component_name, Some("TodoList".to_string()));
		assert_eq!(marker.props, Some(r#"{"items": []}"#.to_string()));
		assert_eq!(marker.strategy, HydrationStrategy::Island);

		let attrs = marker.to_attrs();
		assert!(attrs.contains(&("data-rh-id".to_string(), "rh-0".to_string())));
		assert!(attrs.contains(&("data-rh-island".to_string(), "true".to_string())));
		assert!(attrs.contains(&("data-rh-component".to_string(), "TodoList".to_string())));
		assert!(attrs.contains(&("data-rh-props".to_string(), r#"{"items": []}"#.to_string())));
	}
}
