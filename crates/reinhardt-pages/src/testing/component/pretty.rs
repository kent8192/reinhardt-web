//! Pretty-printer for native component test DOM trees.

use super::tree::{NodeId, TestDom};

pub(crate) fn pretty_dom(dom: &TestDom) -> String {
	let mut output = String::new();
	for child in dom.children(dom.root()) {
		write_node(dom, *child, 0, &mut output);
	}
	output
}

fn write_node(dom: &TestDom, node_id: NodeId, depth: usize, output: &mut String) {
	if let Some(text) = dom.text_node(node_id) {
		output.push_str(&"  ".repeat(depth));
		output.push_str(text);
		output.push('\n');
		return;
	}

	let Some(element) = dom.element(node_id) else {
		for child in dom.children(node_id) {
			write_node(dom, *child, depth, output);
		}
		return;
	};

	output.push_str(&"  ".repeat(depth));
	output.push('<');
	output.push_str(&element.tag);
	for (name, value) in element.attrs() {
		output.push(' ');
		output.push_str(name);
		output.push_str("=\"");
		output.push_str(&escape_attr(value));
		output.push('"');
	}
	output.push_str(">\n");

	if !dom.is_void(node_id) {
		for child in dom.children(node_id) {
			write_node(dom, *child, depth + 1, output);
		}
		output.push_str(&"  ".repeat(depth));
		output.push_str("</");
		output.push_str(&element.tag);
		output.push_str(">\n");
	}
}

fn escape_attr(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('"', "&quot;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
}
