use crate::renderer::{RenderResult, Renderer, RendererContext};
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct XMLRenderer {
	pub root_name: String,
}

impl XMLRenderer {
	/// Creates a new XML renderer with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::XMLRenderer;
	///
	/// let renderer = XMLRenderer::new();
	/// assert_eq!(renderer.root_name, "root");
	/// ```
	pub fn new() -> Self {
		Self {
			root_name: "root".to_string(),
		}
	}
	/// Sets the root element name for XML output
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::XMLRenderer;
	///
	/// let renderer = XMLRenderer::new().root_name("data");
	/// assert_eq!(renderer.root_name, "data");
	/// ```
	pub fn root_name(mut self, name: impl Into<String>) -> Self {
		self.root_name = name.into();
		self
	}
}

#[async_trait]
impl Renderer for XMLRenderer {
	fn media_types(&self) -> Vec<String> {
		vec!["application/xml".to_string()]
	}
	fn format(&self) -> Option<&str> {
		Some("xml")
	}
	async fn render(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		// Convert JSON to XML manually since quick_xml has limitations
		let xml_content = json_to_xml(data, 1);
		let wrapped = format!(
			"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{}>\n{}</{}>",
			self.root_name, xml_content, self.root_name
		);
		Ok(Bytes::from(wrapped))
	}
}

fn json_to_xml(value: &Value, indent: usize) -> String {
	let indent_str = "  ".repeat(indent);
	match value {
		Value::Object(map) => {
			let mut result = String::new();
			for (key, val) in map {
				result.push_str(&format!("{}<{}>\n", indent_str, key));
				result.push_str(&json_to_xml(val, indent + 1));
				result.push_str(&format!("{}</{}>\n", indent_str, key));
			}
			result
		}
		Value::Array(arr) => {
			let mut result = String::new();
			for item in arr {
				result.push_str(&format!("{}<item>\n", indent_str));
				result.push_str(&json_to_xml(item, indent + 1));
				result.push_str(&format!("{}</item>\n", indent_str));
			}
			result
		}
		Value::String(s) => format!("{}{}\n", indent_str, s),
		Value::Number(n) => format!("{}{}\n", indent_str, n),
		Value::Bool(b) => format!("{}{}\n", indent_str, b),
		Value::Null => format!("{}<null/>\n", indent_str),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;
	#[tokio::test]
	async fn test_xml_renderer() {
		let renderer = XMLRenderer::new();
		let data = json!({"name": "test"});
		let result = renderer.render(&data, None).await.unwrap();
		let xml_str = String::from_utf8(result.to_vec()).unwrap();
		assert!(xml_str.contains("<root>"));
	}
}
