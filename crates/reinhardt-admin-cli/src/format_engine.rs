//! Tree-sitter + Topiary based formatter engine for Reinhardt DSL macros.
//!
//! Rust source files are parsed with tree-sitter-rust so `page!`, `form!`, and
//! `head!` invocations are detected as syntax nodes rather than by text search.
//! Each DSL body is then parsed by a small Reinhardt tree-sitter grammar and
//! formatted through Topiary query captures. Rust code formatting remains the
//! responsibility of rustfmt.

use std::path::PathBuf;
use std::process::Command;

use topiary_core::{
	Language as TopiaryLanguage, Operation, TopiaryQuery, formatter_str as topiary_formatter_str,
};
use topiary_tree_sitter_facade::Language as TopiaryTreeSitterLanguage;
use tree_sitter::{Language, Node, Parser};

/// Reason why formatting was skipped for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkipReason {
	/// File-wide ignore-all marker detected.
	FileWideMarker,
	/// All DSL macros were individually ignored.
	AllMacrosIgnored,
}

impl std::fmt::Display for SkipReason {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::FileWideMarker => write!(f, "file-wide ignore marker"),
			Self::AllMacrosIgnored => write!(f, "all macros ignored"),
		}
	}
}

/// Options to pass to rustfmt.
#[derive(Clone, Debug, Default)]
pub(crate) struct RustfmtOptions {
	/// Path to rustfmt.toml configuration file.
	pub config_path: Option<PathBuf>,
	/// Rust edition to use.
	pub edition: Option<String>,
	/// Rustfmt style edition to use.
	pub style_edition: Option<String>,
	/// Inline rustfmt config options.
	pub config: Option<String>,
	/// Rustfmt color setting.
	pub color: Option<String>,
}

impl RustfmtOptions {
	/// Apply these options to a rustfmt command.
	pub(crate) fn apply_to_command(&self, cmd: &mut Command) {
		if let Some(ref path) = self.config_path {
			cmd.arg("--config-path").arg(path);
		}
		if let Some(ref edition) = self.edition {
			cmd.arg("--edition").arg(edition);
		}
		if let Some(ref style_edition) = self.style_edition {
			cmd.arg("--style-edition").arg(style_edition);
		}
		if let Some(ref config) = self.config {
			cmd.arg("--config").arg(config);
		}
		if let Some(ref color) = self.color {
			cmd.arg("--color").arg(color);
		}
	}
}

/// Result of formatting operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormatResult {
	/// Formatted content.
	pub content: String,
	/// Whether the file contains a DSL macro.
	pub contains_dsl_macro: bool,
	/// If set, formatting was skipped for this reason.
	pub skipped: Option<SkipReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MacroKind {
	Page,
	Form,
	Head,
}

impl MacroKind {
	fn name(self) -> &'static str {
		match self {
			Self::Page => "page",
			Self::Form => "form",
			Self::Head => "head",
		}
	}

	fn query(self) -> &'static str {
		match self {
			Self::Page => include_str!("../queries/page_formatting.scm"),
			Self::Form => include_str!("../queries/form_formatting.scm"),
			Self::Head => include_str!("../queries/head_formatting.scm"),
		}
	}

	fn grammar(self) -> Language {
		match self {
			Self::Page => tree_sitter_reinhardt_page::LANGUAGE.into(),
			Self::Form => tree_sitter_reinhardt_form::LANGUAGE.into(),
			Self::Head => tree_sitter_reinhardt_head::LANGUAGE.into(),
		}
	}
}

#[derive(Debug, Clone)]
struct MacroInfo {
	start: usize,
	end: usize,
	kind: MacroKind,
	should_skip: bool,
}

#[derive(Debug)]
struct MacroParts<'a> {
	open: char,
	close: char,
	inner: &'a str,
}

/// Formatter engine used by `fmt` and `fmt-all`.
#[derive(Debug, Default)]
pub(crate) struct FormatEngine;

impl FormatEngine {
	/// Create a new formatter engine.
	pub(crate) fn new() -> Self {
		Self
	}

	/// Check if a source file has a file-wide ignore marker.
	pub(crate) fn has_ignore_all_marker(&self, content: &str) -> bool {
		content
			.lines()
			.take(50)
			.take_while(|line| {
				let trimmed = line.trim();
				trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*")
			})
			.any(|line| marker_matches(line, "reinhardt-fmt:ignore-all"))
	}

