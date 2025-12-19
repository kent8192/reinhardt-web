//! Hydration markers for SSR.
//!
//! These markers are embedded in the SSR-rendered HTML to enable
//! client-side hydration. The hydration process uses these markers
//! to reconnect reactive state with the existing DOM.

use std::sync::atomic::{AtomicU64, Ordering};

/// The attribute name for hydration IDs.
pub const HYDRATION_ATTR_ID: &str = "data-rh-id";

/// The attribute name for serialized props.
pub const HYDRATION_ATTR_PROPS: &str = "data-rh-props";

/// Global counter for generating unique hydration IDs.
static HYDRATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generates a unique hydration ID.
pub fn generate_hydration_id() -> String {
	let id = HYDRATION_COUNTER.fetch_add(1, Ordering::SeqCst);
	format!("rh-{}", id)
}

/// Resets the hydration counter (for testing).
#[cfg(test)]
pub fn reset_hydration_counter() {
	HYDRATION_COUNTER.store(0, Ordering::SeqCst);
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
}

impl HydrationMarker {
	/// Creates a new hydration marker with an auto-generated ID.
	pub fn new() -> Self {
		Self {
			id: generate_hydration_id(),
			component_name: None,
			props: None,
		}
	}

	/// Creates a marker with a specific component name.
	pub fn with_component(name: impl Into<String>) -> Self {
		Self {
			id: generate_hydration_id(),
			component_name: Some(name.into()),
			props: None,
		}
	}

	/// Sets the serialized props.
	pub fn with_props(mut self, props: impl Into<String>) -> Self {
		self.props = Some(props.into());
		self
	}

	/// Generates the HTML attributes for this marker.
	pub fn to_attrs(&self) -> Vec<(String, String)> {
		let mut attrs = vec![(HYDRATION_ATTR_ID.to_string(), self.id.clone())];

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
pub fn hydration_boundary_start(id: &str) -> String {
	format!("<!--rh-start:{}-->", id)
}

/// Generates a hydration boundary end comment.
#[cfg(test)]
pub fn hydration_boundary_end(id: &str) -> String {
	format!("<!--rh-end:{}-->", id)
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
}
