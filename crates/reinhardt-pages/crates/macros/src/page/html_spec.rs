//! HTML element specifications for validation.
//!
//! This module defines HTML element specifications based on the WHATWG HTML Standard.
//! It provides validation rules for element-specific attributes, required attributes,
//! and content models.
//!
//! # Priority
//!
//! Elements are categorized by priority:
//! - **High Priority**: Commonly used elements with strict rules (img, a, input, button, form)
//! - **Medium Priority**: Basic structural elements (div, span, p, headings)
//! - **Low Priority**: Lists, tables, and other less commonly validated elements

use syn::Result;

use reinhardt_pages_ast::TypedPageElement;

/// HTML element specification.
#[derive(Debug, Clone)]
pub(super) struct ElementSpec {
	/// Element tag name
	pub tag: &'static str,
	/// Required attributes (must be present)
	pub required_attrs: &'static [AttrSpec],
	/// Allowed attributes (None = all attributes allowed)
	pub allowed_attrs: Option<&'static [&'static str]>,
	/// Whether this is a void element (no children allowed)
	#[allow(dead_code)] // Phase 3で使用予定
	pub is_void: bool,
	/// Whether this is an interactive element (cannot nest)
	#[allow(dead_code)] // Phase 3で使用予定
	pub is_interactive: bool,
	/// Content model constraints
	pub content_model: Option<ContentModel>,
}

/// Attribute specification.
#[derive(Debug, Clone)]
pub(super) struct AttrSpec {
	/// Attribute name
	pub name: &'static str,
	/// Expected type of attribute value
	#[allow(dead_code)] // Phase 3で使用予定
	pub expected_type: AttrType,
	/// Whether this attribute is required
	pub required: bool,
}

/// Expected type for attribute values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AttrType {
	/// String value (any string)
	String,
	/// URL value (should be string literal for static validation)
	Url,
	/// Boolean value (true/false)
	#[allow(dead_code)] // Phase 3で使用予定
	Boolean,
	/// Numeric value (integer or float)
	#[allow(dead_code)] // Phase 3で使用予定
	Number,
	/// Any type (no validation)
	#[allow(dead_code)] // Phase 3で使用予定
	Any,
}

/// Content model constraints.
#[derive(Debug, Clone)]
pub(super) enum ContentModel {
	/// Only specific child tags are allowed
	OnlyTags(&'static [&'static str]),
	/// Only text content (no elements)
	TextOnly,
	/// No children allowed (void elements)
	Empty,
}

/// Global HTML attributes (allowed on all elements).
///
/// Based on: https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes
pub(super) static GLOBAL_ATTRS: &[&str] = &[
	// Core attributes
	"id",
	"class",
	"style",
	"title",
	"lang",
	"dir",
	// Interaction attributes
	"tabindex",
	"accesskey",
	"contenteditable",
	"draggable",
	"hidden",
	"spellcheck",
	"translate",
	// ARIA role attribute
	"role",
	// Data attributes (data-*)
	// ARIA attributes (aria-*)
	// Note: data-* and aria-* are validated with pattern matching
];

//
// High Priority Element Specifications
//

/// Specification for `<img>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img
pub(super) static IMG_SPEC: ElementSpec = ElementSpec {
	tag: "img",
	required_attrs: &[
		AttrSpec {
			name: "src",
			expected_type: AttrType::Url,
			required: true,
		},
		AttrSpec {
			name: "alt",
			expected_type: AttrType::String,
			required: true,
		},
	],
	allowed_attrs: Some(&[
		// Required
		"src",
		"alt",
		// Optional
		"width",
		"height",
		"loading",
		"decoding",
		"srcset",
		"sizes",
		"crossorigin",
		"usemap",
		"ismap",
		"referrerpolicy",
	]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<a>` (anchor) element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a
pub(super) static A_SPEC: ElementSpec = ElementSpec {
	tag: "a",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"href",
		"target",
		"download",
		"ping",
		"rel",
		"hreflang",
		"type",
		"referrerpolicy",
	]),
	is_void: false,
	is_interactive: true,
	content_model: None, // Flow content (but no nested interactive elements)
};

/// Specification for `<button>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button
pub(super) static BUTTON_SPEC: ElementSpec = ElementSpec {
	tag: "button",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"type",
		"name",
		"value",
		"disabled",
		"form",
		"formaction",
		"formenctype",
		"formmethod",
		"formnovalidate",
		"formtarget",
		"autofocus",
		"popovertarget",
		"popovertargetaction",
	]),
	is_void: false,
	is_interactive: true,
	content_model: None, // Phrasing content (but no nested interactive elements)
};