	/// Format all supported DSL macros in a Rust source string.
	pub(crate) fn format(&self, content: &str) -> Result<FormatResult, String> {
		let mut macros = find_dsl_macros(content)?;
		if macros.is_empty() {
			return Ok(FormatResult {
				content: content.to_string(),
				contains_dsl_macro: false,
				skipped: None,
			});
		}

		if self.has_ignore_all_marker(content) {
			return Ok(FormatResult {
				content: content.to_string(),
				contains_dsl_macro: true,
				skipped: Some(SkipReason::FileWideMarker),
			});
		}

		apply_ignore_markers(content, &mut macros);
		if macros.iter().all(|info| info.should_skip) {
			return Ok(FormatResult {
				content: content.to_string(),
				contains_dsl_macro: true,
				skipped: Some(SkipReason::AllMacrosIgnored),
			});
		}

		let mut result = String::with_capacity(content.len());
		let mut last_end = 0;
		for info in &macros {
			result.push_str(&content[last_end..info.start]);
			if info.should_skip {
				result.push_str(&content[info.start..info.end]);
			} else {
				let original = &content[info.start..info.end];
				match format_macro(original, info.kind, base_indent(content, info.start)) {
					Ok(formatted) => result.push_str(&formatted),
					Err(_) => result.push_str(original),
				}
			}
			last_end = info.end;
		}
		result.push_str(&content[last_end..]);

		Ok(FormatResult {
			content: result,
			contains_dsl_macro: true,
			skipped: None,
		})
	}
}

fn find_dsl_macros(content: &str) -> Result<Vec<MacroInfo>, String> {
	let mut parser = Parser::new();
	let rust_language = tree_sitter_rust::LANGUAGE.into();
	parser
		.set_language(&rust_language)
		.map_err(|e| format!("failed to load tree-sitter-rust grammar: {e}"))?;
	let tree = parser
		.parse(content, None)
		.ok_or_else(|| "failed to parse Rust source with tree-sitter".to_string())?;

	let mut macros = Vec::new();
	collect_macro_nodes(tree.root_node(), content, &mut macros);
	macros.sort_by_key(|info| info.start);
	macros.dedup_by_key(|info| (info.start, info.end));
	Ok(macros)
}

fn collect_macro_nodes(node: Node<'_>, source: &str, macros: &mut Vec<MacroInfo>) {
	if node.kind() == "macro_invocation"
		&& let Ok(text) = node.utf8_text(source.as_bytes())
		&& let Some(kind) = macro_kind(text)
	{
		macros.push(MacroInfo {
			start: node.start_byte(),
			end: node.end_byte(),
			kind,
			should_skip: false,
		});
	}

	let mut cursor = node.walk();
	for child in node.children(&mut cursor) {
		collect_macro_nodes(child, source, macros);
	}
}

fn macro_kind(text: &str) -> Option<MacroKind> {
	let trimmed = text.trim_start();
	for (name, kind) in [
		("page", MacroKind::Page),
		("form", MacroKind::Form),
		("head", MacroKind::Head),
	] {
		let Some(rest) = trimmed.strip_prefix(name) else {
			continue;
		};
		if rest.trim_start().starts_with('!') {
			return Some(kind);
		}
	}
	None
}

fn format_macro(original: &str, kind: MacroKind, base_indent: usize) -> Result<String, String> {
	let parts = split_macro(original, kind)?;
	let dsl_input = if parts.open == '{' {
		format!("{}{}{}", parts.open, parts.inner, parts.close)
	} else {
		parts.inner.to_string()
	};
	let formatted_dsl = format_dsl(kind, &dsl_input)?;
	let formatted_dsl = indent_relative(formatted_dsl.trim_end(), base_indent);

	let spacer = if parts.open == '(' { "" } else { " " };
	if parts.open == '{' {
		Ok(format!("{}!{}{}", kind.name(), spacer, formatted_dsl))
	} else {
		Ok(format!(
			"{}!{}{}{}{}",
			kind.name(),
			spacer,
			parts.open,
			formatted_dsl,
			parts.close
		))
	}
}

