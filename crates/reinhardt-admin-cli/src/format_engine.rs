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
	use rstest::rstest;

	use super::*;

	// -----------------------------------------------------------------------
	// Existing tests (converted from #[test] to #[rstest])
	// -----------------------------------------------------------------------

	#[rstest]
	fn tree_sitter_detection_ignores_strings() {
		// Arrange
		let source = r#"fn main() {
	let literal = "page!(|| { div { \"x\" } })";
	let view = page!(|| { div { "x" } });
}"#;

		// Act
		let macros = find_dsl_macros(source).expect("parse source");

		// Assert
		assert_eq!(macros.len(), 1);
		assert_eq!(macros[0].kind, MacroKind::Page);
	}

	#[rstest]
	fn formats_page_macro_with_topiary_query() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let view = page!(|| { div { "x" } });
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert_eq!(
			result.content,
			"fn main() {\n\tlet view = page!(|| {\n\t\tdiv {\n\t\t\t\"x\"\n\t\t}\n\t});\n}"
		);
		assert_eq!(result.skipped, None);
	}

	#[rstest]
	fn formats_page_dsl_directly_with_topiary_query() {
		// Act
		let formatted =
			format_dsl(MacroKind::Page, r#"|| { div { "x" } }"#).expect("format page DSL");

		// Assert
		assert_eq!(formatted, "|| {\n\tdiv {\n\t\t\"x\"\n\t}\n}");
	}

	#[rstest]
	fn preserves_separator_between_text_literal_and_following_fragment() {
		for kind in [MacroKind::Page, MacroKind::Form, MacroKind::Head] {
			// Act
			let formatted = format_dsl(kind, r#"|| { label { "hello"span { "world" } } }"#)
				.expect("format DSL");

			// Assert
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

	#[rstest]
	fn preserves_ignored_macro() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	// reinhardt-fmt: ignore
	let view = page!(|| { div { "x" } });
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert_eq!(result.skipped, Some(SkipReason::AllMacrosIgnored));
		assert_eq!(result.content, source);
	}

	#[rstest]
	fn formats_form_brace_macro() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let form = form! { name: "User", fields { email { label: "Email", } } };
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert!(result.content.contains("form! {"));
		assert!(result.content.contains("name:"));
		assert!(result.content.contains("\n\t\tfields {"));
	}

	#[rstest]
	fn formats_head_macro() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let head = head!(|| { title { "Polls" } });
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert!(result.content.contains("head!(|| {"));
		assert!(result.content.contains("\n\t\ttitle {"));
	}

	#[rstest]
	fn preserves_rust_spacing_and_comments_inside_fragments() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let view = page!(|| { if let Some(e) = err.clone() { div { // keep words (Edit / Delete)
		"x"
	} } for item in items { div {} } });
	let form = form! { watch: { result: |form| { match user { Some(Some(ref u)) => u.id, _ => 0, } }, } };
}"#;

		// Act
		let result = formatter.format(source).expect("format source");
		let second = formatter
			.format(&result.content)
			.expect("format source again");

		// Assert
		assert_eq!(second.content, result.content);
		assert!(result.content.contains("if let Some(e) = err.clone() {"));
		assert!(result.content.contains("Some(Some(ref u)) => u.id"));
		assert!(result.content.contains("// keep words (Edit / Delete)"));
		assert!(!result.content.contains("}for"));
	}

	// -----------------------------------------------------------------------
	// marker_matches tests
	// -----------------------------------------------------------------------

	#[rstest]
	fn marker_matches_in_line_comment() {
		// Arrange
		let line = "// reinhardt-fmt: off";

		// Act
		let result = marker_matches(line, "reinhardt-fmt:off");

		// Assert
		assert!(result);
	}

	#[rstest]
	fn marker_matches_ignores_string_literal() {
		// Arrange
		let line = r#"let s = "reinhardt-fmt: off";"#;

		// Act
		let result = marker_matches(line, "reinhardt-fmt:off");

		// Assert
		assert!(!result);
	}

	#[rstest]
	fn marker_matches_ignores_code_line() {
		// Arrange
		let line = "let off = reinhardt_fmt_off;";

		// Act
		let result = marker_matches(line, "reinhardt-fmt:off");

		// Assert
		assert!(!result);
	}

	#[rstest]
	fn marker_matches_with_leading_whitespace_comment() {
		// Arrange: indented comment line
		let line = "\t\t// reinhardt-fmt: ignore";

		// Act
		let result = marker_matches(line, "reinhardt-fmt:ignore");

		// Assert
		assert!(result);
	}

	#[rstest]
	fn marker_matches_with_extra_spaces_in_marker() {
		// Arrange: spaces around colon
		let line = "//  reinhardt-fmt :  off";

		// Act
		let result = marker_matches(line, "reinhardt-fmt:off");

		// Assert: whitespace is stripped before comparison
		assert!(result);
	}

	#[rstest]
	fn marker_matches_returns_false_for_empty_line() {
		// Arrange
		let line = "";

		// Act
		let result = marker_matches(line, "reinhardt-fmt:off");

		// Assert
		assert!(!result);
	}

	// -----------------------------------------------------------------------
	// has_ignore_all_marker tests
	// -----------------------------------------------------------------------

	#[rstest]
	fn has_ignore_all_marker_in_first_lines() {
		// Arrange
		let formatter = FormatEngine::new();
		let content = "// reinhardt-fmt: ignore-all\nfn main() {}";

		// Act
		let result = formatter.has_ignore_all_marker(content);

		// Assert
		assert!(result);
	}

	#[rstest]
	fn has_ignore_all_marker_absent() {
		// Arrange
		let formatter = FormatEngine::new();
		let content = "fn main() {\n\tpage!(|| { div { \"x\" } });\n}";

		// Act
		let result = formatter.has_ignore_all_marker(content);

		// Assert
		assert!(!result);
	}

	#[rstest]
	fn has_ignore_all_marker_after_code_is_ignored() {
		// Arrange: marker appears after a non-comment, non-empty line, so
		// the `take_while` predicate stops before reaching it
		let formatter = FormatEngine::new();
		let mut lines: Vec<String> = Vec::new();
		lines.push("fn main() {}".to_string());
		for _ in 0..50 {
			lines.push(String::new());
		}
		lines.push("// reinhardt-fmt: ignore-all".to_string());
		let content = lines.join("\n");

		// Act
		let result = formatter.has_ignore_all_marker(&content);

		// Assert: marker is unreachable because the predicate stops at the
		// first non-comment, non-empty line (line 0: "fn main() {}")
		assert!(!result);
	}

	#[rstest]
	fn has_ignore_all_marker_among_comments() {
		// Arrange: all lines before the marker are comments or empty
		let formatter = FormatEngine::new();
		let content = "\
// Module doc comment
// More comments

// reinhardt-fmt: ignore-all
fn main() {}";

		// Act
		let result = formatter.has_ignore_all_marker(content);

		// Assert
		assert!(result);
	}

	// -----------------------------------------------------------------------
	// Format engine: no DSL macros
	// -----------------------------------------------------------------------

	#[rstest]
	fn no_dsl_macros_returns_unchanged_content() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = "fn main() {\n\tprintln!(\"hello\");\n}";

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert_eq!(result.content, source);
		assert!(!result.contains_dsl_macro);
		assert_eq!(result.skipped, None);
	}

	// -----------------------------------------------------------------------
	// Format engine: multiple macros
	// -----------------------------------------------------------------------

	#[rstest]
	fn formats_multiple_macros_in_one_file() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let view = page!(|| { div { "hello" } });
	let h = head!(|| { title { "Title" } });
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert: both macros should be formatted (multi-line expansion)
		assert!(
			result.content.contains("page!(|| {"),
			"page macro should be formatted: {}",
			result.content
		);
		assert!(
			result.content.contains("head!(|| {"),
			"head macro should be formatted: {}",
			result.content
		);
		assert!(result.contains_dsl_macro);
		assert_eq!(result.skipped, None);
	}

	// -----------------------------------------------------------------------
	// Format engine: file-wide ignore marker
	// -----------------------------------------------------------------------

	#[rstest]
	fn file_wide_ignore_marker_skips_formatting() {
		// Arrange
		let formatter = FormatEngine::new();
		let source =
			"// reinhardt-fmt: ignore-all\nfn main() {\n\tlet v = page!(|| { div { \"x\" } });\n}";

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert_eq!(result.skipped, Some(SkipReason::FileWideMarker));
		assert!(result.contains_dsl_macro);
		assert_eq!(result.content, source);
	}

	// -----------------------------------------------------------------------
	// Format engine: individual ignore markers
	// -----------------------------------------------------------------------

	#[rstest]
	fn individual_ignore_preserves_targeted_macro_only() {
		// Arrange: two macros, only the first has an ignore marker
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	// reinhardt-fmt: ignore
	let view = page!(|| { div { "x" } });
	let h = head!(|| { title { "Title" } });
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert: first macro preserved verbatim, second formatted
		assert!(
			result.content.contains(r#"page!(|| { div { "x" } })"#),
			"ignored macro should be preserved verbatim: {}",
			result.content
		);
		assert!(
			result.content.contains("\n\t\ttitle {"),
			"non-ignored macro should be formatted: {}",
			result.content
		);
		assert_eq!(result.skipped, None);
	}

	// -----------------------------------------------------------------------
	// Format engine: off/on range markers
	// -----------------------------------------------------------------------

	#[rstest]
	fn off_on_range_preserves_macros_in_disabled_region() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	// reinhardt-fmt: off
	let view = page!(|| { div { "x" } });
	// reinhardt-fmt: on
	let h = head!(|| { title { "Title" } });
}"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert: macro inside off/on region is preserved, outside is formatted
		assert!(
			result.content.contains(r#"page!(|| { div { "x" } })"#),
			"macro in disabled region should be preserved: {}",
			result.content
		);
		assert!(
			result.content.contains("\n\t\ttitle {"),
			"macro outside disabled region should be formatted: {}",
			result.content
		);
		assert_eq!(result.skipped, None);
	}

	// -----------------------------------------------------------------------
	// Format engine: idempotency
	// -----------------------------------------------------------------------

	#[rstest]
	fn formatting_is_idempotent() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() {
	let view = page!(|| { div { "x" } });
	let h = head!(|| { title { "T" } });
}"#;

		// Act
		let first = formatter.format(source).expect("first format");
		let second = formatter.format(&first.content).expect("second format");

		// Assert
		assert_eq!(first.content, second.content);
	}

	// -----------------------------------------------------------------------
	// find_dsl_macros tests
	// -----------------------------------------------------------------------

	#[rstest]
	fn find_dsl_macros_empty_for_no_macros() {
		// Arrange
		let source = "fn main() {\n\tprintln!(\"hello\");\n}";

		// Act
		let macros = find_dsl_macros(source).expect("parse source");

		// Assert
		assert_eq!(macros.len(), 0);
	}

	#[rstest]
	fn find_dsl_macros_detects_all_three_kinds() {
		// Arrange
		let source = r#"fn main() {
	let p = page!(|| { div {} });
	let f = form! { name: "F", fields { x { label: "X", } } };
	let h = head!(|| { title { "T" } });
}"#;

		// Act
		let macros = find_dsl_macros(source).expect("parse source");

		// Assert
		assert_eq!(macros.len(), 3);
		let kinds: Vec<MacroKind> = macros.iter().map(|m| m.kind).collect();
		assert!(kinds.contains(&MacroKind::Page));
		assert!(kinds.contains(&MacroKind::Form));
		assert!(kinds.contains(&MacroKind::Head));
	}

	#[rstest]
	fn find_dsl_macros_ignores_non_reinhardt_macros() {
		// Arrange
		let source = r#"fn main() {
	let v = vec![1, 2, 3];
	println!("hello");
	assert_eq!(1, 1);
}"#;

		// Act
		let macros = find_dsl_macros(source).expect("parse source");

		// Assert
		assert_eq!(macros.len(), 0);
	}

	#[rstest]
	fn find_dsl_macros_ignores_macro_inside_string() {
		// Arrange
		let source = r#"fn main() {
	let s = "page!(|| { div {} })";
}"#;

		// Act
		let macros = find_dsl_macros(source).expect("parse source");

		// Assert
		assert_eq!(macros.len(), 0);
	}

	// -----------------------------------------------------------------------
	// format_dsl edge cases
	// -----------------------------------------------------------------------

	#[rstest]
	fn format_dsl_empty_closure_body() {
		// Arrange
		let input = "|| {}";

		// Act
		let formatted = format_dsl(MacroKind::Page, input).expect("format empty closure body");

		// Assert: should produce valid output without error
		assert!(
			formatted.contains("||"),
			"formatted output should contain closure syntax: {formatted}"
		);
	}

	#[rstest]
	fn format_dsl_nested_fragments() {
		// Arrange
		let input = r#"|| { div { span { a { "link" } } } }"#;

		// Act
		let formatted = format_dsl(MacroKind::Page, input).expect("format nested fragments");

		// Assert: each nesting level should produce deeper indentation
		assert!(
			formatted.contains("\tdiv {"),
			"outer fragment should be indented: {formatted}"
		);
		assert!(
			formatted.contains("\"link\""),
			"innermost text literal should be preserved: {formatted}"
		);
	}

	// -----------------------------------------------------------------------
	// apply_ignore_markers tests
	// -----------------------------------------------------------------------

	#[rstest]
	fn apply_ignore_markers_skips_macro_with_preceding_ignore_comment() {
		// Arrange
		let source = r#"fn main() {
	// reinhardt-fmt: ignore
	let view = page!(|| { div { "x" } });
}"#;
		let mut macros = find_dsl_macros(source).expect("parse source");

		// Act
		apply_ignore_markers(source, &mut macros);

		// Assert
		assert_eq!(macros.len(), 1);
		assert!(macros[0].should_skip);
	}

	#[rstest]
	fn apply_ignore_markers_does_not_skip_without_comment() {
		// Arrange
		let source = r#"fn main() {
	let view = page!(|| { div { "x" } });
}"#;
		let mut macros = find_dsl_macros(source).expect("parse source");

		// Act
		apply_ignore_markers(source, &mut macros);

		// Assert
		assert_eq!(macros.len(), 1);
		assert!(!macros[0].should_skip);
	}

	// -----------------------------------------------------------------------
	// SkipReason display
	// -----------------------------------------------------------------------

	#[rstest]
	fn skip_reason_display_file_wide_marker() {
		// Arrange
		let reason = SkipReason::FileWideMarker;

		// Act
		let display = format!("{reason}");

		// Assert
		assert_eq!(display, "file-wide ignore marker");
	}

	#[rstest]
	fn skip_reason_display_all_macros_ignored() {
		// Arrange
		let reason = SkipReason::AllMacrosIgnored;

		// Act
		let display = format!("{reason}");

		// Assert
		assert_eq!(display, "all macros ignored");
	}

	// -----------------------------------------------------------------------
	// macro_kind helper
	// -----------------------------------------------------------------------

	#[rstest]
	fn macro_kind_recognizes_page() {
		// Act
		let result = macro_kind("page! { }");

		// Assert
		assert_eq!(result, Some(MacroKind::Page));
	}

	#[rstest]
	fn macro_kind_recognizes_form() {
		// Act
		let result = macro_kind("form! { }");

		// Assert
		assert_eq!(result, Some(MacroKind::Form));
	}

	#[rstest]
	fn macro_kind_recognizes_head() {
		// Act
		let result = macro_kind("head! { }");

		// Assert
		assert_eq!(result, Some(MacroKind::Head));
	}

	#[rstest]
	fn macro_kind_returns_none_for_unknown() {
		// Act
		let result = macro_kind("vec! [1, 2, 3]");

		// Assert
		assert_eq!(result, None);
	}

	#[rstest]
	fn macro_kind_returns_none_for_prefix_without_bang() {
		// Act: "page" without "!" should not match
		let result = macro_kind("page { }");

		// Assert
		assert_eq!(result, None);
	}

	// -----------------------------------------------------------------------
	// base_indent helper
	// -----------------------------------------------------------------------

	#[rstest]
	fn base_indent_zero_for_first_line() {
		// Arrange
		let content = "page!(|| { div {} })";

		// Act
		let indent = base_indent(content, 0);

		// Assert
		assert_eq!(indent, 0);
	}

	#[rstest]
	fn base_indent_counts_tabs() {
		// Arrange
		let content = "fn main() {\n\t\tlet view = page!(|| {});\n}";
		let offset = content.find("page").expect("find page");

		// Act
		let indent = base_indent(content, offset);

		// Assert: two tabs before "let"
		assert_eq!(indent, 2);
	}

	// -----------------------------------------------------------------------
	// indent_relative helper
	// -----------------------------------------------------------------------

	#[rstest]
	fn indent_relative_no_indent() {
		// Arrange
		let input = "line1\nline2";

		// Act
		let result = indent_relative(input, 0);

		// Assert
		assert_eq!(result, "line1\nline2");
	}

	#[rstest]
	fn indent_relative_adds_tabs_to_subsequent_lines() {
		// Arrange
		let input = "line1\nline2\nline3";

		// Act
		let result = indent_relative(input, 1);

		// Assert: first line unchanged, subsequent lines get one tab prefix
		assert_eq!(result, "line1\n\tline2\n\tline3");
	}

	#[rstest]
	fn indent_relative_preserves_empty_lines() {
		// Arrange
		let input = "line1\n\nline3";

		// Act
		let result = indent_relative(input, 1);

		// Assert: empty line should remain empty (no indent added)
		assert_eq!(result, "line1\n\n\tline3");
	}

	// -----------------------------------------------------------------------
	// FormatResult field validation
	// -----------------------------------------------------------------------

	#[rstest]
	fn format_result_contains_dsl_macro_true_when_macro_present() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = r#"fn main() { let v = page!(|| { div { "x" } }); }"#;

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert!(result.contains_dsl_macro);
	}

	#[rstest]
	fn format_result_contains_dsl_macro_false_when_no_macro() {
		// Arrange
		let formatter = FormatEngine::new();
		let source = "fn main() { println!(\"hello\"); }";

		// Act
		let result = formatter.format(source).expect("format source");

		// Assert
		assert!(!result.contains_dsl_macro);
	}
}