/// Specification for `<input>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input
pub(super) static INPUT_SPEC: ElementSpec = ElementSpec {
	tag: "input",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"type",
		"name",
		"value",
		"placeholder",
		"disabled",
		"readonly",
		"required",
		"autofocus",
		"autocomplete",
		"min",
		"max",
		"step",
		"pattern",
		"maxlength",
		"minlength",
		"size",
		"multiple",
		"accept",
		"checked",
		"form",
		"formaction",
		"formenctype",
		"formmethod",
		"formnovalidate",
		"formtarget",
		"list",
		"src",
		"alt",
		"width",
		"height",
	]),
	is_void: true,
	is_interactive: true,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<form>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form
pub(super) static FORM_SPEC: ElementSpec = ElementSpec {
	tag: "form",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"action",
		"method",
		"enctype",
		"name",
		"target",
		"novalidate",
		"autocomplete",
		"accept-charset",
		"rel",
	]),
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content (but no nested form elements)
};

/// Specification for `<label>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label
pub(super) static LABEL_SPEC: ElementSpec = ElementSpec {
	tag: "label",
	required_attrs: &[],
	allowed_attrs: Some(&["for", "form"]),
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<select>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select
pub(super) static SELECT_SPEC: ElementSpec = ElementSpec {
	tag: "select",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"name",
		"disabled",
		"required",
		"autofocus",
		"form",
		"multiple",
		"size",
	]),
	is_void: false,
	is_interactive: true,
	content_model: Some(ContentModel::OnlyTags(&["option", "optgroup"])),
};

/// Specification for `<textarea>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea
pub(super) static TEXTAREA_SPEC: ElementSpec = ElementSpec {
	tag: "textarea",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"name",
		"rows",
		"cols",
		"disabled",
		"readonly",
		"required",
		"autofocus",
		"autocomplete",
		"maxlength",
		"minlength",
		"placeholder",
		"wrap",
		"form",
		"spellcheck",
	]),
	is_void: false,
	is_interactive: true,
	content_model: Some(ContentModel::TextOnly),
};

/// Specification for `<option>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option
pub(super) static OPTION_SPEC: ElementSpec = ElementSpec {
	tag: "option",
	required_attrs: &[],
	allowed_attrs: Some(&["value", "label", "disabled", "selected"]),
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::TextOnly),
};

//
// Medium Priority Element Specifications
//