fn split_macro(original: &str, kind: MacroKind) -> Result<MacroParts<'_>, String> {
	let bang = original
		.find('!')
		.ok_or_else(|| format!("{} macro has no !", kind.name()))?;
	let after_bang = &original[bang + 1..];
	let after_ws = after_bang.trim_start();
	let open = after_ws
		.chars()
		.next()
		.ok_or_else(|| format!("{} macro has no delimiter", kind.name()))?;
	let close = match open {
		'(' => ')',
		'{' => '}',
		'[' => ']',
		other => return Err(format!("unsupported macro delimiter: {other}")),
	};
	let open_offset = original.len() - after_ws.len();
	let close_offset = original
		.rfind(close)
		.ok_or_else(|| format!("{} macro has no closing delimiter", kind.name()))?;
	if close_offset <= open_offset {
		return Err(format!("{} macro delimiter range is invalid", kind.name()));
	}

	Ok(MacroParts {
		open,
		close,
		inner: &original[open_offset + open.len_utf8()..close_offset],
	})
}

fn format_dsl(kind: MacroKind, input: &str) -> Result<String, String> {
	validate_dsl_with_tree_sitter(kind, input)?;
	if kind.query().trim().is_empty() {
		return Err(format!("Topiary query for {} DSL is empty", kind.name()));
	}
	let mut output = Vec::new();
	let language = topiary_language(kind)?;
	topiary_formatter_str(
		input,
		&mut output,
		&language,
		Operation::Format {
			skip_idempotence: false,
			tolerate_parsing_errors: false,
		},
	)
	.map_err(|e| format!("Topiary failed to format {} DSL: {e}", kind.name()))?;
	String::from_utf8(output)
		.map(|formatted| formatted.trim().to_string())
		.map_err(|e| {
			format!(
				"Topiary produced invalid UTF-8 for {} DSL: {e}",
				kind.name()
			)
		})
}

fn topiary_language(kind: MacroKind) -> Result<TopiaryLanguage, String> {
	let grammar: TopiaryTreeSitterLanguage = match kind {
		MacroKind::Page => tree_sitter_reinhardt_page::LANGUAGE.into(),
		MacroKind::Form => tree_sitter_reinhardt_form::LANGUAGE.into(),
		MacroKind::Head => tree_sitter_reinhardt_head::LANGUAGE.into(),
	};
	let query = TopiaryQuery::new(&grammar, kind.query())
		.map_err(|e| format!("invalid Topiary query for {} DSL: {e}", kind.name()))?;
	Ok(TopiaryLanguage {
		name: format!("reinhardt_{}", kind.name()),
		query,
		grammar,
		indent: Some("\t".to_string()),
	})
}

fn validate_dsl_with_tree_sitter(kind: MacroKind, input: &str) -> Result<(), String> {
	let mut parser = Parser::new();
	let language = kind.grammar();
	parser
		.set_language(&language)
		.map_err(|e| format!("failed to load {} DSL grammar: {e}", kind.name()))?;
	let tree = parser
		.parse(input, None)
		.ok_or_else(|| format!("failed to parse {} DSL", kind.name()))?;
	if tree.root_node().has_error() {
		return Err(format!("{} DSL parse contains ERROR nodes", kind.name()));
	}
	Ok(())
}

fn indent_relative(input: &str, base_indent: usize) -> String {
	if base_indent == 0 {
		return input.to_string();
	}
	let prefix = "\t".repeat(base_indent);
	let mut lines = input.lines();
	let Some(first) = lines.next() else {
		return String::new();
	};
	let mut result = first.to_string();
	for line in lines {
		result.push('\n');
		if !line.is_empty() {
			result.push_str(&prefix);
		}
		result.push_str(line);
	}
	result
}

fn base_indent(content: &str, offset: usize) -> usize {
	let line_start = content[..offset].rfind('\n').map_or(0, |pos| pos + 1);
	content[line_start..offset]
		.chars()
		.filter(|&ch| ch == '\t')
		.count()
}

fn apply_ignore_markers(content: &str, macros: &mut [MacroInfo]) {
	let line_starts = line_starts(content);
	for info in macros {
		let line_index = line_index_for_offset(&line_starts, info.start);
		info.should_skip = has_line_ignore(content, &line_starts, line_index)
			|| is_in_disabled_range(content, line_index);
	}
}

fn line_starts(content: &str) -> Vec<usize> {
	let mut starts = vec![0];
	for (index, byte) in content.bytes().enumerate() {
		if byte == b'\n' {
			starts.push(index + 1);
		}
	}
	starts
}

fn line_index_for_offset(starts: &[usize], offset: usize) -> usize {
	match starts.binary_search(&offset) {
		Ok(index) => index,
		Err(index) => index.saturating_sub(1),
	}
}

fn has_line_ignore(content: &str, starts: &[usize], line_index: usize) -> bool {
	if line_index == 0 {
		return false;
	}
	let previous = line(content, starts, line_index - 1);
	marker_matches(previous, "reinhardt-fmt:ignore")
}

