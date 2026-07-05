//! Accessible role support for native component queries.

use super::tree::{NodeId, TestDom};

/// ARIA role supported by the native component test harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
	/// Alert role.
	Alert,
	/// Button role.
	Button,
	/// Checkbox role.
	Checkbox,
	/// Combobox role.
	Combobox,
	/// Dialog role.
	Dialog,
	/// Form role.
	Form,
	/// Heading role.
	Heading,
	/// Link role.
	Link,
	/// List role.
	List,
	/// Listbox role.
	Listbox,
	/// List item role.
	ListItem,
	/// Main landmark role.
	Main,
	/// Navigation landmark role.
	Navigation,
	/// Option role.
	Option,
	/// Progressbar role.
	Progressbar,
	/// Radio role.
	Radio,
	/// Status role.
	Status,
	/// Textbox role.
	Textbox,
}

impl Role {
	/// Returns the canonical ARIA role name.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Alert => "alert",
			Self::Button => "button",
			Self::Checkbox => "checkbox",
			Self::Combobox => "combobox",
			Self::Dialog => "dialog",
			Self::Form => "form",
			Self::Heading => "heading",
			Self::Link => "link",
			Self::List => "list",
			Self::Listbox => "listbox",
			Self::ListItem => "listitem",
			Self::Main => "main",
			Self::Navigation => "navigation",
			Self::Option => "option",
			Self::Progressbar => "progressbar",
			Self::Radio => "radio",
			Self::Status => "status",
			Self::Textbox => "textbox",
		}
	}

	pub(crate) fn from_role_attr(value: &str) -> Option<Self> {
		match value.split_whitespace().next()? {
			"alert" => Some(Self::Alert),
			"button" => Some(Self::Button),
			"checkbox" => Some(Self::Checkbox),
			"combobox" => Some(Self::Combobox),
			"dialog" => Some(Self::Dialog),
			"form" => Some(Self::Form),
			"heading" => Some(Self::Heading),
			"link" => Some(Self::Link),
			"list" => Some(Self::List),
			"listbox" => Some(Self::Listbox),
			"listitem" => Some(Self::ListItem),
			"main" => Some(Self::Main),
			"navigation" => Some(Self::Navigation),
			"option" => Some(Self::Option),
			"progressbar" => Some(Self::Progressbar),
			"radio" => Some(Self::Radio),
			"status" => Some(Self::Status),
			"textbox" => Some(Self::Textbox),
			_ => None,
		}
	}
}

impl std::fmt::Display for Role {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

pub(crate) fn role_for(dom: &TestDom, node_id: NodeId) -> Option<Role> {
	let node = dom.element(node_id)?;
	if let Some(role_attr) = node.attr("role")
		&& let Some(first_role) = role_attr.split_whitespace().next()
	{
		if matches!(first_role, "presentation" | "none") {
			return None;
		}
		if let Some(role) = Role::from_role_attr(role_attr) {
			return Some(role);
		}
	}

	match node.tag.as_str() {
		"a" if node.attr("href").is_some() => Some(Role::Link),
		"button" => Some(Role::Button),
		"dialog" => Some(Role::Dialog),
		"form" => Some(Role::Form),
		"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some(Role::Heading),
		"input" => input_role(node.attr("type").unwrap_or("text")),
		"li" => Some(Role::ListItem),
		"main" => Some(Role::Main),
		"nav" => Some(Role::Navigation),
		"ol" | "ul" => Some(Role::List),
		"option" => Some(Role::Option),
		"progress" => Some(Role::Progressbar),
		"select" if node.has_attr("multiple") => Some(Role::Listbox),
		"select" => Some(Role::Combobox),
		"textarea" => Some(Role::Textbox),
		_ => None,
	}
}

pub(crate) fn accessible_name(dom: &TestDom, node_id: NodeId) -> Option<String> {
	let node = dom.element(node_id)?;
	if let Some(label) = node.attr("aria-label") {
		return non_empty(label);
	}

	if let Some(labelledby) = node.attr("aria-labelledby") {
		let label = labelledby
			.split_whitespace()
			.filter_map(|id| dom.find_element_by_id(id))
			.map(|id| dom.text_content(id))
			.filter(|text| !text.is_empty())
			.collect::<Vec<_>>()
			.join(" ");
		if !label.is_empty() {
			return Some(label);
		}
	}

	if let Some(id) = node.attr("id")
		&& let Some(label) = dom.label_for(id)
	{
		return non_empty(&dom.text_content(label));
	}

	if let Some(label) = dom.closest_label(node_id) {
		return non_empty(&dom.text_content(label));
	}

	if matches!(
		role_for(dom, node_id),
		Some(Role::Button | Role::Link | Role::Heading)
	) {
		return non_empty(&dom.text_content(node_id));
	}

	None
}

fn input_role(input_type: &str) -> Option<Role> {
	match input_type {
		"button" | "image" | "reset" | "submit" => Some(Role::Button),
		"checkbox" => Some(Role::Checkbox),
		"radio" => Some(Role::Radio),
		"hidden" => None,
		_ => Some(Role::Textbox),
	}
}

fn non_empty(value: &str) -> Option<String> {
	if value.is_empty() {
		None
	} else {
		Some(value.to_string())
	}
}