/// Specification for `<div>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div
pub(super) static DIV_SPEC: ElementSpec = ElementSpec {
	tag: "div",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

/// Specification for `<span>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span
pub(super) static SPAN_SPEC: ElementSpec = ElementSpec {
	tag: "span",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<p>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p
pub(super) static P_SPEC: ElementSpec = ElementSpec {
	tag: "p",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<h1>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h1
pub(super) static H1_SPEC: ElementSpec = ElementSpec {
	tag: "h1",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<h2>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h2
pub(super) static H2_SPEC: ElementSpec = ElementSpec {
	tag: "h2",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<h3>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h3
pub(super) static H3_SPEC: ElementSpec = ElementSpec {
	tag: "h3",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<h4>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4
pub(super) static H4_SPEC: ElementSpec = ElementSpec {
	tag: "h4",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<h5>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5
pub(super) static H5_SPEC: ElementSpec = ElementSpec {
	tag: "h5",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<h6>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6
pub(super) static H6_SPEC: ElementSpec = ElementSpec {
	tag: "h6",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<header>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header
pub(super) static HEADER_SPEC: ElementSpec = ElementSpec {
	tag: "header",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content (but no nested header/footer)
};

/// Specification for `<footer>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer
pub(super) static FOOTER_SPEC: ElementSpec = ElementSpec {
	tag: "footer",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content (but no nested header/footer)
};

/// Specification for `<main>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main
pub(super) static MAIN_SPEC: ElementSpec = ElementSpec {
	tag: "main",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

/// Specification for `<nav>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav
pub(super) static NAV_SPEC: ElementSpec = ElementSpec {
	tag: "nav",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

/// Specification for `<section>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section
pub(super) static SECTION_SPEC: ElementSpec = ElementSpec {
	tag: "section",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

/// Specification for `<article>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article
pub(super) static ARTICLE_SPEC: ElementSpec = ElementSpec {
	tag: "article",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

//
// Text-Level (Phrasing) Element Specifications
//

/// Specification for `<em>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em
pub(super) static EM_SPEC: ElementSpec = ElementSpec {
	tag: "em",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<strong>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong
pub(super) static STRONG_SPEC: ElementSpec = ElementSpec {
	tag: "strong",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<small>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small
pub(super) static SMALL_SPEC: ElementSpec = ElementSpec {
	tag: "small",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<code>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code
pub(super) static CODE_SPEC: ElementSpec = ElementSpec {
	tag: "code",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<kbd>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd
pub(super) static KBD_SPEC: ElementSpec = ElementSpec {
	tag: "kbd",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<samp>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp
pub(super) static SAMP_SPEC: ElementSpec = ElementSpec {
	tag: "samp",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<var>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var
pub(super) static VAR_SPEC: ElementSpec = ElementSpec {
	tag: "var",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<i>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i
pub(super) static I_SPEC: ElementSpec = ElementSpec {
	tag: "i",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<b>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b
pub(super) static B_SPEC: ElementSpec = ElementSpec {
	tag: "b",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<u>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u
pub(super) static U_SPEC: ElementSpec = ElementSpec {
	tag: "u",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<mark>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark
pub(super) static MARK_SPEC: ElementSpec = ElementSpec {
	tag: "mark",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<s>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s
pub(super) static S_SPEC: ElementSpec = ElementSpec {
	tag: "s",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<sub>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub
pub(super) static SUB_SPEC: ElementSpec = ElementSpec {
	tag: "sub",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<sup>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup
pub(super) static SUP_SPEC: ElementSpec = ElementSpec {
	tag: "sup",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Phrasing content
};

/// Specification for `<br>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br
pub(super) static BR_SPEC: ElementSpec = ElementSpec {
	tag: "br",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<wbr>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr
pub(super) static WBR_SPEC: ElementSpec = ElementSpec {
	tag: "wbr",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

//
// List Element Specifications
//

/// Specification for `<ul>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul
pub(super) static UL_SPEC: ElementSpec = ElementSpec {
	tag: "ul",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["li"])),
};

/// Specification for `<ol>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol
pub(super) static OL_SPEC: ElementSpec = ElementSpec {
	tag: "ol",
	required_attrs: &[],
	allowed_attrs: Some(&["reversed", "start", "type"]),
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["li"])),
};

/// Specification for `<li>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li
pub(super) static LI_SPEC: ElementSpec = ElementSpec {
	tag: "li",
	required_attrs: &[],
	allowed_attrs: Some(&["value"]),
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

/// Specification for `<dl>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl
pub(super) static DL_SPEC: ElementSpec = ElementSpec {
	tag: "dl",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["dt", "dd", "div"])),
};

/// Specification for `<dt>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt
pub(super) static DT_SPEC: ElementSpec = ElementSpec {
	tag: "dt",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content (but no header/footer/sectioning/heading)
};

/// Specification for `<dd>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd
pub(super) static DD_SPEC: ElementSpec = ElementSpec {
	tag: "dd",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

//
// Table Element Specifications
//

/// Specification for `<table>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table
pub(super) static TABLE_SPEC: ElementSpec = ElementSpec {
	tag: "table",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&[
		"caption", "colgroup", "thead", "tbody", "tfoot", "tr",
	])),
};

/// Specification for `<thead>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead
pub(super) static THEAD_SPEC: ElementSpec = ElementSpec {
	tag: "thead",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["tr"])),
};

/// Specification for `<tbody>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody
pub(super) static TBODY_SPEC: ElementSpec = ElementSpec {
	tag: "tbody",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["tr"])),
};

/// Specification for `<tfoot>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot
pub(super) static TFOOT_SPEC: ElementSpec = ElementSpec {
	tag: "tfoot",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["tr"])),
};

/// Specification for `<tr>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr
pub(super) static TR_SPEC: ElementSpec = ElementSpec {
	tag: "tr",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["th", "td"])),
};

/// Specification for `<th>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th
pub(super) static TH_SPEC: ElementSpec = ElementSpec {
	tag: "th",
	required_attrs: &[],
	allowed_attrs: Some(&["colspan", "rowspan", "headers", "scope", "abbr"]),
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content (but no header/footer/sectioning/heading)
};

/// Specification for `<td>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td
pub(super) static TD_SPEC: ElementSpec = ElementSpec {
	tag: "td",
	required_attrs: &[],
	allowed_attrs: Some(&["colspan", "rowspan", "headers"]),
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content
};

/// Specification for `<caption>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption
pub(super) static CAPTION_SPEC: ElementSpec = ElementSpec {
	tag: "caption",
	required_attrs: &[],
	allowed_attrs: None, // All attributes allowed
	is_void: false,
	is_interactive: false,
	content_model: None, // Flow content (but no table elements)
};

/// Specification for `<colgroup>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup
pub(super) static COLGROUP_SPEC: ElementSpec = ElementSpec {
	tag: "colgroup",
	required_attrs: &[],
	allowed_attrs: Some(&["span"]),
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["col"])),
};

/// Specification for `<col>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col
pub(super) static COL_SPEC: ElementSpec = ElementSpec {
	tag: "col",
	required_attrs: &[],
	allowed_attrs: Some(&["span"]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

// ============================================================================
// Embedded Content Elements
// ============================================================================

/// Specification for `<iframe>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe
pub(super) static IFRAME_SPEC: ElementSpec = ElementSpec {
	tag: "iframe",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"src",
		"srcdoc",
		"name",
		"sandbox",
		"allow",
		"allowfullscreen",
		"width",
		"height",
		"referrerpolicy",
		"loading",
	]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<video>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video
pub(super) static VIDEO_SPEC: ElementSpec = ElementSpec {
	tag: "video",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"src",
		"poster",
		"preload",
		"autoplay",
		"playsinline",
		"loop",
		"muted",
		"controls",
		"width",
		"height",
		"crossorigin",
	]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<audio>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio
pub(super) static AUDIO_SPEC: ElementSpec = ElementSpec {
	tag: "audio",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"src",
		"preload",
		"autoplay",
		"loop",
		"muted",
		"controls",
		"crossorigin",
	]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<source>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source
pub(super) static SOURCE_SPEC: ElementSpec = ElementSpec {
	tag: "source",
	required_attrs: &[],
	allowed_attrs: Some(&["src", "type", "srcset", "sizes", "media", "width", "height"]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<track>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track
pub(super) static TRACK_SPEC: ElementSpec = ElementSpec {
	tag: "track",
	required_attrs: &[AttrSpec {
		name: "src",
		expected_type: AttrType::String,
		required: true,
	}],
	allowed_attrs: Some(&["kind", "label", "srclang", "default"]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<canvas>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas
pub(super) static CANVAS_SPEC: ElementSpec = ElementSpec {
	tag: "canvas",
	required_attrs: &[],
	allowed_attrs: Some(&["width", "height"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<script>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script
pub(super) static SCRIPT_SPEC: ElementSpec = ElementSpec {
	tag: "script",
	required_attrs: &[],
	allowed_attrs: Some(&[
		"src",
		"type",
		"nomodule",
		"async",
		"defer",
		"crossorigin",
		"integrity",
		"referrerpolicy",
	]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<noscript>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript
pub(super) static NOSCRIPT_SPEC: ElementSpec = ElementSpec {
	tag: "noscript",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<embed>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed
pub(super) static EMBED_SPEC: ElementSpec = ElementSpec {
	tag: "embed",
	required_attrs: &[],
	allowed_attrs: Some(&["src", "type", "width", "height"]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<object>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object
pub(super) static OBJECT_SPEC: ElementSpec = ElementSpec {
	tag: "object",
	required_attrs: &[],
	allowed_attrs: Some(&["data", "type", "name", "usemap", "form", "width", "height"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<param>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param
pub(super) static PARAM_SPEC: ElementSpec = ElementSpec {
	tag: "param",
	required_attrs: &[AttrSpec {
		name: "name",
		expected_type: AttrType::String,
		required: true,
	}],
	allowed_attrs: Some(&["value"]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<picture>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture
pub(super) static PICTURE_SPEC: ElementSpec = ElementSpec {
	tag: "picture",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: Some(ContentModel::OnlyTags(&["source", "img"])),
};

// ============================================================================
// Other Important Elements
// ============================================================================

/// Specification for `<hr>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr
pub(super) static HR_SPEC: ElementSpec = ElementSpec {
	tag: "hr",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: true,
	is_interactive: false,
	content_model: Some(ContentModel::Empty),
};

/// Specification for `<pre>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre
pub(super) static PRE_SPEC: ElementSpec = ElementSpec {
	tag: "pre",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<blockquote>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote
pub(super) static BLOCKQUOTE_SPEC: ElementSpec = ElementSpec {
	tag: "blockquote",
	required_attrs: &[],
	allowed_attrs: Some(&["cite"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<q>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q
pub(super) static Q_SPEC: ElementSpec = ElementSpec {
	tag: "q",
	required_attrs: &[],
	allowed_attrs: Some(&["cite"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<cite>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite
pub(super) static CITE_SPEC: ElementSpec = ElementSpec {
	tag: "cite",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<abbr>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr
pub(super) static ABBR_SPEC: ElementSpec = ElementSpec {
	tag: "abbr",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<time>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time
pub(super) static TIME_SPEC: ElementSpec = ElementSpec {
	tag: "time",
	required_attrs: &[],
	allowed_attrs: Some(&["datetime"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<data>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data
pub(super) static DATA_SPEC: ElementSpec = ElementSpec {
	tag: "data",
	required_attrs: &[AttrSpec {
		name: "value",
		expected_type: AttrType::String,
		required: true,
	}],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<dfn>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn
pub(super) static DFN_SPEC: ElementSpec = ElementSpec {
	tag: "dfn",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<ins>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins
pub(super) static INS_SPEC: ElementSpec = ElementSpec {
	tag: "ins",
	required_attrs: &[],
	allowed_attrs: Some(&["cite", "datetime"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<del>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del
pub(super) static DEL_SPEC: ElementSpec = ElementSpec {
	tag: "del",
	required_attrs: &[],
	allowed_attrs: Some(&["cite", "datetime"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<ruby>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby
pub(super) static RUBY_SPEC: ElementSpec = ElementSpec {
	tag: "ruby",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<rt>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt
pub(super) static RT_SPEC: ElementSpec = ElementSpec {
	tag: "rt",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<rp>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp
pub(super) static RP_SPEC: ElementSpec = ElementSpec {
	tag: "rp",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<bdi>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi
pub(super) static BDI_SPEC: ElementSpec = ElementSpec {
	tag: "bdi",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<bdo>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo
pub(super) static BDO_SPEC: ElementSpec = ElementSpec {
	tag: "bdo",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<address>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address
pub(super) static ADDRESS_SPEC: ElementSpec = ElementSpec {
	tag: "address",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<figure>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure
pub(super) static FIGURE_SPEC: ElementSpec = ElementSpec {
	tag: "figure",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<figcaption>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption
pub(super) static FIGCAPTION_SPEC: ElementSpec = ElementSpec {
	tag: "figcaption",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<details>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details
pub(super) static DETAILS_SPEC: ElementSpec = ElementSpec {
	tag: "details",
	required_attrs: &[],
	allowed_attrs: Some(&["open"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<summary>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary
pub(super) static SUMMARY_SPEC: ElementSpec = ElementSpec {
	tag: "summary",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<dialog>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog
pub(super) static DIALOG_SPEC: ElementSpec = ElementSpec {
	tag: "dialog",
	required_attrs: &[],
	allowed_attrs: Some(&["open"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<template>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template
pub(super) static TEMPLATE_SPEC: ElementSpec = ElementSpec {
	tag: "template",
	required_attrs: &[],
	allowed_attrs: Some(&[]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Specification for `<slot>` element.
///
/// Reference: https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot
pub(super) static SLOT_SPEC: ElementSpec = ElementSpec {
	tag: "slot",
	required_attrs: &[],
	allowed_attrs: Some(&["name"]),
	is_void: false,
	is_interactive: false,
	content_model: None,
};

/// Returns the element specification for a given tag name.
///
/// # Arguments
///
/// * `tag` - The HTML tag name
///
/// # Returns
///
/// An optional reference to the element specification. Returns `None` if the
/// element is not in the high-priority list.
pub(super) fn get_element_spec(tag: &str) -> Option<&'static ElementSpec> {
	match tag {
		// High priority elements
		"img" => Some(&IMG_SPEC),
		"a" => Some(&A_SPEC),
		"button" => Some(&BUTTON_SPEC),
		"input" => Some(&INPUT_SPEC),
		"form" => Some(&FORM_SPEC),
		"label" => Some(&LABEL_SPEC),
		"select" => Some(&SELECT_SPEC),
		"textarea" => Some(&TEXTAREA_SPEC),
		"option" => Some(&OPTION_SPEC),
		// Medium priority elements
		"div" => Some(&DIV_SPEC),
		"span" => Some(&SPAN_SPEC),
		"p" => Some(&P_SPEC),
		"h1" => Some(&H1_SPEC),
		"h2" => Some(&H2_SPEC),
		"h3" => Some(&H3_SPEC),
		"h4" => Some(&H4_SPEC),
		"h5" => Some(&H5_SPEC),
		"h6" => Some(&H6_SPEC),
		"header" => Some(&HEADER_SPEC),
		"footer" => Some(&FOOTER_SPEC),
		"main" => Some(&MAIN_SPEC),
		"nav" => Some(&NAV_SPEC),
		"section" => Some(&SECTION_SPEC),
		"article" => Some(&ARTICLE_SPEC),
		// Text-level elements
		"em" => Some(&EM_SPEC),
		"strong" => Some(&STRONG_SPEC),
		"small" => Some(&SMALL_SPEC),
		"code" => Some(&CODE_SPEC),
		"kbd" => Some(&KBD_SPEC),
		"samp" => Some(&SAMP_SPEC),
		"var" => Some(&VAR_SPEC),
		"i" => Some(&I_SPEC),
		"b" => Some(&B_SPEC),
		"u" => Some(&U_SPEC),
		"mark" => Some(&MARK_SPEC),
		"s" => Some(&S_SPEC),
		"sub" => Some(&SUB_SPEC),
		"sup" => Some(&SUP_SPEC),
		"br" => Some(&BR_SPEC),
		"wbr" => Some(&WBR_SPEC),
		// List elements
		"ul" => Some(&UL_SPEC),
		"ol" => Some(&OL_SPEC),
		"li" => Some(&LI_SPEC),
		"dl" => Some(&DL_SPEC),
		"dt" => Some(&DT_SPEC),
		"dd" => Some(&DD_SPEC),
		// Table elements
		"table" => Some(&TABLE_SPEC),
		"thead" => Some(&THEAD_SPEC),
		"tbody" => Some(&TBODY_SPEC),
		"tfoot" => Some(&TFOOT_SPEC),
		"tr" => Some(&TR_SPEC),
		"th" => Some(&TH_SPEC),
		"td" => Some(&TD_SPEC),
		"caption" => Some(&CAPTION_SPEC),
		"colgroup" => Some(&COLGROUP_SPEC),
		"col" => Some(&COL_SPEC),
		// Embedded content elements
		"iframe" => Some(&IFRAME_SPEC),
		"video" => Some(&VIDEO_SPEC),
		"audio" => Some(&AUDIO_SPEC),
		"source" => Some(&SOURCE_SPEC),
		"track" => Some(&TRACK_SPEC),
		"canvas" => Some(&CANVAS_SPEC),
		"script" => Some(&SCRIPT_SPEC),
		"noscript" => Some(&NOSCRIPT_SPEC),
		"embed" => Some(&EMBED_SPEC),
		"object" => Some(&OBJECT_SPEC),
		"param" => Some(&PARAM_SPEC),
		"picture" => Some(&PICTURE_SPEC),
		// Other important elements
		"hr" => Some(&HR_SPEC),
		"pre" => Some(&PRE_SPEC),
		"blockquote" => Some(&BLOCKQUOTE_SPEC),
		"q" => Some(&Q_SPEC),
		"cite" => Some(&CITE_SPEC),
		"abbr" => Some(&ABBR_SPEC),
		"time" => Some(&TIME_SPEC),
		"data" => Some(&DATA_SPEC),
		"dfn" => Some(&DFN_SPEC),
		"ins" => Some(&INS_SPEC),
		"del" => Some(&DEL_SPEC),
		"ruby" => Some(&RUBY_SPEC),
		"rt" => Some(&RT_SPEC),
		"rp" => Some(&RP_SPEC),
		"bdi" => Some(&BDI_SPEC),
		"bdo" => Some(&BDO_SPEC),
		"address" => Some(&ADDRESS_SPEC),
		"figure" => Some(&FIGURE_SPEC),
		"figcaption" => Some(&FIGCAPTION_SPEC),
		"details" => Some(&DETAILS_SPEC),
		"summary" => Some(&SUMMARY_SPEC),
		"dialog" => Some(&DIALOG_SPEC),
		"template" => Some(&TEMPLATE_SPEC),
		"slot" => Some(&SLOT_SPEC),
		// Other elements not yet implemented
		_ => None,
	}
}

/// Validates an element against its HTML specification.
///
/// This function performs the following checks:
/// 1. Required attributes are present
/// 2. All attributes are allowed (if allowlist is defined)
/// 3. Content model constraints are satisfied
///
/// # Arguments
///
/// * `element` - The typed element to validate
///
/// # Errors
///
/// Returns a compile error if any validation rule is violated.
pub(super) fn validate_against_spec(element: &TypedPageElement) -> Result<()> {
	let tag = element.tag.to_string();

	// Get specification for this element (if it exists)
	let Some(spec) = get_element_spec(&tag) else {
		// No spec defined for this element, skip validation
		return Ok(());
	};

	// Check required attributes
	validate_required_attributes(element, spec)?;

	// Check allowed attributes (if allowlist is defined)
	if let Some(allowed) = spec.allowed_attrs {
		validate_allowed_attributes(element, allowed)?;
	}

	// Check content model (void elements, text-only, etc.)
	validate_content_model(element, spec)?;

	Ok(())
}

/// Validates that all required attributes are present.
fn validate_required_attributes(element: &TypedPageElement, spec: &ElementSpec) -> Result<()> {
	for required_attr in spec.required_attrs {
		if !required_attr.required {
			continue;
		}

		let attr_found = element.attrs.iter().any(|attr| {
			let attr_name = attr.name.to_string();
			// Remove raw identifier prefix (r#type -> type)
			let attr_name = attr_name.strip_prefix("r#").unwrap_or(&attr_name);
			// Handle underscore-to-hyphen conversion
			let html_name = attr_name.replace('_', "-");
			html_name == required_attr.name
		});

		if !attr_found {
			return Err(syn::Error::new(
				element.span,
				format!(
					"Element <{}> requires '{}' attribute",
					spec.tag, required_attr.name
				),
			));
		}
	}

	Ok(())
}

/// Validates that all attributes are in the allowed list.
fn validate_allowed_attributes(element: &TypedPageElement, allowed: &[&str]) -> Result<()> {
	for attr in &element.attrs {
		let attr_name = attr.name.to_string();
		// Remove raw identifier prefix (r#type -> type)
		let attr_name = attr_name.strip_prefix("r#").unwrap_or(&attr_name);
		let html_name = attr_name.replace('_', "-");

		// Check if it's a global attribute
		if is_global_attribute(&html_name) {
			continue;
		}

		// Check if it's in the allowed list
		if !allowed.contains(&html_name.as_str()) {
			return Err(syn::Error::new(
				attr.span,
				format!(
					"Attribute '{}' is not allowed on <{}>",
					html_name, element.tag
				),
			));
		}
	}

	Ok(())
}

/// Validates content model constraints.
fn validate_content_model(element: &TypedPageElement, spec: &ElementSpec) -> Result<()> {
	match &spec.content_model {
		Some(ContentModel::Empty) => {
			if !element.children.is_empty() {
				return Err(syn::Error::new(
					element.span,
					format!("Element <{}> cannot have children (void element)", spec.tag),
				));
			}
		}
		Some(ContentModel::TextOnly) => {
			// Text-only elements can have text and expressions, but not other elements
			for child in &element.children {
				if matches!(child, reinhardt_pages_ast::TypedPageNode::Element(_)) {
					return Err(syn::Error::new(
						element.span,
						format!(
							"Element <{}> can only contain text, not child elements",
							spec.tag
						),
					));
				}
			}
		}
		Some(ContentModel::OnlyTags(allowed_tags)) => {
			// Check that all child elements are in the allowed list
			for child in &element.children {
				if let reinhardt_pages_ast::TypedPageNode::Element(child_elem) = child {
					let child_tag = child_elem.tag.to_string();
					if !allowed_tags.contains(&child_tag.as_str()) {
						return Err(syn::Error::new(
							child_elem.span,
							format!(
								"Element <{}> can only contain: {}",
								spec.tag,
								allowed_tags.join(", ")
							),
						));
					}
				}
			}
		}
		None => {
			// No content model constraint
		}
	}

	Ok(())
}

/// Checks if an attribute is a global HTML attribute.
fn is_global_attribute(attr: &str) -> bool {
	// Check standard global attributes
	if GLOBAL_ATTRS.contains(&attr) {
		return true;
	}

	// Check data-* attributes
	if attr.starts_with("data-") {
		return true;
	}

	// Check aria-* attributes
	if attr.starts_with("aria-") {
		return true;
	}

	false
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_element_spec() {
		assert!(get_element_spec("img").is_some());
		assert!(get_element_spec("a").is_some());
		assert!(get_element_spec("button").is_some());
		assert!(get_element_spec("div").is_none()); // Not yet implemented
	}

	#[test]
	fn test_is_global_attribute() {
		assert!(is_global_attribute("id"));
		assert!(is_global_attribute("class"));
		assert!(is_global_attribute("data-testid"));
		assert!(is_global_attribute("aria-label"));
		assert!(!is_global_attribute("href"));
	}

	#[test]
	fn test_img_spec() {
		assert_eq!(IMG_SPEC.tag, "img");
		assert_eq!(IMG_SPEC.required_attrs.len(), 2);
		assert!(IMG_SPEC.is_void);
		assert!(!IMG_SPEC.is_interactive);
	}

	#[test]
	fn test_select_content_model() {
		let spec = get_element_spec("select").unwrap();
		match &spec.content_model {
			Some(ContentModel::OnlyTags(tags)) => {
				assert!(tags.contains(&"option"));
				assert!(tags.contains(&"optgroup"));
			}
			_ => panic!("Expected OnlyTags content model"),
		}
	}
}