fn is_in_disabled_range(content: &str, line_index: usize) -> bool {
	let mut disabled = false;
	for line in content.lines().take(line_index) {
		if marker_matches(line, "reinhardt-fmt:off") {
			disabled = true;
		} else if marker_matches(line, "reinhardt-fmt:on") {
			disabled = false;
		}
	}
	disabled
}

fn line<'a>(content: &'a str, starts: &[usize], line_index: usize) -> &'a str {
	let start = starts[line_index];
	let end = starts.get(line_index + 1).copied().unwrap_or(content.len());
	content[start..end].trim_end_matches('\n')
}

fn marker_matches(line: &str, compact_marker: &str) -> bool {
	let trimmed = line.trim_start();
	if !trimmed.starts_with("//") {
		return false;
	}
	trimmed
		.chars()
		.filter(|ch| !ch.is_whitespace())
		.collect::<String>()
		.contains(compact_marker)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tree_sitter_detection_ignores_strings() {
		let source = r#"fn main() {
	let literal = "page!(|| { div { \"x\" } })";
	let view = page!(|| { div { "x" } });
}"#;

		let macros = find_dsl_macros(source).expect("parse source");

		assert_eq!(macros.len(), 1);
		assert_eq!(macros[0].kind, MacroKind::Page);
	}

	#[test]
	fn formats_page_macro_with_topiary_query() {
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let view = page!(|| { div { "x" } });
}"#;

		let result = formatter.format(source).expect("format source");

		assert_eq!(
			result.content,
			"fn main() {\n\tlet view = page!(|| {\n\t\tdiv {\n\t\t\t\"x\"\n\t\t}\n\t});\n}"
		);
		assert_eq!(result.skipped, None);
	}

	#[test]
	fn formats_page_dsl_directly_with_topiary_query() {
		let formatted =
			format_dsl(MacroKind::Page, r#"|| { div { "x" } }"#).expect("format page DSL");

		assert_eq!(formatted, "|| {\n\tdiv {\n\t\t\"x\"\n\t}\n}");
	}

	#[test]
	fn preserves_separator_between_text_literal_and_following_fragment() {
		for kind in [MacroKind::Page, MacroKind::Form, MacroKind::Head] {
			let formatted = format_dsl(kind, r#"|| { label { "hello"span { "world" } } }"#)
				.expect("format DSL");

			assert!(
				formatted.contains(r#""hello" span {"#),
				"{kind:?} formatter should keep a separator before the following fragment: {formatted}"
			);
			assert!(
				!formatted.contains(r#""hello"span"#),
				"{kind:?} formatter must not concatenate text literal and fragment: {formatted}"
			);
		}
	}

	#[test]
	fn preserves_ignored_macro() {
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	// reinhardt-fmt: ignore
	let view = page!(|| { div { "x" } });
}"#;

		let result = formatter.format(source).expect("format source");

		assert_eq!(result.skipped, Some(SkipReason::AllMacrosIgnored));
		assert_eq!(result.content, source);
	}

	#[test]
	fn formats_form_brace_macro() {
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let form = form! { name: "User", fields { email { label: "Email", } } };
}"#;

		let result = formatter.format(source).expect("format source");

		assert!(result.content.contains("form! {"));
		assert!(result.content.contains("name:"));
		assert!(result.content.contains("\n\t\tfields {"));
	}

	#[test]
	fn formats_head_macro() {
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let head = head!(|| { title { "Polls" } });
}"#;

		let result = formatter.format(source).expect("format source");

		assert!(result.content.contains("head!(|| {"));
		assert!(result.content.contains("\n\t\ttitle {"));
	}

	#[test]
	fn preserves_rust_spacing_and_comments_inside_fragments() {
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let view = page!(|| { if let Some(e) = err.clone() { div { // keep words (Edit / Delete)
		"x"
	} } for item in items { div {} } });
	let form = form! { watch: { result: |form| { match user { Some(Some(ref u)) => u.id, _ => 0, } }, } };
}"#;

		let result = formatter.format(source).expect("format source");
		let second = formatter
			.format(&result.content)
			.expect("format source again");

		assert_eq!(second.content, result.content);
		assert!(result.content.contains("if let Some(e) = err.clone() {"));
		assert!(result.content.contains("Some(Some(ref u)) => u.id"));
		assert!(result.content.contains("// keep words (Edit / Delete)"));
		assert!(!result.content.contains("}for"));
	}
}
