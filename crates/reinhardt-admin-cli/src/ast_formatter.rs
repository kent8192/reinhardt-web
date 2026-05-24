//! AST-based page! and form! macro formatter implementation.
//!
//! This module provides formatting for `page!` and `form!` macro DSL using proper AST parsing.
//! Unlike the text-based approach, this implementation:
//!
//! - Uses `syn::parse_file()` to parse the entire Rust source file
//! - Uses `syn::visit` to accurately detect `page!` and `form!` macro invocations
//! - Ignores content in comments and strings (guaranteed by AST)
//! - Uses `reinhardt-pages-ast` for parsing the macro DSLs
//!
//! ## Architecture
//!
//! ```mermaid
//! flowchart TB
//!     A["Rust source file"] --> B["syn::parse_file()<br/>Parse entire file to AST"]
//!     B --> C["MacroVisitors<br/>Walk AST to find page!/form! macros"]
//!     C --> D["reinhardt_pages::ast<br/>Parse macro tokens to DSL AST"]
//!     D --> E["format_page_macro()/<br/>format_form_macro()<br/>Generate formatted code from AST"]
//!     E --> F["replace by span<br/>Replace original text"]
//!     F --> G["Formatted source file"]
//! ```

use quote::ToTokens;
use regex::Regex;
use reinhardt_pages::ast::{
	ClientTrigger, FormAction, FormCallbacks, FormDerived, FormFieldDef, FormFieldEntry,
	FormFieldGroup, FormFieldProperty, FormMacro, FormSlots, FormState, FormSubmitButtonDef,
	FormValidator, FormWatch, PageAttr, PageBody, PageComponent, PageElement, PageElse, PageEvent,
	PageExpression, PageFor, PageIf, PageMacro, PageNode, PageParam, PageText, ValidatorRule,
	ValidatorScope,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use syn::visit::Visit;
use syn::{ExprMacro, Macro, parse_file};

/// Reason why formatting was skipped for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkipReason {
	/// File-wide ignore-all marker detected
	FileWideMarker,
	/// All page! macros were individually ignored
	AllMacrosIgnored,
}

impl std::fmt::Display for SkipReason {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SkipReason::FileWideMarker => write!(f, "file-wide ignore marker"),
			SkipReason::AllMacrosIgnored => write!(f, "all macros ignored"),
		}
	}
}

/// Options to pass to rustfmt.
///
/// These options mirror rustfmt's command-line arguments and allow
/// customizing formatting behavior.
#[derive(Clone, Debug, Default)]
pub(crate) struct RustfmtOptions {
	/// Path to rustfmt.toml configuration file
	pub config_path: Option<PathBuf>,
	/// Rust edition to use (e.g., "2021", "2024")
	pub edition: Option<String>,
	/// Style edition to use
	pub style_edition: Option<String>,
	/// Inline config options (e.g., "max_width=120,hard_tabs=false")
	pub config: Option<String>,
	/// Color output setting (e.g., "auto", "always", "never")
	pub color: Option<String>,
}

impl RustfmtOptions {
	/// Apply these options to a rustfmt Command.
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
	/// Formatted content
	pub content: String,
	/// Whether the file contains page! macros
	pub contains_page_macro: bool,
	/// If set, formatting was skipped for this reason
	pub skipped: Option<SkipReason>,
}

/// The kind of macro being formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacroKind {
	/// A `page!` macro invocation.
	Page,
	/// A `form!` macro invocation.
	Form,
}

/// Information about a detected macro invocation (page! or form!).
#[derive(Debug)]
struct MacroInfo {
	/// Start byte offset in the source
	start: usize,
	/// End byte offset in the source
	end: usize,
	/// The macro's tokens (content inside page!(...) or form!(...))
	tokens: proc_macro2::TokenStream,
	/// Whether this macro should be skipped during formatting
	should_skip: bool,
	/// Original source text the tokens were parsed from (for span-based blank line detection)
	original_text: String,
	/// The kind of macro (page! or form!)
	kind: MacroKind,
}

/// Backup information for a protected page!/form! macro.
///
/// Used during the protect/restore cycle to preserve page! and form! macros
/// while rustfmt processes the surrounding Rust code.
#[derive(Debug, Clone)]
pub(crate) struct PageMacroBackup {
	/// Unique identifier for this macro (used in placeholder)
	pub id: usize,
	/// Original macro text (including "page!(...)" or "form!(...)")
	pub original: String,
	/// The kind of macro being backed up
	pub kind: MacroKind,
}

/// Result of protecting page! macros in source code.
#[derive(Debug)]
pub(crate) struct ProtectResult {
	/// Source code with page! macros replaced by placeholders
	pub protected_content: String,
	/// Backup information for each replaced macro
	pub backups: Vec<PageMacroBackup>,
}

/// Visitor that walks the AST to find page! macro invocations.
struct PageMacroVisitor<'a> {
	/// Collected macro information
	macros: Vec<MacroInfo>,
	/// Original source code for offset calculation
	source: &'a str,
}

impl<'a> PageMacroVisitor<'a> {
	fn new(source: &'a str) -> Self {
		Self {
			macros: Vec::new(),
			source,
		}
	}

	/// Extract macro info from a Macro node (page! only).
	fn extract_macro_info(&mut self, mac: &Macro) {
		if mac.path.is_ident("page") {
			// Get span information
			// Note: proc_macro2::Span in non-procedural-macro context doesn't
			// give us byte offsets directly. We need to find the macro in source.
			let tokens_str = mac.tokens.to_string();

			// Find this macro in the source by searching for "page!("
			// We use the token stream content to verify we found the right one
			if let Some(mut info) = self.find_macro_in_source(&tokens_str, MacroKind::Page) {
				info.kind = MacroKind::Page;
				self.macros.push(info);
			}
		}
	}

	/// Find the page! or form! macro in source and return its position info.
	///
	/// Accepts both the human-authored form `name!(...)` and the
	/// `proc_macro2::TokenStream` Display form `name ! ( ... )`, which
	/// appears when the formatter recurses via wrapper code built from
	/// `expr.to_token_stream()` (it inserts whitespace between tokens).
	/// Without the lenient match, nested macros inside such
	/// wrapper code would be invisible to `protect_page_macros`.
	///
	/// `tokens_content` is the `TokenStream` Display form of the macro
	/// being located (i.e. `mac.tokens.to_string()` from syn). It is
	/// used to disambiguate when a `name!(...)`-shaped substring also
	/// appears in a preceding string literal or comment: the parsed
	/// candidate's `to_string()` must match `tokens_content` to be
	/// accepted.
	///
	/// `kind` identifies whether this is a `page!` or `form!` macro,
	/// so the correct delimiters are used for scanning.
	fn find_macro_in_source(&self, tokens_content: &str, kind: MacroKind) -> Option<MacroInfo> {
		let mut search_start = 0;

		// Skip already found macros
		for found in &self.macros {
			if found.end > search_start {
				search_start = found.end;
			}
		}

		let find_fn: fn(&str) -> Option<PageBangParen> = match kind {
			MacroKind::Page => find_page_bang_paren,
			MacroKind::Form => find_form_bang_brace,
		};

		let find_matching: fn(&str, usize) -> Option<usize> = match kind {
			MacroKind::Page => find_matching_paren,
			MacroKind::Form => find_matching_brace,
		};

		while let Some(hit) = find_fn(&self.source[search_start..]) {
			let abs_start = search_start + hit.start;
			let content_start = search_start + hit.paren_open + 1;

			// Find matching closing delimiter
			if let Some(end_pos) = find_matching(self.source, content_start) {
				let macro_content = &self.source[content_start..end_pos];

				// Parse the content to get tokens, and verify it matches the
				// AST node we're locating. Without this check, a macro-shaped
				// substring inside a preceding string literal or `//` comment
				// could be mistaken for the real macro invocation.
				if let Ok(tokens) = syn::parse_str::<proc_macro2::TokenStream>(macro_content)
					&& tokens.to_string() == tokens_content
				{
					return Some(MacroInfo {
						start: abs_start,
						end: end_pos + 1, // Include closing delimiter
						tokens,
						should_skip: false,
						original_text: macro_content.to_string(),
						kind,
					});
				}
			}

			search_start = abs_start + 1;
		}

		None
	}
}

impl<'ast, 'a> Visit<'ast> for PageMacroVisitor<'a> {
	fn visit_expr_macro(&mut self, expr: &'ast ExprMacro) {
		self.extract_macro_info(&expr.mac);
		syn::visit::visit_expr_macro(self, expr);
	}

	fn visit_macro(&mut self, mac: &'ast Macro) {
		self.extract_macro_info(mac);
		syn::visit::visit_macro(self, mac);
	}
}

/// Visitor that walks the AST to find form! macro invocations.
struct FormMacroVisitor<'a> {
	/// Collected macro information
	macros: Vec<MacroInfo>,
	/// Original source code for offset calculation
	source: &'a str,
}

impl<'a> FormMacroVisitor<'a> {
	fn new(source: &'a str) -> Self {
		Self {
			macros: Vec::new(),
			source,
		}
	}

	/// Extract form! macro info from a Macro node.
	fn extract_macro_info(&mut self, mac: &Macro) {
		if mac.path.is_ident("form") {
			let tokens_str = mac.tokens.to_string();
			if let Some(mut info) = self.find_macro_in_source(&tokens_str) {
				info.kind = MacroKind::Form;
				self.macros.push(info);
			}
		}
	}

	/// Find the form! macro in source and return its position info.
	///
	/// `tokens_content` is the `TokenStream` Display form of the macro
	/// being located (i.e. `mac.tokens.to_string()` from syn).
	fn find_macro_in_source(&self, tokens_content: &str) -> Option<MacroInfo> {
		let mut search_start = 0;

		// Skip already found macros
		for found in &self.macros {
			if found.end > search_start {
				search_start = found.end;
			}
		}

		while let Some(hit) = find_form_bang_brace(&self.source[search_start..]) {
			let abs_start = search_start + hit.start;
			let content_start = search_start + hit.paren_open + 1;

			// Find matching closing brace
			if let Some(end_pos) = find_matching_brace(self.source, content_start) {
				let macro_content = &self.source[content_start..end_pos];

				if let Ok(tokens) = syn::parse_str::<proc_macro2::TokenStream>(macro_content)
					&& tokens.to_string() == tokens_content
				{
					return Some(MacroInfo {
						start: abs_start,
						end: end_pos + 1, // Include closing brace
						tokens,
						should_skip: false,
						original_text: macro_content.to_string(),
						kind: MacroKind::Form,
					});
				}
			}

			search_start = abs_start + 1;
		}

		None
	}
}

impl<'ast, 'a> Visit<'ast> for FormMacroVisitor<'a> {
	fn visit_expr_macro(&mut self, expr: &'ast ExprMacro) {
		self.extract_macro_info(&expr.mac);
		syn::visit::visit_expr_macro(self, expr);
	}

	fn visit_macro(&mut self, mac: &'ast Macro) {
		self.extract_macro_info(mac);
		syn::visit::visit_macro(self, mac);
	}
}

/// Result of locating a `page!(` invocation (with possible whitespace).
struct PageBangParen {
	/// Byte offset of the leading `p` of `page`.
	start: usize,
	/// Byte offset of the opening `(`.
	paren_open: usize,
}

/// Find the next `page <ws>* ! <ws>* (` occurrence in `s`, ensuring the
/// `page` token is at a word boundary (not part of a larger identifier
/// like `mypage`). Skips matches that appear inside line comments
/// (`// ...`), block comments (`/* ... */`, nested), regular string
/// literals (`"..."`), raw string literals (`r"..."`, `r#"..."#`, ...),
/// and char literals (`'x'`), so a `page!(...)` substring embedded in
/// such content cannot be mistaken for a real macro invocation.
/// Returns `None` if none found. Accepts the canonical `page!(` form
/// authored by users as well as the `page ! (` form emitted by
/// `proc_macro2::TokenStream`'s `Display`.
/// Find the next `<name> <ws>* ! <ws>* (` occurrence in `s`, ensuring the
/// `<name>` token is at a word boundary. Skips matches that appear inside
/// line comments, block comments, string literals, raw string literals,
/// and char literals. Returns `None` if none found. Accepts the canonical
/// `<name>!(` form authored by users as well as the `<name> ! (` form
/// emitted by `proc_macro2::TokenStream`'s `Display`.
fn find_macro_bang_paren(name: &str, s: &str) -> Option<PageBangParen> {
	let bytes = s.as_bytes();
	let name_bytes = name.as_bytes();
	let name_len = name_bytes.len();
	let mut i = 0;
	while i + name_len <= bytes.len() {
		let b = bytes[i];

		// Line comment: skip to end of line.
		if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
			i += 2;
			while i < bytes.len() && bytes[i] != b'\n' {
				i += 1;
			}
			continue;
		}

		// Block comment (Rust allows nesting): skip to matching `*/`.
		if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
			i += 2;
			let mut depth: usize = 1;
			while i + 1 < bytes.len() && depth > 0 {
				if bytes[i] == b'/' && bytes[i + 1] == b'*' {
					depth += 1;
					i += 2;
				} else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
					depth -= 1;
					i += 2;
				} else {
					i += 1;
				}
			}
			continue;
		}

		// String literal — handle both raw (`r"..."`, `r#"..."#`, `br#"..."#`...)
		// and regular forms. `detect_raw_string_start` walks backwards from
		// the `"` to find a leading `r` or `br` (plus optional `#`s) at a word
		// boundary.
		if b == b'"' {
			if let Some(hash_count) = detect_raw_string_start(s, i)
				&& let Some(end) = skip_raw_string(s, i + 1, hash_count)
			{
				i = end;
				continue;
			}
			i += 1;
			while i < bytes.len() {
				match bytes[i] {
					b'\\' if i + 1 < bytes.len() => i += 2,
					b'"' => {
						i += 1;
						break;
					}
					_ => i += 1,
				}
			}
			continue;
		}

		// Char literal vs lifetime. A char literal closes its apostrophe
		// within a few bytes (`'x'`, `'\n'`, `'\u{1F600}'`); a lifetime
		// (`'a`, `'static`) does not. Look ahead with a small budget and
		// only skip when a closing quote is found.
		if b == b'\'' {
			let mut j = i + 1;
			let limit = (i + 10).min(bytes.len());
			let mut closed = None;
			while j < limit {
				match bytes[j] {
					b'\\' if j + 1 < bytes.len() => j += 2,
					b'\'' => {
						closed = Some(j);
						break;
					}
					_ => j += 1,
				}
			}
			if let Some(close) = closed {
				i = close + 1;
				continue;
			}
			// Treat as lifetime — step past the apostrophe only.
			i += 1;
			continue;
		}

		// Not in a comment or literal — look for the name keyword.
		if &bytes[i..i + name_len] != name_bytes {
			i += 1;
			continue;
		}
		let start = i;
		// Reject if preceded by an identifier-continuation byte.
		if start > 0 {
			let prev = bytes[start - 1];
			if prev.is_ascii_alphanumeric() || prev == b'_' {
				i = start + 1;
				continue;
			}
		}
		let after = start + name_len;
		// Reject if followed by an identifier-continuation byte (excluding `!`).
		if after < bytes.len() {
			let nx = bytes[after];
			if nx.is_ascii_alphanumeric() || nx == b'_' {
				i = start + 1;
				continue;
			}
		}
		// Skip whitespace between name and `!`.
		let mut j = after;
		while j < bytes.len() && bytes[j].is_ascii_whitespace() {
			j += 1;
		}
		if j >= bytes.len() || bytes[j] != b'!' {
			i = start + 1;
			continue;
		}
		j += 1;
		// Skip whitespace between `!` and `(`.
		while j < bytes.len() && bytes[j].is_ascii_whitespace() {
			j += 1;
		}
		if j >= bytes.len() || bytes[j] != b'(' {
			i = start + 1;
			continue;
		}
		return Some(PageBangParen {
			start,
			paren_open: j,
		});
	}
	None
}

/// Find the next `page <ws>* ! <ws>* (` occurrence. Delegates to
/// `find_macro_bang_paren` with the name "page".
fn find_page_bang_paren(s: &str) -> Option<PageBangParen> {
	find_macro_bang_paren("page", s)
}

/// Find the next `form <ws>* ! <ws>* (` occurrence (with possible whitespace), skipping
/// comments, strings, and char literals.
///
/// Returns `None` if none found. Accepts the canonical `form!(` form
/// authored by users as well as the `form ! (` form emitted by
/// `proc_macro2::TokenStream`'s `Display`.
#[allow(dead_code, reason = "symmetry helper for form! compact syntax")]
fn find_form_bang_paren(s: &str) -> Option<PageBangParen> {
	find_macro_bang_paren("form", s)
}

/// Find the next `<name> <ws>* ! <ws>* {` occurrence in `s`, ensuring the
/// `<name>` token is at a word boundary. Skips matches inside line
/// comments, block comments, string literals, raw string literals,
/// and char literals.
///
/// Returns `None` if none found. Accepts the canonical `name!{` form
/// authored by users as well as the `name ! {` form emitted by
/// `proc_macro2::TokenStream`'s `Display`.
fn find_macro_bang_brace(name: &str, s: &str) -> Option<PageBangParen> {
	let bytes = s.as_bytes();
	let name_bytes = name.as_bytes();
	let name_len = name_bytes.len();
	let mut i = 0;
	while i + name_len <= bytes.len() {
		let b = bytes[i];

		// Line comment: skip to end of line.
		if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
			i += 2;
			while i < bytes.len() && bytes[i] != b'\n' {
				i += 1;
			}
			continue;
		}

		// Block comment (Rust allows nesting): skip to matching `*/`.
		if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
			i += 2;
			let mut depth: usize = 1;
			while i + 1 < bytes.len() && depth > 0 {
				if bytes[i] == b'/' && bytes[i + 1] == b'*' {
					depth += 1;
					i += 2;
				} else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
					depth -= 1;
					i += 2;
				} else {
					i += 1;
				}
			}
			continue;
		}

		// String literal — handle both raw and regular forms.
		if b == b'"' {
			if let Some(hash_count) = detect_raw_string_start(s, i)
				&& let Some(end) = skip_raw_string(s, i + 1, hash_count)
			{
				i = end;
				continue;
			}
			i += 1;
			while i < bytes.len() {
				match bytes[i] {
					b'\\' if i + 1 < bytes.len() => i += 2,
					b'"' => {
						i += 1;
						break;
					}
					_ => i += 1,
				}
			}
			continue;
		}

		// Char literal vs lifetime.
		if b == b'\'' {
			let mut j = i + 1;
			let limit = (i + 10).min(bytes.len());
			let mut closed = None;
			while j < limit {
				match bytes[j] {
					b'\\' if j + 1 < bytes.len() => j += 2,
					b'\'' => {
						closed = Some(j);
						break;
					}
					_ => j += 1,
				}
			}
			if let Some(close) = closed {
				i = close + 1;
				continue;
			}
			// Treat as lifetime — step past the apostrophe only.
			i += 1;
			continue;
		}

		// Not in a comment or literal — look for the name keyword.
		if &bytes[i..i + name_len] != name_bytes {
			i += 1;
			continue;
		}
		let start = i;
		// Reject if preceded by an identifier-continuation byte.
		if start > 0 {
			let prev = bytes[start - 1];
			if prev.is_ascii_alphanumeric() || prev == b'_' {
				i = start + 1;
				continue;
			}
		}
		let after = start + name_len;
		// Reject if followed by an identifier-continuation byte (excluding `!`).
		if after < bytes.len() {
			let nx = bytes[after];
			if nx.is_ascii_alphanumeric() || nx == b'_' {
				i = start + 1;
				continue;
			}
		}
		// Skip whitespace between name and `!`.
		let mut j = after;
		while j < bytes.len() && bytes[j].is_ascii_whitespace() {
			j += 1;
		}
		if j >= bytes.len() || bytes[j] != b'!' {
			i = start + 1;
			continue;
		}
		j += 1;
		// Skip whitespace between `!` and `{`.
		while j < bytes.len() && bytes[j].is_ascii_whitespace() {
			j += 1;
		}
		if j >= bytes.len() || bytes[j] != b'{' {
			i = start + 1;
			continue;
		}
		return Some(PageBangParen {
			start,
			paren_open: j,
		});
	}
	None
}

/// Find the next `page <ws>* ! <ws>* {` occurrence. Delegates to
/// `find_macro_bang_brace` with the name "page".
#[allow(dead_code, reason = "symmetry helper for page! brace syntax")]
fn find_page_bang_brace(s: &str) -> Option<PageBangParen> {
	find_macro_bang_brace("page", s)
}

/// Find the next `form <ws>* ! <ws>* {` occurrence. Delegates to
/// `find_macro_bang_brace` with the name "form".
fn find_form_bang_brace(s: &str) -> Option<PageBangParen> {
	find_macro_bang_brace("form", s)
}

/// Find the matching closing parenthesis, handling strings and nested parens.
///
/// Uses char_indices() to properly handle UTF-8 multi-byte characters.
fn find_matching_paren(source: &str, start: usize) -> Option<usize> {
	let substring = &source[start..];
	let mut depth = 1;
	let mut in_string = false;
	let mut in_char = false;
	let mut escape_next = false;
	let chars: Vec<(usize, char)> = substring.char_indices().collect();
	let mut i = 0;

	while i < chars.len() {
		let (offset, ch) = chars[i];

		if escape_next {
			escape_next = false;
			i += 1;
			continue;
		}

		if in_string {
			match ch {
				'\\' => escape_next = true,
				'"' => in_string = false,
				_ => {}
			}
			i += 1;
			continue;
		}

		if in_char {
			match ch {
				'\\' => escape_next = true,
				'\'' => in_char = false,
				_ => {}
			}
			i += 1;
			continue;
		}

		match ch {
			'"' => {
				// Check for raw strings: r#"..."# or r"..."
				// Look back to see if preceded by 'r' and optional '#'s
				let raw_start = detect_raw_string_start(substring, offset);
				if let Some(hash_count) = raw_start {
					// Skip raw string content until closing "###
					if let Some(end_offset) = skip_raw_string(substring, offset + 1, hash_count) {
						// Find the index in chars that corresponds to end_offset
						while i < chars.len() && chars[i].0 < end_offset {
							i += 1;
						}
						i += 1; // skip past end
						continue;
					}
				}
				in_string = true;
			}
			'\'' => {
				// Distinguish char literal from lifetime annotation:
				// Char literal: 'a', '\n', '\\'
				// Lifetime: 'a (letter not followed by closing quote in char-literal pattern)
				if is_char_literal(&chars, i) {
					in_char = true;
				}
				// Otherwise it's a lifetime, just skip the tick
			}
			'(' => depth += 1,
			')' => {
				depth -= 1;
				if depth == 0 {
					return Some(start + offset);
				}
			}
			_ => {}
		}
		i += 1;
	}

	None
}
/// Find the matching closing brace, handling strings and nested braces.
///
/// Uses char_indices() to properly handle UTF-8 multi-byte characters.
fn find_matching_brace(source: &str, start: usize) -> Option<usize> {
	let substring = &source[start..];
	let mut depth = 1;
	let mut in_string = false;
	let mut in_char = false;
	let mut escape_next = false;
	let chars: Vec<(usize, char)> = substring.char_indices().collect();
	let mut i = 0;

	while i < chars.len() {
		let (offset, ch) = chars[i];

		if escape_next {
			escape_next = false;
			i += 1;
			continue;
		}

		if in_string {
			match ch {
				'\\' => escape_next = true,
				'"' => in_string = false,
				_ => {}
			}
			i += 1;
			continue;
		}

		if in_char {
			match ch {
				'\\' => escape_next = true,
				'\'' => in_char = false,
				_ => {}
			}
			i += 1;
			continue;
		}

		match ch {
			'"' => {
				// Check for raw strings: r#"..."# or r"..."
				let raw_start = detect_raw_string_start(substring, offset);
				if let Some(hash_count) = raw_start {
					if let Some(end_offset) = skip_raw_string(substring, offset + 1, hash_count) {
						while i < chars.len() && chars[i].0 < end_offset {
							i += 1;
						}
						i += 1; // skip past end
						continue;
					}
				}
				in_string = true;
			}
			'\'' => {
				// Distinguish char literal from lifetime annotation.
				if is_char_literal(&chars, i) {
					in_char = true;
				}
			}
			'{' => depth += 1,
			'}' => {
				depth -= 1;
				if depth == 0 {
					return Some(start + offset);
				}
			}
			_ => {}
		}
		i += 1;
	}

	None
}

/// Detect if a '"' at the given offset is the start of a raw string.
/// Returns Some(hash_count) if so (0 for r"...", 1 for r#"..."#, etc.).
fn detect_raw_string_start(s: &str, quote_offset: usize) -> Option<usize> {
	// Walk backwards from the quote to find r (or br) followed by optional #s
	let before = &s[..quote_offset];
	let trimmed = before.trim_end_matches('#');
	let hash_count = before.len() - trimmed.len();

	// Check for raw string (r"..." or r#"..."#) or raw byte string (br"..." or br#"..."#)
	if trimmed.ends_with('r') {
		// Verify the 'r' is not part of an identifier
		let r_pos = trimmed.len() - 1;
		if r_pos == 0 || !before.as_bytes()[r_pos - 1].is_ascii_alphanumeric() {
			return Some(hash_count);
		}
	} else if trimmed.len() >= 2 && trimmed.ends_with("br") {
		// Check for raw byte string: br"..." or br#"..."#
		let br_pos = trimmed.len() - 2;
		if br_pos == 0 || !before.as_bytes()[br_pos - 1].is_ascii_alphanumeric() {
			return Some(hash_count);
		}
	}
	None
}

/// Skip past the contents of a raw string starting after the opening '"'.
/// Returns the byte offset just past the closing '"' + hashes.
fn skip_raw_string(s: &str, start_after_quote: usize, hash_count: usize) -> Option<usize> {
	let closing_pattern: String = std::iter::once('"')
		.chain(std::iter::repeat_n('#', hash_count))
		.collect();
	s[start_after_quote..]
		.find(&closing_pattern)
		.map(|pos| start_after_quote + pos + closing_pattern.len())
}

/// Check if a `'\''` at `chars\[idx\]` starts a char literal (not a lifetime).
/// A char literal has the pattern: 'x' or '\x' or '\xx'
fn is_char_literal(chars: &[(usize, char)], idx: usize) -> bool {
	// After the opening quote, check if we see a closing quote pattern
	let remaining = &chars[idx + 1..];

	if remaining.is_empty() {
		return false;
	}

	// Pattern: '\...' (escaped char literal)
	if remaining[0].1 == '\\' {
		// Look for closing quote within the next few chars
		for item in remaining.iter().take(remaining.len().min(5)).skip(2) {
			if item.1 == '\'' {
				return true;
			}
		}
		return false;
	}

	// Pattern: 'x' (single char literal) - must have closing quote at position +2
	if remaining.len() >= 2 && remaining[1].1 == '\'' {
		return true;
	}

	// Otherwise, it's a lifetime ('a in type position, no closing quote)
	false
}

/// Maximum recursion depth for formatting nested nodes.
///
/// Prevents stack overflow from deeply nested or maliciously crafted
/// page! macro content. 128 levels is far more than any realistic
/// template would need.
const MAX_FORMAT_DEPTH: usize = 128;

/// Line length threshold for triggering rustfmt on expression blocks.
/// Expressions (including indentation and braces) shorter than this are kept on a single line.
const EXPRESSION_LINE_LENGTH_THRESHOLD: usize = 100;

/// AST-based page! macro formatter.
pub(crate) struct AstPageFormatter {
	/// Indentation string (tab by default)
	indent: String,
	/// Options to pass to rustfmt
	rustfmt_options: RustfmtOptions,
}

impl Default for AstPageFormatter {
	fn default() -> Self {
		Self::new()
	}
}

impl AstPageFormatter {
	/// Create a new formatter with default settings.
	pub(crate) fn new() -> Self {
		Self {
			indent: "\t".to_string(),
			rustfmt_options: RustfmtOptions::default(),
		}
	}

	/// Create a new formatter with the specified rustfmt options.
	// Allow dead_code: reserved for future use when full rustfmt options support is needed
	#[allow(dead_code)]
	pub(crate) fn with_options(rustfmt_options: RustfmtOptions) -> Self {
		Self {
			indent: "\t".to_string(),
			rustfmt_options,
		}
	}

	/// Create a new formatter with a specific config path.
	pub(crate) fn with_config(config_path: PathBuf) -> Self {
		Self {
			indent: "\t".to_string(),
			rustfmt_options: RustfmtOptions {
				config_path: Some(config_path),
				..Default::default()
			},
		}
	}

	/// Calculate the base indentation level for a macro at the given position.
	///
	/// Returns the number of tabs from the start of the line to the macro position.
	fn calculate_base_indent(content: &str, macro_start: usize) -> usize {
		// Find the start of the line containing the macro
		let line_start = content[..macro_start]
			.rfind('\n')
			.map(|pos| pos + 1)
			.unwrap_or(0);

		// Count tabs in the indentation
		let indent_str = &content[line_start..macro_start];
		indent_str.chars().filter(|&c| c == '\t').count()
	}

	/// Format the content of a Rust source file.
	///
	/// Uses AST parsing for accurate macro detection. Falls back to returning
	/// the original content if parsing fails.
	pub(crate) fn format(&self, content: &str) -> Result<FormatResult, String> {
		// Safety check FIRST: If no page! or form! pattern exists, return unchanged.
		// This is a successful no-op, not an intentional skip — skipped stays None.
		// Match compact `page!(`/`form!(`/`form!{` and the TokenStream Display
		// forms so recursive formatting (which wraps via `to_token_stream()`)
		// still sees nested macros at every depth.
		if find_page_bang_paren(content).is_none()
			&& find_form_bang_paren(content).is_none()
			&& find_form_bang_brace(content).is_none()
		{
			return Ok(FormatResult {
				content: content.to_string(),
				contains_page_macro: false,
				skipped: None,
			});
		}

		// Then check for file-wide ignore marker
		if self.has_ignore_all_marker(content) {
			return Ok(FormatResult {
				content: content.to_string(),
				contains_page_macro: true, // Contains page! but ignored
				skipped: Some(SkipReason::FileWideMarker),
			});
		}

		// Collect all page! macro locations
		let macros = self.find_page_macros(content)?;

		if macros.is_empty() {
			// Substring matched but AST found no real invocation (e.g., inside
			// a comment or string literal). Successful no-op, not a skip.
			return Ok(FormatResult {
				content: content.to_string(),
				contains_page_macro: false,
				skipped: None,
			});
		}

		// Sort macros by position (they should already be in order)
		let mut macros = macros;
		macros.sort_by_key(|m| m.start);

		// Apply ignore markers to determine which macros to skip
		self.apply_ignore_markers(content, &mut macros);

		// Check if all macros are individually ignored
		if macros.iter().all(|m| m.should_skip) {
			return Ok(FormatResult {
				content: content.to_string(),
				contains_page_macro: true,
				skipped: Some(SkipReason::AllMacrosIgnored),
			});
		}

		// Build result by replacing each macro
		let mut result = String::with_capacity(content.len() * 2);
		let mut last_end = 0;

		for macro_info in &macros {
			// Skip if marked for ignore
			if macro_info.should_skip {
				// Copy the original macro as-is
				result.push_str(&content[last_end..macro_info.end]);
				last_end = macro_info.end;
				continue;
			}

			// Copy content before this macro
			result.push_str(&content[last_end..macro_info.start]);

			// Calculate base indentation for this macro
			let base_indent = Self::calculate_base_indent(content, macro_info.start);

			// Try to parse and format the macro
			match self.format_macro_tokens(
				&macro_info.tokens,
				&macro_info.original_text,
				base_indent,
				macro_info.kind,
			) {
				Ok(formatted) => match macro_info.kind {
					MacroKind::Page => {
						result.push_str("page!(");
						result.push_str(&formatted);
						result.push(')');
					}
					MacroKind::Form => {
						result.push_str("form! ");
						result.push_str(&formatted);
					}
				},
				Err(_) => {
					// If formatting fails, keep original
					result.push_str(&content[macro_info.start..macro_info.end]);
				}
			}

			last_end = macro_info.end;
		}

		// Copy remaining content
		result.push_str(&content[last_end..]);

		Ok(FormatResult {
			content: result,
			contains_page_macro: true,
			skipped: None,
		})
	}

	/// Find all page! and form! macros in the source.
	fn find_page_macros(&self, content: &str) -> Result<Vec<MacroInfo>, String> {
		// Try to parse as a complete Rust file first
		match parse_file(content) {
			Ok(file) => {
				let mut page_visitor = PageMacroVisitor::new(content);
				page_visitor.visit_file(&file);
				let mut form_visitor = FormMacroVisitor::new(content);
				form_visitor.visit_file(&file);

				let mut all_macros = page_visitor.macros;
				all_macros.extend(form_visitor.macros);
				Ok(all_macros)
			}
			Err(_) => {
				// If file parsing fails, fall back to text-based detection
				self.find_page_macros_text_based(content)
			}
		}
	}

	/// Text-based fallback for finding page! and form! macros.
	///
	/// Accepts both the compact forms and the TokenStream Display forms.
	fn find_page_macros_text_based(&self, content: &str) -> Result<Vec<MacroInfo>, String> {
		let mut macros = Vec::new();

		// Scan for page! macros (using paren delimiter)
		let mut search_start = 0;
		while let Some(hit) = find_page_bang_paren(&content[search_start..]) {
			let abs_start = search_start + hit.start;
			let abs_open = search_start + hit.paren_open;

			// Check if we're in a comment or string
			if self.is_in_comment_or_string(content, abs_start) {
				search_start = abs_start + 1;
				continue;
			}

			let content_start = abs_open + 1;

			if let Some(end_pos) = find_matching_paren(content, content_start) {
				let macro_content = &content[content_start..end_pos];

				if let Ok(tokens) = syn::parse_str::<proc_macro2::TokenStream>(macro_content) {
					macros.push(MacroInfo {
						start: abs_start,
						end: end_pos + 1,
						tokens,
						should_skip: false,
						original_text: macro_content.to_string(),
						kind: MacroKind::Page,
					});
				}

				search_start = end_pos + 1;
			} else {
				search_start = abs_start + 1;
			}
		}

		// Scan for form! macros (using brace delimiter)
		let mut search_start = 0;
		while let Some(hit) = find_form_bang_brace(&content[search_start..]) {
			let abs_start = search_start + hit.start;
			let abs_open = search_start + hit.paren_open;

			// Check if we're in a comment or string
			if self.is_in_comment_or_string(content, abs_start) {
				search_start = abs_start + 1;
				continue;
			}

			let content_start = abs_open + 1;

			if let Some(end_pos) = find_matching_brace(content, content_start) {
				let macro_content = &content[content_start..end_pos];

				if let Ok(tokens) = syn::parse_str::<proc_macro2::TokenStream>(macro_content) {
					macros.push(MacroInfo {
						start: abs_start,
						end: end_pos + 1,
						tokens,
						should_skip: false,
						original_text: macro_content.to_string(),
						kind: MacroKind::Form,
					});
				}

				search_start = end_pos + 1;
			} else {
				search_start = abs_start + 1;
			}
		}

		Ok(macros)
	}

	/// Check if a position is inside a comment or string literal.
	///
	/// Uses char_indices() to properly handle UTF-8 multi-byte characters.
	fn is_in_comment_or_string(&self, content: &str, pos: usize) -> bool {
		let mut chars = content.char_indices().peekable();
		let mut in_string = false;
		let mut in_line_comment = false;
		let mut in_block_comment = false;
		let mut escape_next = false;

		while let Some((offset, ch)) = chars.next() {
			if offset >= pos {
				break;
			}

			if escape_next {
				escape_next = false;
				continue;
			}

			// Check for two-character sequences
			if !in_string
				&& !in_block_comment
				&& ch == '/' && let Some(&(_, next_ch)) = chars.peek()
			{
				if next_ch == '/' {
					in_line_comment = true;
					chars.next(); // consume second '/'
					continue;
				} else if next_ch == '*' {
					in_block_comment = true;
					chars.next(); // consume '*'
					continue;
				}
			}

			// Check for end of line comment
			if in_line_comment && ch == '\n' {
				in_line_comment = false;
				continue;
			}

			// Check for end of block comment
			if in_block_comment
				&& ch == '*' && let Some(&(_, next_ch)) = chars.peek()
				&& next_ch == '/'
			{
				in_block_comment = false;
				chars.next(); // consume '/'
				continue;
			}

			// Handle strings
			if !in_line_comment && !in_block_comment {
				match ch {
					'\\' if in_string => escape_next = true,
					'"' => in_string = !in_string,
					_ => {}
				}
			}
		}

		in_string || in_line_comment || in_block_comment
	}

	/// Extract the span from a PageNode.
	#[allow(
		dead_code,
		reason = "will be used when blank-line detection is fully implemented"
	)]
	fn node_span(node: &PageNode) -> proc_macro2::Span {
		match node {
			PageNode::Element(e) => e.span,
			PageNode::Text(t) => t.span,
			PageNode::Expression(e) => e.span,
			PageNode::If(i) => i.span,
			PageNode::For(f) => f.span,
			PageNode::Component(c) => c.span,
			PageNode::Watch(w) => w.span,
		}
	}

	/// Detect node indices after which a blank line should be inserted.
	///
	/// Examines the original source text between adjacent nodes in the body.
	/// When `\n\n` appears between the end of node i and the start of node i+1,
	/// index i is added to the returned set.
	fn detect_blank_lines_between_nodes(_body: &PageBody, _original_text: &str) -> BTreeSet<usize> {
		// TODO(#4767): Detect blank lines from original text and re-insert between
		// formatted nodes. Spans from parsed AST nodes don't map back to byte
		// positions in the original body text, so this needs a text-scanning
		// approach that walks brace-depth in the original source.
		BTreeSet::new()
	}

	/// Format macro tokens to formatted string, dispatching based on MacroKind.
	fn format_macro_tokens(
		&self,
		tokens: &proc_macro2::TokenStream,
		original_text: &str,
		base_indent: usize,
		kind: MacroKind,
	) -> Result<String, String> {
		match kind {
			MacroKind::Page => {
				// Parse tokens as PageMacro
				let page_macro: PageMacro =
					syn::parse2(tokens.clone()).map_err(|e| format!("Parse error: {}", e))?;

				// Detect blank lines between nodes in original source
				let blank_lines =
					Self::detect_blank_lines_between_nodes(&page_macro.body, original_text);

				// Format the macro
				self.format_page_macro(&page_macro, base_indent, &blank_lines)
			}
			MacroKind::Form => {
				// Parse tokens as FormMacro
				let form_macro: FormMacro =
					syn::parse2(tokens.clone()).map_err(|e| format!("Parse error: {}", e))?;

				// Format the macro
				self.format_form_macro(&form_macro, base_indent)
			}
		}
	}

	/// Check if a page macro body is simple and can be formatted on a single line.
	fn is_simple_body(body: &PageBody) -> bool {
		// Simple if it has exactly one element with no attributes, events, or children
		if body.nodes.len() == 1
			&& let PageNode::Element(elem) = &body.nodes[0]
		{
			return elem.attrs.is_empty() && elem.events.is_empty() && elem.children.is_empty();
		}
		false
	}

	/// Format a PageMacro AST to string.
	fn format_page_macro(
		&self,
		macro_ast: &PageMacro,
		base_indent: usize,
		blank_lines: &BTreeSet<usize>,
	) -> Result<String, String> {
		let mut output = String::new();

		// Format closure parameters
		self.format_params(&mut output, &macro_ast.params);

		// Check if body is simple enough for single-line format
		if Self::is_simple_body(&macro_ast.body) {
			// Single-line format: || { div {} }
			output.push_str(" { ");
			if let PageNode::Element(elem) = &macro_ast.body.nodes[0] {
				output.push_str(&elem.tag.to_string());
				output.push_str(" {}");
			}
			output.push_str(" }");
		} else {
			// Multi-line format
			output.push_str(" {\n");
			self.format_body(
				&mut output,
				&macro_ast.body,
				base_indent + 1,
				0,
				blank_lines,
			);
			output.push_str(&self.make_indent(base_indent));
			output.push('}');
		}

		Ok(output)
	}

	/// Format a FormMacro AST to string.
	fn format_form_macro(
		&self,
		macro_ast: &FormMacro,
		base_indent: usize,
	) -> Result<String, String> {
		let mut output = String::new();
		let inner_indent = base_indent + 1;

		// Open brace
		output.push_str("{\n");

		// --- Header items ---

		// name (required)
		if let Some(name) = &macro_ast.name {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("name: ");
			output.push_str(&name.to_string());
			output.push_str(",\n");
		}

		// action (URL, server_fn, or None)
		match &macro_ast.action {
			FormAction::Url(url) => {
				let ind = self.make_indent(inner_indent);
				output.push_str(&ind);
				output.push_str("action: ");
				let url_str = Self::clean_expression_spaces(&url.to_token_stream().to_string());
				output.push_str(&url_str);
				output.push_str(",\n");
			}
			FormAction::ServerFn(path) => {
				let ind = self.make_indent(inner_indent);
				output.push_str(&ind);
				output.push_str("server_fn: ");
				let path_str = Self::clean_expression_spaces(&path.to_token_stream().to_string());
				output.push_str(&path_str);
				output.push_str(",\n");
			}
			FormAction::None => {}
		}

		// method (optional)
		if let Some(method) = &macro_ast.method {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("method: ");
			output.push_str(&method.to_string());
			output.push_str(",\n");
		}

		// class (optional)
		if let Some(class) = &macro_ast.class {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("class: ");
			let class_str = Self::clean_expression_spaces(&class.to_token_stream().to_string());
			output.push_str(&class_str);
			output.push_str(",\n");
		}

		// redirect_on_success / success_url
		if let Some(redirect) = &macro_ast.redirect_on_success {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("redirect_on_success: ");
			let redirect_str =
				Self::clean_expression_spaces(&redirect.to_token_stream().to_string());
			output.push_str(&redirect_str);
			output.push_str(",\n");
		}
		if let Some(success_url) = &macro_ast.success_url {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("success_url: ");
			let url_str = Self::clean_expression_spaces(&success_url.to_token_stream().to_string());
			output.push_str(&url_str);
			output.push_str(",\n");
		}

		// initial_loader / choices_loader
		if let Some(loader) = &macro_ast.initial_loader {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("initial_loader: ");
			let loader_str = Self::clean_expression_spaces(&loader.to_token_stream().to_string());
			output.push_str(&loader_str);
			output.push_str(",\n");
		}
		if let Some(loader) = &macro_ast.choices_loader {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("choices_loader: ");
			let loader_str = Self::clean_expression_spaces(&loader.to_token_stream().to_string());
			output.push_str(&loader_str);
			output.push_str(",\n");
		}

		// strip_arguments
		if !macro_ast.strip_arguments.is_empty() {
			let ind = self.make_indent(inner_indent);
			output.push_str(&ind);
			output.push_str("strip_arguments: {\n");
			for arg in &macro_ast.strip_arguments {
				let fi = self.make_indent(inner_indent + 1);
				let val_str =
					Self::clean_expression_spaces(&arg.value.to_token_stream().to_string());
				output.push_str(&fi);
				output.push_str(&arg.name.to_string());
				output.push_str(": ");
				output.push_str(&val_str);
				output.push_str(",\n");
			}
			output.push_str(&ind);
			output.push_str("},\n");
		}

		// Blank line before sections
		output.push('\n');

		// --- Sections ---

		// state section
		if let Some(state) = &macro_ast.state {
			self.format_form_state(&mut output, state, inner_indent);
			output.push('\n');
		}

		// fields section (always present)
		self.format_form_entries(&mut output, &macro_ast.fields, inner_indent);
		output.push('\n');

		// validators section
		if !macro_ast.validators.is_empty() {
			self.format_form_validators(&mut output, &macro_ast.validators, inner_indent);
			output.push('\n');
		}

		// callbacks section
		if macro_ast.callbacks.has_any() {
			self.format_form_callbacks(&mut output, &macro_ast.callbacks, inner_indent);
			output.push('\n');
		}

		// watch section
		if let Some(watch) = &macro_ast.watch {
			self.format_form_watch(&mut output, watch, inner_indent);
			output.push('\n');
		}

		// derived section
		if let Some(derived) = &macro_ast.derived {
			self.format_form_derived(&mut output, derived, inner_indent);
			output.push('\n');
		}

		// slots section
		if let Some(slots) = &macro_ast.slots {
			self.format_form_slots(&mut output, slots, inner_indent);
			output.push('\n');
		}

		// Close brace
		output.push_str(&self.make_indent(base_indent));
		output.push_str("}");

		Ok(output)
	}

	/// Format the form state section.
	fn format_form_state(&self, output: &mut String, state: &FormState, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str("state: {\n");
		let inner_ind = indent + 1;
		for field in &state.fields {
			let fi = self.make_indent(inner_ind);
			output.push_str(&fi);
			output.push_str(&field.name.to_string());
			output.push_str(",\n");
		}
		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format the form derived section.
	fn format_form_derived(&self, output: &mut String, derived: &FormDerived, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str("derived: {\n");
		let inner_ind = indent + 1;
		for item in &derived.items {
			let fi = self.make_indent(inner_ind);
			let name = item.name.to_string();
			let closure_str = self.format_closure_expression(&item.closure, inner_ind);
			output.push_str(&fi);
			output.push_str(&name);
			output.push_str(": ");
			output.push_str(&closure_str);
			output.push_str(",\n");
		}
		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format the form slots section.
	fn format_form_slots(&self, output: &mut String, slots: &FormSlots, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str("slots: {\n");
		let inner_ind = indent + 1;

		if let Some(before) = &slots.before_fields {
			self.format_slot_closure(output, "before_fields", before, inner_ind);
		}
		if let Some(after) = &slots.after_fields {
			self.format_slot_closure(output, "after_fields", after, inner_ind);
		}
		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Emit a named slot closure entry.
	fn format_slot_closure(
		&self,
		output: &mut String,
		name: &str,
		closure: &syn::ExprClosure,
		indent: usize,
	) {
		let fi = self.make_indent(indent);
		let expr_str = self.format_closure_expression(closure, indent);
		output.push_str(&fi);
		output.push_str(name);
		output.push_str(": ");
		output.push_str(&expr_str);
		output.push_str(",\n");
	}

	/// Format the form callbacks (emitted at the current indent level without a wrapper).
	fn format_form_callbacks(&self, output: &mut String, callbacks: &FormCallbacks, indent: usize) {
		if let Some(cb) = &callbacks.on_submit {
			self.format_callback_entry(output, "on_submit", cb, indent);
		}
		if let Some(cb) = &callbacks.on_success {
			self.format_callback_entry(output, "on_success", cb, indent);
		}
		if let Some(cb) = &callbacks.on_success_ref {
			self.format_callback_entry(output, "on_success_ref", cb, indent);
		}
		if let Some(cb) = &callbacks.on_error {
			self.format_callback_entry(output, "on_error", cb, indent);
		}
		if let Some(cb) = &callbacks.on_loading {
			self.format_callback_entry(output, "on_loading", cb, indent);
		}
	}

	/// Emit a single named callback entry.
	fn format_callback_entry(
		&self,
		output: &mut String,
		name: &str,
		closure: &syn::ExprClosure,
		indent: usize,
	) {
		let fi = self.make_indent(indent);
		let expr_str = self.format_closure_expression(closure, indent);
		output.push_str(&fi);
		output.push_str(name);
		output.push_str(": ");
		output.push_str(&expr_str);
		output.push_str(",\n");
	}

	/// Format the form watch section.
	fn format_form_watch(&self, output: &mut String, watch: &FormWatch, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str("watch: {\n");
		let inner_ind = indent + 1;
		for item in &watch.items {
			let fi = self.make_indent(inner_ind);
			let name = item.name.to_string();
			let closure_str = self.format_closure_expression(&item.closure, inner_ind);
			output.push_str(&fi);
			output.push_str(&name);
			output.push_str(": ");
			output.push_str(&closure_str);
			output.push_str(",\n");
		}
		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format the form validators section.
	fn format_form_validators(
		&self,
		output: &mut String,
		validators: &[FormValidator],
		indent: usize,
	) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str("validators: {\n");
		let inner_ind = indent + 1;

		for validator in validators {
			match validator {
				FormValidator::Field {
					field_name, rules, ..
				} => {
					let fi = self.make_indent(inner_ind);
					output.push_str(&fi);
					output.push_str(&field_name.to_string());
					if rules.is_empty() {
						output.push_str(": [],\n");
					} else {
						output.push_str(": [\n");
						self.format_validator_rules(output, rules, inner_ind + 1);
						output.push_str(&fi);
						output.push_str("],\n");
					}
				}
				FormValidator::Form { rules, .. } => {
					let fi = self.make_indent(inner_ind);
					output.push_str(&fi);
					if rules.is_empty() {
						output.push_str("@form: [],\n");
					} else {
						output.push_str("@form: [\n");
						self.format_validator_rules(output, rules, inner_ind + 1);
						output.push_str(&fi);
						output.push_str("],\n");
					}
				}
			}
		}

		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format individual validator rules.
	fn format_validator_rules(&self, output: &mut String, rules: &[ValidatorRule], indent: usize) {
		for rule in rules {
			let fi = self.make_indent(indent);
			// Scope annotation
			match &rule.scope {
				ValidatorScope::Both => {
					// Default — no annotation
				}
				ValidatorScope::Server => {
					output.push_str(&fi);
					output.push_str("#[server]\n");
				}
				ValidatorScope::Client { trigger } => {
					output.push_str(&fi);
					let trigger_str = match trigger {
						ClientTrigger::Submit => "submit",
						ClientTrigger::Input => "input",
						ClientTrigger::Blur => "blur",
					};
					output.push_str(&format!("#[client(on = {})]\n", trigger_str));
				}
				ValidatorScope::ServerAndClient { trigger } => {
					output.push_str(&fi);
					let trigger_str = match trigger {
						ClientTrigger::Submit => "submit",
						ClientTrigger::Input => "input",
						ClientTrigger::Blur => "blur",
					};
					output.push_str(&format!("#[server_and_client(on = {})]\n", trigger_str));
				}
			}

			let expr_str = Self::clean_expression_spaces(&rule.expr.to_token_stream().to_string());
			let msg_str =
				Self::clean_expression_spaces(&rule.message.to_token_stream().to_string());

			output.push_str(&fi);
			output.push_str(&expr_str);
			output.push_str(" => ");
			output.push_str(&msg_str);
			output.push_str(",\n");
		}
	}

	/// Format form field entries (fields, groups, submit buttons).
	fn format_form_entries(&self, output: &mut String, entries: &[FormFieldEntry], indent: usize) {
		let ind = self.make_indent(indent);
		if entries.is_empty() {
			output.push_str(&ind);
			output.push_str("fields: {}\n");
			return;
		}
		output.push_str(&ind);
		output.push_str("fields: {\n");
		let inner_ind = indent + 1;

		for entry in entries {
			match entry {
				FormFieldEntry::Field(field_def) => {
					self.format_form_field(output, field_def, inner_ind);
				}
				FormFieldEntry::Group(group) => {
					self.format_form_field_group(output, group, inner_ind);
				}
				FormFieldEntry::SubmitButton(button) => {
					self.format_form_submit_button(output, button, inner_ind);
				}
			}
		}

		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format a single form field definition.
	fn format_form_field(&self, output: &mut String, field: &FormFieldDef, indent: usize) {
		let ind = self.make_indent(indent);
		let name = field.name.to_string();
		let field_type_str =
			Self::clean_expression_spaces(&field.field_type.to_token_stream().to_string());

		output.push_str(&ind);
		output.push_str(&name);
		output.push_str(": ");
		output.push_str(&field_type_str);

		// Field properties
		if !field.properties.is_empty() {
			output.push_str(" {\n");
			let inner_ind = indent + 1;
			for prop in &field.properties {
				self.format_form_field_property(output, prop, inner_ind);
			}
			output.push_str(&ind);
			output.push_str("}\n");
		} else {
			output.push_str(",\n");
		}
	}

	/// Format a single form field property.
	fn format_form_field_property(
		&self,
		output: &mut String,
		prop: &FormFieldProperty,
		indent: usize,
	) {
		let fi = self.make_indent(indent);
		match prop {
			FormFieldProperty::Flag { name, .. } => {
				output.push_str(&fi);
				output.push_str(&name.to_string());
				output.push_str(",\n");
			}
			FormFieldProperty::Named { name, value, .. } => {
				let val_str = Self::clean_expression_spaces(&value.to_token_stream().to_string());
				output.push_str(&fi);
				output.push_str(&name.to_string());
				output.push_str(": ");
				output.push_str(&val_str);
				output.push_str(",\n");
			}
			FormFieldProperty::Widget { widget_type, .. } => {
				output.push_str(&fi);
				output.push_str("widget: ");
				output.push_str(&widget_type.to_string());
				output.push_str(",\n");
			}
			FormFieldProperty::Wrapper { element, .. } => {
				let elem_str = self.format_wrapper_element(element, indent);
				output.push_str(&fi);
				output.push_str("wrapper: ");
				output.push_str(&elem_str);
				output.push_str(",\n");
			}
			FormFieldProperty::Icon { element, .. } => {
				let elem_str = self.format_icon_element(element, indent);
				output.push_str(&fi);
				output.push_str("icon: ");
				output.push_str(&elem_str);
				output.push_str(",\n");
			}
			FormFieldProperty::IconPosition { position, .. } => {
				output.push_str(&fi);
				output.push_str("icon_position: ");
				output.push_str(&self.format_icon_position(position));
				output.push_str(",\n");
			}
			FormFieldProperty::Attrs { attrs, .. } => {
				output.push_str(&fi);
				output.push_str("attrs: {\n");
				let inner_ind = indent + 1;
				for attr in attrs {
					let ai = self.make_indent(inner_ind);
					let attr_name = attr.name.to_string().replace('_', "-");
					let val_str =
						Self::clean_expression_spaces(&attr.value.to_token_stream().to_string());
					output.push_str(&ai);
					output.push_str(&attr_name);
					output.push_str(": ");
					output.push_str(&val_str);
					output.push_str(",\n");
				}
				output.push_str(&fi);
				output.push_str("},\n");
			}
			FormFieldProperty::Bind { enabled, .. } => {
				output.push_str(&fi);
				output.push_str("bind: ");
				output.push_str(if *enabled { "true" } else { "false" });
				output.push_str(",\n");
			}
			FormFieldProperty::InitialFrom { field_name, .. } => {
				let val_str =
					Self::clean_expression_spaces(&field_name.to_token_stream().to_string());
				output.push_str(&fi);
				output.push_str("initial_from: ");
				output.push_str(&val_str);
				output.push_str(",\n");
			}
			FormFieldProperty::ChoicesFrom { field_name, .. } => {
				let val_str =
					Self::clean_expression_spaces(&field_name.to_token_stream().to_string());
				output.push_str(&fi);
				output.push_str("choices_from: ");
				output.push_str(&val_str);
				output.push_str(",\n");
			}
			FormFieldProperty::ChoiceValue { path, .. } => {
				let val_str = Self::clean_expression_spaces(&path.to_token_stream().to_string());
				output.push_str(&fi);
				output.push_str("choice_value: ");
				output.push_str(&val_str);
				output.push_str(",\n");
			}
			FormFieldProperty::ChoiceLabel { path, .. } => {
				let val_str = Self::clean_expression_spaces(&path.to_token_stream().to_string());
				output.push_str(&fi);
				output.push_str("choice_label: ");
				output.push_str(&val_str);
				output.push_str(",\n");
			}
		}
	}

	/// Format a `WrapperElement` to its DSL representation.
	fn format_wrapper_element(
		&self,
		element: &reinhardt_pages::ast::WrapperElement,
		indent: usize,
	) -> String {
		let mut out = String::new();
		out.push_str(&element.tag.to_string());
		if element.attrs.is_empty() {
			out.push_str(" {}");
		} else {
			out.push_str(" {\n");
			let ai = self.make_indent(indent + 1);
			for attr in &element.attrs {
				let val_str =
					Self::clean_expression_spaces(&attr.value.to_token_stream().to_string());
				out.push_str(&ai);
				out.push_str(&attr.name.to_string());
				out.push_str(": ");
				out.push_str(&val_str);
				out.push_str(",\n");
			}
			let ind = self.make_indent(indent);
			out.push_str(&ind);
			out.push('}');
		}
		out
	}

	/// Format an `IconElement` to its DSL representation.
	fn format_icon_element(
		&self,
		element: &reinhardt_pages::ast::IconElement,
		indent: usize,
	) -> String {
		let mut out = String::new();
		out.push_str("svg {\n");
		let inner = self.make_indent(indent + 1);
		for attr in &element.attrs {
			let val_str = Self::clean_expression_spaces(&attr.value.to_token_stream().to_string());
			out.push_str(&inner);
			out.push_str(&attr.name.to_string());
			out.push_str(": ");
			out.push_str(&val_str);
			out.push_str(",\n");
		}
		for child in &element.children {
			let child_str = self.format_icon_child(child, indent + 1);
			out.push_str(&child_str);
			out.push_str(",\n");
		}
		let ind = self.make_indent(indent);
		out.push_str(&ind);
		out.push('}');
		out
	}

	/// Format an `IconChild` node to its DSL representation.
	fn format_icon_child(&self, child: &reinhardt_pages::ast::IconChild, indent: usize) -> String {
		let mut out = String::new();
		let ci = self.make_indent(indent);
		out.push_str(&ci);
		out.push_str(&child.tag.to_string());
		out.push_str(" {\n");
		let inner = self.make_indent(indent + 1);
		for attr in &child.attrs {
			let val_str = Self::clean_expression_spaces(&attr.value.to_token_stream().to_string());
			out.push_str(&inner);
			out.push_str(&attr.name.to_string());
			out.push_str(": ");
			out.push_str(&val_str);
			out.push_str(",\n");
		}
		if !child.children.is_empty() {
			for nested in &child.children {
				let nested_str = self.format_icon_child(nested, indent + 1);
				out.push_str(&nested_str);
				out.push_str(",\n");
			}
		}
		out.push_str(&ci);
		out.push('}');
		out
	}

	/// Format `IconPosition` to its DSL representation.
	fn format_icon_position(&self, position: &reinhardt_pages::ast::IconPosition) -> String {
		match position {
			reinhardt_pages::ast::IconPosition::Left => "\"left\"".to_string(),
			reinhardt_pages::ast::IconPosition::Right => "\"right\"".to_string(),
			reinhardt_pages::ast::IconPosition::Label => "\"label\"".to_string(),
		}
	}

	/// Format a form field group.
	fn format_form_field_group(&self, output: &mut String, group: &FormFieldGroup, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str(&group.name.to_string());
		output.push_str(": Group {\n");
		let inner_ind = indent + 1;

		// Group-level label
		if let Some(label) = &group.label {
			let fi = self.make_indent(inner_ind);
			let label_str = Self::clean_expression_spaces(&label.to_token_stream().to_string());
			output.push_str(&fi);
			output.push_str("label: ");
			output.push_str(&label_str);
			output.push_str(",\n");
		}

		// Group-level class
		if let Some(class) = &group.class {
			let fi = self.make_indent(inner_ind);
			let class_str = Self::clean_expression_spaces(&class.to_token_stream().to_string());
			output.push_str(&fi);
			output.push_str("class: ");
			output.push_str(&class_str);
			output.push_str(",\n");
		}

		// Fields within the group
		if !group.fields.is_empty() {
			let fi = self.make_indent(inner_ind);
			output.push_str(&fi);
			output.push_str("fields: {\n");
			for field in &group.fields {
				self.format_form_field(output, field, inner_ind + 1);
			}
			output.push_str(&fi);
			output.push_str("},\n");
		}

		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format a form submit button definition.
	fn format_form_submit_button(
		&self,
		output: &mut String,
		button: &FormSubmitButtonDef,
		indent: usize,
	) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str(&button.name.to_string());
		output.push_str(": SubmitButton");

		if !button.properties.is_empty() {
			output.push_str(" {\n");
			let inner_ind = indent + 1;
			for prop in &button.properties {
				self.format_form_field_property(output, prop, inner_ind);
			}
			output.push_str(&ind);
			output.push_str("}\n");
		} else {
			output.push_str(",\n");
		}
	}

	/// Format closure parameters: |param: Type, ...|
	fn format_params(&self, output: &mut String, params: &[PageParam]) {
		output.push('|');
		for (i, param) in params.iter().enumerate() {
			if i > 0 {
				output.push_str(", ");
			}

			let param_name = param.name.to_string();
			let ty_str = param.ty.to_token_stream().to_string();

			output.push_str(&param_name);

			// Skip type annotation for underscore-only parameters with type inference
			// to preserve |_| format instead of |_: _|
			if param_name == "_" && ty_str.trim() == "_" {
				// No type annotation added
			} else {
				// Normal parameters or explicit type annotations
				output.push_str(": ");
				let cleaned = Self::clean_expression_spaces(&ty_str);
				output.push_str(&cleaned);
			}
		}
		output.push('|');
	}

	/// Format the page body.
	fn format_body(
		&self,
		output: &mut String,
		body: &PageBody,
		indent: usize,
		depth: usize,
		blank_lines: &BTreeSet<usize>,
	) {
		for (i, node) in body.nodes.iter().enumerate() {
			self.format_node(output, node, indent, depth);
			if blank_lines.contains(&i) {
				output.push('\n');
			}
		}
	}

	/// Format a single node.
	///
	/// The `depth` parameter tracks recursion depth to prevent stack overflow
	/// from deeply nested templates. When the maximum depth is exceeded,
	/// the node is rendered as a raw token stream instead.
	fn format_node(&self, output: &mut String, node: &PageNode, indent: usize, depth: usize) {
		if depth > MAX_FORMAT_DEPTH {
			// Prevent stack overflow: emit a comment indicating depth limit
			let ind = self.make_indent(indent);
			output.push_str(&ind);
			output.push_str("/* formatting depth limit exceeded */\n");
			return;
		}

		match node {
			PageNode::Element(elem) => self.format_element(output, elem, indent, depth),
			PageNode::Text(text) => self.format_text(output, text, indent),
			PageNode::Expression(expr) => self.format_expression(output, expr, indent),
			PageNode::If(if_node) => self.format_if(output, if_node, indent, depth),
			PageNode::For(for_node) => self.format_for(output, for_node, indent, depth),
			PageNode::Component(comp) => self.format_component(output, comp, indent, depth),
			PageNode::Watch(watch_node) => self.format_watch(output, watch_node, indent, depth),
		}
	}

	/// Format an element node.
	fn format_element(&self, output: &mut String, elem: &PageElement, indent: usize, depth: usize) {
		let ind = self.make_indent(indent);

		// Check if element is empty (no attrs, events, or children)
		let is_empty = elem.attrs.is_empty() && elem.events.is_empty() && elem.children.is_empty();

		// Element tag
		output.push_str(&ind);
		output.push_str(&elem.tag.to_string());

		if is_empty {
			// Empty element: single line format
			output.push_str(" {}\n");
		} else {
			// Non-empty element: multi-line format
			output.push_str(" {\n");

			// Attributes (one per line)
			for attr in &elem.attrs {
				self.format_attr(output, attr, indent + 1);
			}

			// Event handlers (one per line)
			for event in &elem.events {
				self.format_event(output, event, indent + 1);
			}

			// Children
			for child in &elem.children {
				self.format_node(output, child, indent + 1, depth + 1);
			}

			// Closing brace
			output.push_str(&ind);
			output.push_str("}\n");
		}
	}

	/// Format an attribute.
	fn format_attr(&self, output: &mut String, attr: &PageAttr, indent: usize) {
		let ind = self.make_indent(indent);
		let value_str = Self::clean_expression_spaces(&attr.value.to_token_stream().to_string());
		output.push_str(&ind);
		output.push_str(&attr.name.to_string());
		output.push_str(": ");
		output.push_str(&value_str);
		output.push_str(",\n");
	}

	/// Format an event handler.
	///
	/// Uses rustfmt to format complex closures for better readability.
	/// Empty closures (e.g., `|_| {}`) are kept as-is.
	fn format_event(&self, output: &mut String, event: &PageEvent, indent: usize) {
		let ind = self.make_indent(indent);

		// Format handler with rustfmt (empty closures are kept as-is)
		let handler_str = self.format_handler_expression(&event.handler, indent + 1);

		output.push_str(&ind);
		output.push('@');
		output.push_str(&event.event_type.to_string());
		output.push_str(": ");
		output.push_str(&handler_str);
		output.push_str(",\n");
	}

	/// Format a text node.
	fn format_text(&self, output: &mut String, text: &PageText, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		// Escape and quote the text
		let escaped = text.content.replace('\\', "\\\\").replace('"', "\\\"");
		output.push('"');
		output.push_str(&escaped);
		output.push_str("\"\n");
	}

	/// Clean up extra spaces in expression strings.
	fn clean_expression_spaces(s: &str) -> String {
		// Static regex compilation (compiled once, reused)
		// [\w:]+ matches identifiers and path-qualified names (e.g., Vec::new, std::iter::once)
		static IDENT_PAREN: LazyLock<Regex> = LazyLock::new(|| {
			Regex::new(r"([\w:]+) \(").expect("Failed to compile IDENT_PAREN regex")
		});
		static IDENT_MACRO: LazyLock<Regex> = LazyLock::new(|| {
			Regex::new(r"([\w:]+) !").expect("Failed to compile IDENT_MACRO regex")
		});
		// Match generic type opening: Result <T> -> Result<T>
		// Only matches when followed by an identifier (not =, <, > which indicate operators)
		static IDENT_ANGLE: LazyLock<Regex> = LazyLock::new(|| {
			Regex::new(r"([\w:>)]+) <([A-Za-z_&'\[(\*])")
				.expect("Failed to compile IDENT_ANGLE regex")
		});
		// Match generic type closing: String > -> String>
		// Only matches when preceded by an identifier/closing bracket and not followed by =, >, <
		static ANGLE_CLOSE: LazyLock<Regex> = LazyLock::new(|| {
			Regex::new(r"([\w>)]) >([\s,;)}\]>])").expect("Failed to compile ANGLE_CLOSE regex")
		});

		let s = s
			// Existing: Dot and method chaining
			.replace(" . ", ".")

			// Existing: Parentheses (function calls, tuples)
			.replace(" ( ", "(")
			.replace(" )", ")")
			.replace("( ", "(")
			.replace(" )", ")")
			.replace(" ()", "()")

			// Path separator must be processed before angle brackets
			// to avoid leaving a space before :: (e.g., collect ::<Vec<_>>)
			.replace(" :: ", "::")
			.replace(" ::", "::")
			.replace(":: ", "::")

			// Generic type angle brackets: Vec < String > -> Vec<String>
			// These handle spaces around < and > in generic type parameters
			// Note: We don't use ".replace("> ", ">")" because it would incorrectly
			// affect arrow operators like "-> Result" turning them into "->Result"
			.replace("< ", "<")
			.replace(" <", "<")
			.replace(" >", ">")

			// New: Arrays and slices
			.replace("[ ", "[")
			.replace(" ]", "]")
			.replace(" ; ", "; ")  // Array size separator (preserve space after semicolon)

			// New: Reference types
			.replace("& ", "&")

			// New: Pointer types
			.replace("* const ", "*const ")
			.replace("* mut ", "*mut ")

			// New: Lifetime syntax
			.replace("for < ", "for<")
			.replace(" > fn", ">fn")

			// New: Comma in generics (Result<T, E>)
			.replace(" , ", ", ") // Note: Preserve space after comma

			// New: Macro calls (format! macro, etc.)
			.replace("! (", "!(") // Macro symbol before parenthesis
			.replace("! [", "![") // Macro with brackets
			.replace("! {", "!{") // Macro with braces

			// New: Closure parameter pipes
			// Handle closure syntax: | param | -> |param|
			// Note: OR operator (a | b) has different context and should be preserved
			.replace("| }", "|}") // Closing of empty closure before brace
			;

		// Apply regex replacements for identifier patterns
		let s = IDENT_PAREN.replace_all(&s, "$1("); // identifier ( -> identifier(
		let s = IDENT_MACRO.replace_all(&s, "$1!"); // identifier ! -> identifier!
		let s = IDENT_ANGLE.replace_all(&s, "$1<$2"); // identifier <T -> identifier<T (for generics)
		// Apply closing angle bracket repeatedly for nested generics like Option<String >
		let s = ANGLE_CLOSE.replace_all(&s, "$1>$2");

		// Handle closure pipes: | x | -> |x|, | x, y | -> |x, y|, || -> ||
		// This regex matches closure parameter lists between pipes
		static CLOSURE_PARAMS: LazyLock<Regex> = LazyLock::new(|| {
			Regex::new(r"\| ([^|]*?) \|").expect("Failed to compile CLOSURE_PARAMS regex")
		});
		let s = CLOSURE_PARAMS.replace_all(&s, |caps: &regex::Captures| {
			let inner = &caps[1];
			// Clean up spaces around commas in closure params
			let cleaned = inner.trim();
			format!("|{}|", cleaned)
		});

		s.into_owned()
	}

	/// Check if the expression is an empty closure (e.g., `|_| {}`, `|| {}`)
	///
	/// Empty closures are kept as-is without rustfmt formatting.
	fn is_empty_closure(expr: &syn::Expr) -> bool {
		if let syn::Expr::Closure(closure) = expr
			&& let syn::Expr::Block(block) = closure.body.as_ref()
		{
			return block.block.stmts.is_empty();
		}
		false
	}

	/// Format Rust code with rustfmt
	///
	/// Falls back to the input code if rustfmt is not available or fails.
	fn format_with_rustfmt(&self, code: &str) -> String {
		use std::io::Write;
		use std::process::Stdio;

		let mut cmd = Command::new("rustfmt");
		self.rustfmt_options.apply_to_command(&mut cmd);

		// Fallback to default edition if no config is specified
		if self.rustfmt_options.config_path.is_none() && self.rustfmt_options.edition.is_none() {
			cmd.arg("--edition=2024");
		}

		let child = cmd
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn();

		match child {
			Ok(mut child_process) => {
				if let Some(stdin) = child_process.stdin.as_mut() {
					let _ = stdin.write_all(code.as_bytes());
				}
				match child_process.wait_with_output() {
					Ok(output) if output.status.success() => {
						String::from_utf8(output.stdout).unwrap_or_else(|_| code.to_string())
					}
					_ => code.to_string(),
				}
			}
			Err(_) => code.to_string(),
		}
	}

	/// Find the end of an expression considering nested braces
	fn find_expression_end(s: &str) -> Option<usize> {
		let mut brace_depth = 0;
		let mut paren_depth = 0;
		let mut in_string = false;
		let mut escape_next = false;

		for (i, c) in s.chars().enumerate() {
			if escape_next {
				escape_next = false;
				continue;
			}

			match c {
				'\\' if in_string => escape_next = true,
				'"' if !in_string => in_string = true,
				'"' if in_string => in_string = false,
				'{' if !in_string => brace_depth += 1,
				'}' if !in_string => brace_depth -= 1,
				'(' if !in_string => paren_depth += 1,
				')' if !in_string => paren_depth -= 1,
				';' if !in_string && brace_depth == 0 && paren_depth == 0 => return Some(i),
				_ => {}
			}
		}
		None
	}

	/// Extract the handler expression from the wrapper code
	///
	/// Pattern: `let _handler = <expr>;`
	fn extract_handler_from_wrapper(formatted: &str) -> Option<String> {
		let start_marker = "let _handler = ";
		let start = formatted.find(start_marker)? + start_marker.len();
		let handler_part = &formatted[start..];
		let end = Self::find_expression_end(handler_part)?;
		Some(handler_part[..end].trim().to_string())
	}

	/// Apply base indentation to each line of a multi-line handler
	fn apply_base_indent(&self, handler: &str, base_indent: usize) -> String {
		let lines: Vec<&str> = handler.lines().collect();

		if lines.len() == 1 {
			return handler.to_string();
		}

		// First line has no additional indent (format_event adds the base indent)
		// Subsequent lines get the base indent applied
		let indent_str = self.make_indent(base_indent);
		let mut result = lines[0].to_string();

		for line in &lines[1..] {
			result.push('\n');
			if !line.trim().is_empty() {
				result.push_str(&indent_str);
			}
			result.push_str(line);
		}

		result
	}

	/// Format an event handler expression with rustfmt
	///
	/// Empty closures are kept as-is, complex closures are formatted with rustfmt.
	fn format_handler_expression(&self, expr: &syn::Expr, base_indent: usize) -> String {
		// Empty closures are kept as-is
		if Self::is_empty_closure(expr) {
			return Self::clean_expression_spaces(&expr.to_token_stream().to_string());
		}

		let token_str = expr.to_token_stream().to_string();

		if token_str.contains("page !") || token_str.contains("form !") {
			let cleaned = Self::clean_expression_spaces(&token_str);
			let wrapper_code = format!("fn _wrapper() {{ let _handler = {}; }}", cleaned);

			let protect_result = self.protect_page_macros(&wrapper_code);

			let Ok(file) = syn::parse_file(&protect_result.protected_content) else {
				return cleaned;
			};

			let prettyplease_output = prettyplease::unparse(&file);
			let formatted = self.format_with_rustfmt(&prettyplease_output);
			let restored = self.restore_page_macros_recursive(&formatted, &protect_result.backups);

			if let Some(handler_str) = Self::extract_handler_from_wrapper(&restored) {
				return self.apply_base_indent(&handler_str, base_indent);
			}
			return cleaned;
		}

		// Wrap the expression in a valid Rust file
		let wrapper_code = format!("fn _wrapper() {{ let _handler = {}; }}", token_str);

		let Ok(file) = syn::parse_file(&wrapper_code) else {
			return Self::clean_expression_spaces(&token_str);
		};

		// Format with prettyplease + rustfmt
		let prettyplease_output = prettyplease::unparse(&file);
		let formatted = self.format_with_rustfmt(&prettyplease_output);

		let Some(handler_str) = Self::extract_handler_from_wrapper(&formatted) else {
			return Self::clean_expression_spaces(&token_str);
		};

		// Apply base indentation
		self.apply_base_indent(&handler_str, base_indent)
	}
	/// Format a closure expression, using rustfmt for block-body closures.
	fn format_closure_expression(&self, closure: &syn::ExprClosure, base_indent: usize) -> String {
		// For non-block closures (e.g., |x| x + 1), clean_expression_spaces is sufficient
		if !matches!(closure.body.as_ref(), syn::Expr::Block(_)) {
			return Self::clean_expression_spaces(&closure.to_token_stream().to_string());
		}

		let token_str = closure.to_token_stream().to_string();

		if token_str.contains("page !") || token_str.contains("form !") {
			let cleaned = Self::clean_expression_spaces(&token_str);
			let wrapper_code = format!("fn _wrapper() {{ let _handler = {}; }}", cleaned);

			let protect_result = self.protect_page_macros(&wrapper_code);

			let Ok(file) = syn::parse_file(&protect_result.protected_content) else {
				return cleaned;
			};

			let prettyplease_output = prettyplease::unparse(&file);
			let formatted = self.format_with_rustfmt(&prettyplease_output);
			let restored = self.restore_page_macros_recursive(&formatted, &protect_result.backups);

			if let Some(handler_str) = Self::extract_handler_from_wrapper(&restored) {
				return self.apply_base_indent(&handler_str, base_indent);
			}
			return cleaned;
		}

		// Wrap the full closure in a valid Rust file for formatting
		let wrapper_code = format!("fn _wrapper() {{ let _handler = {}; }}", token_str);

		let Ok(file) = syn::parse_file(&wrapper_code) else {
			return Self::clean_expression_spaces(&token_str);
		};

		// Format with prettyplease + rustfmt
		let prettyplease_output = prettyplease::unparse(&file);
		let formatted = self.format_with_rustfmt(&prettyplease_output);

		let Some(handler_str) = Self::extract_handler_from_wrapper(&formatted) else {
			return Self::clean_expression_spaces(&token_str);
		};

		self.apply_base_indent(&handler_str, base_indent)
	}

	/// Format a Rust expression with rustfmt when it exceeds the line length threshold.
	///
	/// Returns `(formatted_string, is_multiline)`.
	/// Short expressions are returned as-is. Long expressions are wrapped in a
	/// temporary function, formatted with prettyplease + rustfmt, and then extracted.
	fn format_rust_expression(&self, expr: &syn::Expr, base_indent: usize) -> (String, bool) {
		let cleaned = Self::clean_expression_spaces(&expr.to_token_stream().to_string());

		// Estimate the total line length: indent + "{ " + expr + " }"
		// Use 4 as the display width per indent level (tab = 4 spaces equivalent)
		let indent_width = base_indent * 4;
		let total_len = indent_width + 2 + cleaned.len() + 2; // "{ " and " }"

		if total_len <= EXPRESSION_LINE_LENGTH_THRESHOLD {
			return (cleaned, false);
		}

		// Wrap in a valid Rust file for formatting
		let wrapper_code = format!(
			"fn _wrapper() {{ let _handler = {}; }}",
			expr.to_token_stream()
		);

		// Protect nested page! macros before formatting
		let protect_result = self.protect_page_macros(&wrapper_code);

		// Parse with syn
		let Ok(file) = syn::parse_file(&protect_result.protected_content) else {
			return (cleaned, false);
		};

		// Format with prettyplease + rustfmt
		let prettyplease_output = prettyplease::unparse(&file);
		let formatted = self.format_with_rustfmt(&prettyplease_output);

		// Restore nested page! macros, recursively re-formatting each one so
		// that nested page!() invocations are pretty-printed instead of being
		// re-inserted as the compact TokenStream stringification captured at
		// protect time.
		let restored = self.restore_page_macros_recursive(&formatted, &protect_result.backups);

		// Extract the expression from the wrapper
		let Some(expr_str) = Self::extract_handler_from_wrapper(&restored) else {
			return (cleaned, false);
		};

		// Apply base indentation
		let indented = self.apply_base_indent(&expr_str, base_indent);
		let is_multiline = indented.contains('\n');
		(indented, is_multiline)
	}

	/// Format an expression node.
	///
	/// Short expressions are kept on a single line. Long expressions are formatted
	/// with rustfmt and rendered as a multiline braced block.
	fn format_expression(&self, output: &mut String, expr: &PageExpression, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);

		let (formatted, is_multiline) = self.format_rust_expression(&expr.expr, indent + 1);

		if expr.braced {
			if is_multiline {
				let inner_ind = self.make_indent(indent + 1);
				output.push_str("{\n");
				output.push_str(&inner_ind);
				output.push_str(&formatted);
				output.push('\n');
				output.push_str(&ind);
				output.push_str("}\n");
			} else {
				output.push_str("{ ");
				output.push_str(&formatted);
				output.push_str(" }\n");
			}
		} else {
			output.push_str(&formatted);
			output.push('\n');
		}
	}

	/// Format an if node.
	fn format_if(&self, output: &mut String, if_node: &PageIf, indent: usize, depth: usize) {
		let ind = self.make_indent(indent);

		// if condition {
		output.push_str(&ind);
		output.push_str("if ");
		output.push_str(&Self::clean_expression_spaces(
			&if_node.condition.to_token_stream().to_string(),
		));
		output.push_str(" {\n");

		// then branch
		for node in &if_node.then_branch {
			self.format_node(output, node, indent + 1, depth + 1);
		}

		// else branch
		match &if_node.else_branch {
			Some(PageElse::Block(nodes)) => {
				output.push_str(&ind);
				output.push_str("} else {\n");
				for node in nodes {
					self.format_node(output, node, indent + 1, depth + 1);
				}
				output.push_str(&ind);
				output.push_str("}\n");
			}
			Some(PageElse::If(nested_if)) => {
				output.push_str(&ind);
				output.push_str("} else ");
				// Format the nested if without initial indent
				self.format_if_inline(output, nested_if, indent, depth + 1);
			}
			None => {
				output.push_str(&ind);
				output.push_str("}\n");
			}
		}
	}

	/// Format an if node inline (for else if chains).
	fn format_if_inline(&self, output: &mut String, if_node: &PageIf, indent: usize, depth: usize) {
		if depth > MAX_FORMAT_DEPTH {
			output.push_str("/* else-if chain depth limit exceeded */ {}\n");
			return;
		}

		let ind = self.make_indent(indent);

		output.push_str("if ");
		output.push_str(&Self::clean_expression_spaces(
			&if_node.condition.to_token_stream().to_string(),
		));
		output.push_str(" {\n");

		for node in &if_node.then_branch {
			self.format_node(output, node, indent + 1, depth + 1);
		}

		match &if_node.else_branch {
			Some(PageElse::Block(nodes)) => {
				output.push_str(&ind);
				output.push_str("} else {\n");
				for node in nodes {
					self.format_node(output, node, indent + 1, depth + 1);
				}
				output.push_str(&ind);
				output.push_str("}\n");
			}
			Some(PageElse::If(nested_if)) => {
				output.push_str(&ind);
				output.push_str("} else ");
				self.format_if_inline(output, nested_if, indent, depth + 1);
			}
			None => {
				output.push_str(&ind);
				output.push_str("}\n");
			}
		}
	}

	/// Format a for node.
	fn format_for(&self, output: &mut String, for_node: &PageFor, indent: usize, depth: usize) {
		let ind = self.make_indent(indent);

		output.push_str(&ind);
		output.push_str("for ");
		output.push_str(&Self::clean_expression_spaces(
			&for_node.pat.to_token_stream().to_string(),
		));
		output.push_str(" in ");
		output.push_str(&Self::clean_expression_spaces(
			&for_node.iter.to_token_stream().to_string(),
		));
		output.push_str(" {\n");

		for node in &for_node.body {
			self.format_node(output, node, indent + 1, depth + 1);
		}

		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format a watch node.
	fn format_watch(
		&self,
		output: &mut String,
		watch_node: &reinhardt_pages::ast::PageWatch,
		indent: usize,
		depth: usize,
	) {
		let ind = self.make_indent(indent);

		output.push_str(&ind);
		output.push_str("watch {\n");

		self.format_node(output, &watch_node.expr, indent + 1, depth + 1);

		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format a component call.
	fn format_component(
		&self,
		output: &mut String,
		comp: &PageComponent,
		indent: usize,
		depth: usize,
	) {
		let ind = self.make_indent(indent);

		output.push_str(&ind);
		output.push_str(&comp.name.to_string());
		output.push('(');

		// Arguments
		for (i, arg) in comp.args.iter().enumerate() {
			if i > 0 {
				output.push_str(", ");
			}
			output.push_str(&arg.name.to_string());
			output.push_str(": ");
			output.push_str(&Self::clean_expression_spaces(
				&arg.value.to_token_stream().to_string(),
			));
		}

		output.push(')');

		// Children
		if let Some(children) = &comp.children {
			output.push_str(" {\n");
			for child in children {
				self.format_node(output, child, indent + 1, depth + 1);
			}
			output.push_str(&ind);
			output.push('}');
		}

		output.push('\n');
	}

	/// Create indentation string.
	fn make_indent(&self, level: usize) -> String {
		self.indent.repeat(level)
	}

	/// Check if the file has an ignore-all marker at the beginning.
	///
	/// This checks the first 50 lines of the file for a comment containing
	/// `reinhardt-fmt:ignore-all`. The marker must appear before any code line.
	pub(crate) fn has_ignore_all_marker(&self, source: &str) -> bool {
		const MARKER: &str = "reinhardt-fmt:ignore-all";

		// Check only the first 50 lines for performance
		for line in source.lines().take(50) {
			let trimmed = line.trim();

			// Check comment lines only
			if let Some(comment) = trimmed.strip_prefix("//") {
				let comment_content = comment.trim();
				// Remove spaces for flexible matching
				if comment_content.replace(' ', "").contains(MARKER) {
					return true;
				}
			}

			// Stop at first code line (non-comment, non-empty)
			if !trimmed.is_empty() && !trimmed.starts_with("//") {
				break;
			}
		}
		false
	}

	/// Find all ignore ranges (off/on pairs) in the source code.
	///
	/// Returns a list of byte offset ranges where formatting should be skipped.
	/// Warns if there are nested 'off' markers or unmatched markers.
	fn find_ignore_ranges(&self, source: &str) -> Vec<(usize, usize)> {
		const OFF_MARKER: &str = "reinhardt-fmt:off";
		const ON_MARKER: &str = "reinhardt-fmt:on";

		let mut ranges = Vec::new();
		let mut current_off_start: Option<usize> = None;
		let mut byte_offset = 0;
		let total_len = source.len();

		for line in source.lines() {
			let trimmed = line.trim();

			if let Some(comment) = trimmed.strip_prefix("//") {
				let comment_content = comment.trim().replace(' ', "");

				if comment_content.contains(OFF_MARKER) {
					if current_off_start.is_some() {
						eprintln!(
							"Warning: Nested 'reinhardt-fmt: off' at byte {}",
							byte_offset
						);
						// Don't update current_off_start if already set (nested case)
					} else {
						current_off_start = Some(byte_offset);
					}
				} else if comment_content.contains(ON_MARKER) {
					if let Some(start) = current_off_start.take() {
						ranges.push((start, byte_offset));
					} else {
						eprintln!(
							"Warning: 'reinhardt-fmt: on' without matching 'off' at byte {}",
							byte_offset
						);
					}
				}
			}

			byte_offset += line.len() + 1; // +1 for newline
		}

		// Handle unclosed range - extend to end of file
		if let Some(start) = current_off_start {
			eprintln!("Warning: Unclosed 'reinhardt-fmt: off' at end of file");
			ranges.push((start, total_len));
		}

		ranges
	}

	/// Check if an individual macro has an ignore marker on the previous line.
	///
	/// The marker must be on the line immediately before the macro (no blank lines).
	fn has_individual_ignore_marker(&self, source: &str, macro_start: usize) -> bool {
		const MARKER: &str = "reinhardt-fmt:ignore";

		// If macro is at the start of the file, no previous line exists
		if macro_start == 0 {
			return false;
		}

		// Find the start of the current line (where the macro is)
		let line_start = source[..macro_start]
			.rfind('\n')
			.map(|pos| pos + 1)
			.unwrap_or(0);

		// If this is the first line, no previous line exists
		if line_start == 0 {
			return false;
		}

		// Find the end of the previous line (newline character position)
		let prev_line_end = line_start - 1; // This is the '\n' character

		// Find the start of the previous line
		let prev_line_start = source[..prev_line_end]
			.rfind('\n')
			.map(|pos| pos + 1)
			.unwrap_or(0);

		// Extract the previous line
		let prev_line = &source[prev_line_start..prev_line_end];
		let trimmed = prev_line.trim();

		// Check if it's a comment with the ignore marker
		if let Some(comment) = trimmed.strip_prefix("//") {
			let comment_content = comment.trim().replace(' ', "");
			return comment_content.contains(MARKER);
		}

		false
	}

	/// Apply ignore markers to macros, setting their should_skip flags.
	///
	/// Priority order:
	/// 1. Individual macro ignore (highest) - implemented in Phase 3
	/// 2. Range ignore (medium) - implemented in Phase 2
	/// 3. File-wide ignore (lowest) - handled in format() method
	fn apply_ignore_markers(&self, source: &str, macros: &mut [MacroInfo]) {
		// Find all ignore ranges
		let ignore_ranges = self.find_ignore_ranges(source);

		// Apply markers to each macro
		for macro_info in macros.iter_mut() {
			// Priority 1: Individual macro ignore (highest priority)
			if self.has_individual_ignore_marker(source, macro_info.start) {
				macro_info.should_skip = true;
				continue;
			}

			// Priority 2: Range ignore (medium priority)
			for (range_start, range_end) in &ignore_ranges {
				if macro_info.start >= *range_start && macro_info.start < *range_end {
					macro_info.should_skip = true;
					break;
				}
			}

			// Priority 3: File-wide ignore is already handled in format()
		}
	}

	/// Protect page! macros by replacing them with placeholders.
	///
	/// This allows rustfmt to process the surrounding Rust code without
	/// modifying the page! macro contents. The macros can be restored
	/// using `restore_page_macros`.
	///
	/// # Placeholder Format
	///
	/// Each page! macro is replaced with:
	/// ```text
	/// __reinhardt_placeholder__!(/*n*/)
	/// ```
	/// where `n` is a unique identifier.
	///
	/// # Example
	///
	/// ```text
	/// // Before:
	/// let view = page!(|| { div { "hello" } })(props);
	///
	/// // After:
	/// let view = __reinhardt_placeholder_0__!()(props);
	/// ```
	pub(crate) fn protect_page_macros(&self, content: &str) -> ProtectResult {
		// Quick check: accept `page!(`/`form!(`/`form!{` and whitespace variants.
		if find_page_bang_paren(content).is_none()
			&& find_form_bang_paren(content).is_none()
			&& find_form_bang_brace(content).is_none()
		{
			return ProtectResult {
				protected_content: content.to_string(),
				backups: Vec::new(),
			};
		}

		// Find all page! and form! macros
		let macros = match self.find_page_macros(content) {
			Ok(m) => m,
			Err(_) => {
				return ProtectResult {
					protected_content: content.to_string(),
					backups: Vec::new(),
				};
			}
		};

		if macros.is_empty() {
			return ProtectResult {
				protected_content: content.to_string(),
				backups: Vec::new(),
			};
		}

		// Sort macros by position
		let mut macros = macros;
		macros.sort_by_key(|m| m.start);

		// Build result by replacing each macro with placeholder
		let mut result = String::with_capacity(content.len());
		let mut backups = Vec::with_capacity(macros.len());
		let mut last_end = 0;

		for (id, macro_info) in macros.iter().enumerate() {
			// Copy content before this macro
			result.push_str(&content[last_end..macro_info.start]);

			// Save original macro text
			let original = content[macro_info.start..macro_info.end].to_string();
			backups.push(PageMacroBackup {
				id,
				original,
				kind: macro_info.kind,
			});

			// Insert placeholder (macro format so rustfmt doesn't touch it)
			result.push_str(&format!("__reinhardt_placeholder_{}__!()", id));

			last_end = macro_info.end;
		}

		// Copy remaining content
		result.push_str(&content[last_end..]);

		ProtectResult {
			protected_content: result,
			backups,
		}
	}

	/// Restore page! macros from placeholders.
	///
	/// This reverses the effect of `protect_page_macros`, replacing
	/// placeholders with the original page! macro content.
	pub(crate) fn restore_page_macros(content: &str, backups: &[PageMacroBackup]) -> String {
		if backups.is_empty() {
			return content.to_string();
		}

		let mut result = content.to_string();

		// Replace placeholders in reverse order to maintain correct positions
		for backup in backups.iter().rev() {
			let placeholder = format!("__reinhardt_placeholder_{}__!()", backup.id);
			result = result.replace(&placeholder, &backup.original);
		}

		result
	}

	/// Restore page! macros from placeholders, re-formatting each one so
	/// nested page! invocations get the same pretty-printing as top-level
	/// ones. The base indent for the recursive format pass is computed from
	/// the column where the placeholder appears in `content`.
	pub(crate) fn restore_page_macros_recursive(
		&self,
		content: &str,
		backups: &[PageMacroBackup],
	) -> String {
		if backups.is_empty() {
			return content.to_string();
		}

		let mut result = content.to_string();

		for backup in backups.iter().rev() {
			let placeholder = format!("__reinhardt_placeholder_{}__!()", backup.id);
			let replacement = self
				.format_inner_page_macro(&backup.original, &result, &placeholder, backup.kind)
				.unwrap_or_else(|| backup.original.clone());
			result = result.replace(&placeholder, &replacement);
		}

		result
	}

	/// Reformat a single backed-up `page!(...)` or `form!(...)` string with
	/// `format_macro_tokens`, using the indentation of the placeholder's line
	/// as the base indent. Returns `None` if parsing or re-formatting fails,
	/// in which case the caller falls back to the original (unformatted) backup text.
	fn format_inner_page_macro(
		&self,
		original: &str,
		surrounding: &str,
		placeholder: &str,
		kind: MacroKind,
	) -> Option<String> {
		// Extract `<inner>` from `page!(<inner>)` or `form!(<inner>)`.
		let (head, tail) = match kind {
			MacroKind::Page => {
				let hit = find_page_bang_paren(original)?;
				let head = hit.paren_open + 1;
				let tail = find_matching_paren(original, head)?;
				(head, tail)
			}
			MacroKind::Form => {
				let hit = find_form_bang_brace(original)?;
				let head = hit.paren_open + 1;
				let tail = find_matching_brace(original, head)?;
				(head, tail)
			}
		};
		if tail <= head {
			return None;
		}
		let inner = &original[head..tail];

		// Reparse so the recursive formatter receives a real TokenStream.
		let tokens = syn::parse_str::<proc_macro2::TokenStream>(inner).ok()?;

		// Compute base indent from the placeholder's column (tabs only — the
		// file uses hard tabs per rustfmt.toml `hard_tabs = true`).
		let pos = surrounding.find(placeholder)?;
		let line_start = surrounding[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
		let base_indent = surrounding[line_start..pos]
			.chars()
			.filter(|c| *c == '\t')
			.count();

		let formatted = self
			.format_macro_tokens(&tokens, inner, base_indent, kind)
			.ok()?;
		match kind {
			MacroKind::Page => Some(format!("page!({})", formatted)),
			MacroKind::Form => Some(format!("form! {}", formatted)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_format_simple_element() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { "hello" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("div {"));
		assert!(result.content.contains("\"hello\""));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_with_attributes() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div class="foo" { "hello" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("div"));
		assert!(result.content.contains("class"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_no_change_non_page() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "fn main() { println!(\"hello\"); }";

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(input, result.content);
		assert!(!result.contains_page_macro);
	}

	#[rstest]
	fn test_skip_page_in_string() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"fn main() { let s = "page!(|| { div { } })"; }"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("page!(|| { div { } })"));
		assert!(!result.contains_page_macro);
	}

	#[rstest]
	fn test_skip_page_in_comment() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// page!(|| { div { } })
fn main() {}"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("// page!(|| { div { } })"));
		assert!(!result.contains_page_macro);
	}

	#[rstest]
	fn test_format_with_params() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|name: String| { div { { name } } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("name: String"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_nested_elements() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { p { "hello" } } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("div {"));
		assert!(result.content.contains("p {"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_if_node() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { @if true { div { } } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("@if"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_for_node() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { @for item in items { div { } } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("@for"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_component() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { <MyComponent /> })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("<MyComponent"));
		assert!(result.content.contains("/>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_event_handler() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { button { @click: |_| {}, "Click" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("@click"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_safety_complex_non_page_file() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"
//! Module documentation

use std::collections::HashMap;

/// A complex struct
#[derive(Debug, Clone)]
pub struct MyStruct<T> {
	field: T,
}

impl<T> MyStruct<T> {
	pub fn new(field: T) -> Self {
		Self { field }
	}
}

// Some comment about the function
fn complex_function(x: i32, y: i32) -> i32 {
	x + y
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_something() {
		assert_eq!(2 + 2, 4);
	}
}
"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(input, result.content);
		assert!(!result.contains_page_macro);
	}

	// ========================================
	// Tests for generic type formatting
	// ========================================

	#[rstest]
	fn test_format_params_with_vec() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|items: Vec<String>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("items: Vec<String>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_option() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|value: Option<i32>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("value: Option<i32>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_result() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|res: Result<String, Error>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("res: Result<String, Error>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_nested_generics() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|items: Vec<Option<String>>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("items: Vec<Option<String>>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_multiple_generics() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|map: HashMap<String, i32>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("map: HashMap<String, i32>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_references() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|s: &str| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("s: &str"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_arrays() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|arr: [i32; 5]| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("arr: [i32; 5]"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_tuples() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|t: (String, i32)| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("t: (String, i32)"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_path_types() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|v: std::vec::Vec<String>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("v: std::vec::Vec<String>"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_with_complex_types() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|f: Box<dyn Fn() -> Result<(), Error>>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(
			result
				.content
				.contains("f: Box<dyn Fn() -> Result<(), Error>>")
		);
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_params_types_idempotent() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|vec: Vec<String>, opt: Option<i32>, res: Result<String, Error>| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();
		// Format again to ensure idempotency
		let result2 = formatter.format(&result.content).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, result2.content);
		assert!(result2.contains_page_macro);
	}

	#[rstest]
	fn test_format_macro_calls() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| {
			div { { format!("Hello {}", name) } }
		})"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("format!"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_function_calls() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| {
			div { { get_message() } }
		})"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("get_message()"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_method_calls() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| {
			div { { user.get_name() } }
		})"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("user.get_name()"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_format_complex_event_handler() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| {
			button {
				@click: |event| {
					prevent_default(event);
					handle_click();
				},
				"Click Me"
			}
		})"#;

		// Act
		let result = formatter.format(input).unwrap();
		// Format should be idempotent
		let result2 = formatter.format(&result.content).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.content.contains("button"));
		assert!(result.content.contains("@click"));
		assert!(result.content.contains("|event|"));
		assert!(result.content.contains("prevent_default(event)"));
		assert!(result.content.contains("handle_click()"));
		assert!(result.content.contains("\"Click Me\""));
		assert_eq!(result.content, result2.content);
		assert!(result2.contains_page_macro);
	}

	#[rstest]
	fn test_format_function_macro_calls_idempotent() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| {
			div {
				{ format!("Count: {}", count) }
				{ get_user().name() }
				{ vec![1, 2, 3].len() }
			}
		})"#;

		// Act
		let result = formatter.format(input).unwrap();
		let result2 = formatter.format(&result.content).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, result2.content);
		assert!(result2.contains_page_macro);
	}

	// ==================== Ignore Marker Tests ====================

	#[rstest]
	fn test_ignore_all_at_file_start() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-all

page!(|| {
div{badly}
})"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert_eq!(input, result.content);
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_all_after_module_doc() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"//! Module documentation
// reinhardt-fmt: ignore-all

page!(|| {
div{badly}
})"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert_eq!(input, result.content);
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_all_not_at_start() {
		// When ignore-all marker appears AFTER code lines, it should NOT be recognized
		// because the marker must appear BEFORE any code line (as documented).

		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"use foo;

// reinhardt-fmt: ignore-all

page!(|| {
div{badly}
})"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.contains_page_macro);
		assert!(result.content.contains("div {"));
		assert!(result.content.contains("badly"));
	}

	#[rstest]
	fn test_ignore_range_basic() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-on
page!(|| {
div{badly}
})
// reinhardt-fmt: ignore-off

page!(|| { div { "formatted" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{badly}"));
		assert!(result.content.contains("div {"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_range_nested_warning() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-on
page!(|| { div { "first" } })
// reinhardt-fmt: ignore-on
page!(|| { div { "second" } })
// reinhardt-fmt: ignore-off
page!(|| { div { "third" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("first"));
		assert!(result.content.contains("second"));
		assert!(result.content.contains("third"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_range_unmatched_on() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-on
page!(|| { div { "first" } })
page!(|| { div { "second" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("first"));
		assert!(result.content.contains("second"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_range_unclosed() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { "before" } })
// reinhardt-fmt: ignore-on
page!(|| { div{badly} })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{badly}"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_multiple_ignore_ranges() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { "formatted1" } })
// reinhardt-fmt: ignore-on
page!(|| { div{ignored1} })
// reinhardt-fmt: ignore-off
page!(|| { div { "formatted2" } })
// reinhardt-fmt: ignore-on
page!(|| { div{ignored2} })
// reinhardt-fmt: ignore-off
page!(|| { div { "formatted3" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{ignored1}"));
		assert!(result.content.contains("div{ignored2}"));
		assert!(result.content.contains("div {"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_individual_ignore_basic() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { "formatted" } })

// reinhardt-fmt: ignore
page!(|| { div{ignored} })

page!(|| { div { "formatted" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{ignored}"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_individual_ignore_with_blank_line() {
		// When there's a blank line between the ignore marker and the macro,
		// the marker should NOT be recognized (as documented: marker must be on
		// the line immediately before the macro, with no blank lines).

		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore

page!(|| { div{ignored} })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.contains_page_macro);
		assert!(result.content.contains("div {"));
		assert!(result.content.contains("ignored"));
	}

	#[rstest]
	fn test_individual_ignore_multiple() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore
page!(|| { div{ignored1} })

page!(|| { div { "formatted" } })

// reinhardt-fmt: ignore
page!(|| { div{ignored2} })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{ignored1}"));
		assert!(result.content.contains("div{ignored2}"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_individual_ignore_mixed_with_format() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { "formatted1" } })

// reinhardt-fmt: ignore
page!(|| { div{ignored} })

page!(|| { div { "formatted2" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{ignored}"));
		assert!(result.content.contains("div {"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_individual_ignore_with_range() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-on
page!(|| { div{range_ignored} })
// reinhardt-fmt: ignore-off

// reinhardt-fmt: ignore
page!(|| { div{individual_ignored} })

page!(|| { div { "formatted" } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{range_ignored}"));
		assert!(result.content.contains("div{individual_ignored}"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_individual_ignore_priority() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-on
// reinhardt-fmt: ignore
page!(|| { div{ignored} })
// reinhardt-fmt: ignore-off"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{ignored}"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_individual_ignore_at_file_start() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore
page!(|| { div{ignored} })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.content.contains("div{ignored}"));
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_contains_page_macro_field_with_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_contains_page_macro_field_without_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"fn main() { println!("test"); }"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert!(!result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_all_with_page_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-all
page!(|| { div { bad } })"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert_eq!(result.content, input);
		assert!(result.contains_page_macro);
	}

	#[rstest]
	fn test_ignore_all_without_page_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"// reinhardt-fmt: ignore-all
fn main() {}"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert_eq!(result.content, input);
		assert!(!result.contains_page_macro);
	}

	// ==================== Protect/Restore Tests ====================

	#[rstest]
	fn test_protect_no_page_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "fn main() { println!(\"hello\"); }";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.protected_content, input);
		assert!(result.backups.is_empty());
	}

	#[rstest]
	fn test_protect_single_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"let view = page!(|| { div { "hello" } });"#;

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(
			result.protected_content,
			r#"let view = __reinhardt_placeholder_0__!();"#
		);
		assert_eq!(result.backups.len(), 1);
		assert_eq!(result.backups[0].id, 0);
		assert!(result.backups[0].original.starts_with("page!("));
	}

	#[rstest]
	fn test_protect_multiple_macros() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"
let view1 = page!(|| { div { "first" } });
let view2 = page!(|| { div { "second" } });
"#;

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(
			result.protected_content,
			"\nlet view1 = __reinhardt_placeholder_0__!();\nlet view2 = __reinhardt_placeholder_1__!();\n"
		);
		assert_eq!(result.backups.len(), 2);
	}

	#[rstest]
	fn test_protect_preserves_surrounding_code() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"use foo::bar;

fn render() -> View {
    page!(|| { div { "hello" } })
}

fn main() {}"#;

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(
			result.protected_content,
			r#"use foo::bar;

fn render() -> View {
    __reinhardt_placeholder_0__!()
}

fn main() {}"#
		);
	}

	#[rstest]
	fn test_restore_single_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let original = r#"let view = page!(|| { div { "hello" } });"#;

		// Act
		let protected = formatter.protect_page_macros(original);
		let restored =
			AstPageFormatter::restore_page_macros(&protected.protected_content, &protected.backups);

		// Assert
		assert_eq!(restored, original);
	}

	#[rstest]
	fn test_restore_multiple_macros() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let original = r#"
let view1 = page!(|| { div { "first" } });
let view2 = page!(|| { div { "second" } });
"#;

		// Act
		let protected = formatter.protect_page_macros(original);
		let restored =
			AstPageFormatter::restore_page_macros(&protected.protected_content, &protected.backups);

		// Assert
		assert_eq!(restored, original);
	}

	#[rstest]
	fn test_protect_restore_roundtrip_complex() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let original = r#"use reinhardt::pages::page;

fn header() -> View {
    page!(|| {
        div {
            class: "header",
            h1 { "Title" }
        }
    })
}

fn footer() -> View {
    page!(|year: i32| {
        div {
            class: "footer",
            { format!("Copyright {}", year) }
        }
    })
}

fn main() {
    let _h = header();
    let _f = footer();
}"#;

		// Act
		let protected = formatter.protect_page_macros(original);
		let restored =
			AstPageFormatter::restore_page_macros(&protected.protected_content, &protected.backups);

		// Assert
		assert_eq!(protected.backups.len(), 2);
		assert!(!protected.protected_content.contains("page!("));
		assert_eq!(restored, original);
	}

	#[rstest]
	fn test_protect_empty_backups_restore() {
		// Arrange
		let content = "fn main() {}";
		let backups: Vec<PageMacroBackup> = Vec::new();

		// Act
		let restored = AstPageFormatter::restore_page_macros(content, &backups);

		// Assert
		assert_eq!(restored, content);
	}

	#[rstest]
	fn test_protect_with_trailing_call() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"let view = page!(|props: Props| { div { } })(props);"#;

		// Act
		let result = formatter.protect_page_macros(input);
		let restored =
			AstPageFormatter::restore_page_macros(&result.protected_content, &result.backups);

		// Assert
		assert_eq!(
			result.protected_content,
			r#"let view = __reinhardt_placeholder_0__!()(props);"#
		);
		assert_eq!(result.backups.len(), 1);
		assert_eq!(restored, input);
	}

	// Regression: a `page!(...)`-shaped substring inside a preceding
	// `//` comment with the same token sequence as the real macro must
	// not be mistaken for the real invocation. Previously the source
	// scanner would lock onto the comment substring and leave the real
	// macro unprotected.
	#[rstest]
	fn test_protect_skips_lookalike_in_line_comment() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "// page!(|| { div { \"hi\" } })\nlet view = page!(|| { div { \"hi\" } });";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.backups.len(), 1);
		assert_eq!(
			result.protected_content,
			"// page!(|| { div { \"hi\" } })\nlet view = __reinhardt_placeholder_0__!();"
		);
	}

	// Regression: same as the line-comment case but with a block comment
	// preceding the real macro.
	#[rstest]
	fn test_protect_skips_lookalike_in_block_comment() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "/* page!(|| { div { \"hi\" } }) */\nlet view = page!(|| { div { \"hi\" } });";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.backups.len(), 1);
		assert_eq!(
			result.protected_content,
			"/* page!(|| { div { \"hi\" } }) */\nlet view = __reinhardt_placeholder_0__!();"
		);
	}

	// Regression: a `page!(...)` literal inside a regular string literal
	// must not be picked up as a macro invocation, even when those tokens
	// happen to match the real macro byte-for-byte.
	#[rstest]
	fn test_protect_skips_lookalike_in_string_literal() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "let s = \"page!(|| { div { \\\"hi\\\" } })\";\nlet view = page!(|| { div { \"hi\" } });";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.backups.len(), 1);
		// The string literal must be left intact and only the real
		// macro on the second line should be replaced with a placeholder.
		assert!(
			result
				.protected_content
				.contains("\"page!(|| { div { \\\"hi\\\" } })\"")
		);
		assert!(
			result
				.protected_content
				.contains("__reinhardt_placeholder_0__!()")
		);
	}

	// Regression: same as the string-literal case but using a raw string
	// (`r#"..."#`), which the scanner must traverse without descending
	// into its body.
	#[rstest]
	fn test_protect_skips_lookalike_in_raw_string() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "let s = r#\"page!(|| { div { \"hi\" } })\"#;\nlet view = page!(|| { div { \"hi\" } });";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.backups.len(), 1);
		assert!(
			result
				.protected_content
				.contains("r#\"page!(|| { div { \"hi\" } })\"#")
		);
		assert!(
			result
				.protected_content
				.contains("__reinhardt_placeholder_0__!()")
		);
	}

	// Regression: same as the raw string test but using raw byte strings
	// (`br#"..."#`), which the scanner must also traverse without descending
	// into the body.
	#[rstest]
	fn test_protect_skips_lookalike_in_raw_byte_string() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "let s = br#\"page!(|| { div { \"hi\" } })\"#;\nlet view = page!(|| { div { \"hi\" } });";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.backups.len(), 1);
		assert_eq!(
			result
				.protected_content
				.matches("br#\"page!(|| { div { \"hi\" } })\"#")
				.count(),
			1
		);
		assert_eq!(
			result
				.protected_content
				.matches("__reinhardt_placeholder_0__!()")
				.count(),
			1
		);
	}

	// Regression: raw byte string with multiple hashes (`br##"..."##`).
	#[rstest]
	fn test_protect_skips_lookalike_in_raw_byte_string_multi_hash() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = "let s = br##\"page!(|| { div { \"#hi#\" } })\"##;\nlet view = page!(|| { div { \"hi\" } });";

		// Act
		let result = formatter.protect_page_macros(input);

		// Assert
		assert_eq!(result.backups.len(), 1);
		assert_eq!(
			result
				.protected_content
				.matches("br##\"page!(|| { div { \"#hi#\" } })\"##")
				.count(),
			1
		);
		assert_eq!(
			result
				.protected_content
				.matches("__reinhardt_placeholder_0__!()")
				.count(),
			1
		);
	}

	// ==================== Unicode Character Tests ====================

	#[rstest]
	fn test_find_matching_paren_with_emoji() {
		// Arrange
		let source = r#"(div { "😀" })"#;

		// Act
		let result = find_matching_paren(source, 1);

		// Assert
		assert_eq!(result, Some(source.len() - 1));
	}

	#[rstest]
	fn test_find_matching_paren_with_cjk() {
		// Arrange
		let source = r#"(div { "日本語" })"#;

		// Act
		let result = find_matching_paren(source, 1);

		// Assert
		assert_eq!(result, Some(source.len() - 1));
	}

	#[rstest]
	fn test_find_matching_paren_nested_with_unicode() {
		// Arrange
		let source = r#"(outer { (inner { "안녕" }) })"#;

		// Act
		let result = find_matching_paren(source, 1);

		// Assert
		assert_eq!(result, Some(source.len() - 1));
	}

	#[rstest]
	fn test_find_matching_paren_mixed() {
		// Arrange
		let source = r#"(div { "Hello 世界 مرحبا" })"#;

		// Act
		let result = find_matching_paren(source, 1);

		// Assert
		assert_eq!(result, Some(source.len() - 1));
	}

	#[rstest]
	fn test_is_in_comment_or_string_unicode_in_string() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let content = r#"let s = "😀🎉日本語";"#;
		let pos_in_string = content.find("日").unwrap();

		// Act & Assert
		assert!(formatter.is_in_comment_or_string(content, pos_in_string));
	}

	#[rstest]
	fn test_is_in_comment_or_string_unicode_in_comment() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let content = r#"// This is a comment with 日本語"#;
		let pos_in_comment = content.find("日").unwrap();

		// Act & Assert
		assert!(formatter.is_in_comment_or_string(content, pos_in_comment));
	}

	#[rstest]
	fn test_protect_restore_with_unicode_content() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let original = r#"let view = page!(|| { div { "😀🎉日本語" } });"#;

		// Act
		let protected = formatter.protect_page_macros(original);
		let restored =
			AstPageFormatter::restore_page_macros(&protected.protected_content, &protected.backups);

		// Assert
		assert_eq!(protected.backups.len(), 1);
		assert_eq!(
			protected.protected_content,
			r#"let view = __reinhardt_placeholder_0__!();"#
		);
		assert_eq!(restored, original);
	}

	// ========================================
	// Tests for expression formatting with rustfmt
	// ========================================

	// Short expressions: stay on a single line

	#[rstest]
	fn test_format_expression_short_braced() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter.format(r#"page!(|| { { some_value } })"#).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ some_value }\n})");
	}

	#[rstest]
	fn test_format_expression_short_unbraced() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter.format(r#"page!(|| { some_value })"#).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\tsome_value\n})");
	}

	#[rstest]
	fn test_format_expression_short_method_call() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter
			.format(r#"page!(|| { { items.len() } })"#)
			.unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ items.len() }\n})");
	}

	#[rstest]
	fn test_format_expression_short_string_literal() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter
			.format(r#"page!(|| { { "hello world" } })"#)
			.unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ \"hello world\" }\n})");
	}

	#[rstest]
	fn test_format_expression_empty_braced() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter.format(r#"page!(|| { { () } })"#).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ () }\n})");
	}

	#[rstest]
	fn test_format_expression_numeric_literal() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter.format(r#"page!(|| { { 42 } })"#).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ 42 }\n})");
	}

	#[rstest]
	fn test_format_expression_boolean() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter.format(r#"page!(|| { { true } })"#).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ true }\n})");
	}

	#[rstest]
	fn test_format_expression_binary_op() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter.format(r#"page!(|| { { x + y } })"#).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(result.content, "page!(|| {\n\t{ x + y }\n})");
	}

	#[rstest]
	fn test_format_expression_with_closure_under_threshold() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter
			.format(
				r#"page!(|| { { items.iter().map(|item| item.render()).collect::<Vec<_>>() } })"#,
			)
			.unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\t{ items.iter().map(|item| item.render()).collect::<Vec<_>>() }\n})"
		);
	}

	#[rstest]
	fn test_format_expression_with_if_condition() {
		// Arrange
		let formatter = AstPageFormatter::new();

		// Act
		let result = formatter
			.format("page!(|| {\n\tif condition {\n\t\t{ short_val }\n\t}\n})")
			.unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\tif condition {\n\t\t{ short_val }\n\t}\n})"
		);
	}

	#[rstest]
	fn test_format_expression_exactly_at_threshold() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let expr = "a".repeat(90);
		let input = format!("page!(|| {{ {{ {} }} }})", expr);

		// Act
		let result = formatter.format(&input);

		// Assert
		assert!(result.is_ok());
	}

	// Long expressions: multiline formatting via rustfmt

	#[rstest]
	fn test_format_expression_long_view_fragment() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = format!(
			"page!(|| {{ {{ {} }} }})",
			"View::fragment(signal.result().unwrap_or_default().iter().map(|item| View::text(item.clone())).collect::<Vec<_>>())"
		);

		// Act
		let result = formatter.format(&input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\t{\n\t\tView::fragment(\n\t\t\t\tsignal\n\t\t\t\t\t.result()\n\t\t\t\t\t.unwrap_or_default()\n\t\t\t\t\t.iter()\n\t\t\t\t\t.map(|item| View::text(item.clone()))\n\t\t\t\t\t.collect::<Vec<_>>(),\n\t\t\t)\n\t}\n})"
		);
	}

	#[rstest]
	fn test_format_expression_long_chained_methods() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = format!(
			"page!(|| {{ {{ {} }} }})",
			r#"data.iter().filter(|x| x.is_active()).map(|x| x.name.clone()).collect::<Vec<String>>().join(", ")"#
		);

		// Act
		let result = formatter.format(&input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\t{\n\t\tdata\n\t\t\t\t.iter()\n\t\t\t\t.filter(|x| x.is_active())\n\t\t\t\t.map(|x| x.name.clone())\n\t\t\t\t.collect::<Vec<String>>()\n\t\t\t\t.join(\", \")\n\t}\n})"
		);
	}

	#[rstest]
	fn test_format_expression_long_nested_function_calls() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = format!(
			"page!(|| {{ {{ {} }} }})",
			r#"format!("User: {} ({})", user.display_name().unwrap_or_default(), user.email().unwrap_or("no email".to_string()))"#
		);

		// Act
		let result = formatter.format(&input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\t{\n\t\tformat!(\n\t\t\t\t\"User: {} ({})\",\n\t\t\t\tuser.display_name().unwrap_or_default(),\n\t\t\t\tuser.email().unwrap_or(\"no email\".to_string())\n\t\t\t)\n\t}\n})"
		);
	}

	#[rstest]
	fn test_format_expression_deeply_nested_in_elements() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = format!(
			r#"page!(|| {{ div {{ span {{ {{ {} }} }} }} }})"#,
			"View::fragment(signal.result().unwrap_or_default().iter().map(|item| View::text(item.clone())).collect::<Vec<_>>())"
		);

		// Act
		let result = formatter.format(&input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\tdiv {\n\t\tspan {\n\t\t\t{\n\t\t\t\tView::fragment(\n\t\t\t\t\t\tsignal\n\t\t\t\t\t\t\t.result()\n\t\t\t\t\t\t\t.unwrap_or_default()\n\t\t\t\t\t\t\t.iter()\n\t\t\t\t\t\t\t.map(|item| View::text(item.clone()))\n\t\t\t\t\t\t\t.collect::<Vec<_>>(),\n\t\t\t\t\t)\n\t\t\t}\n\t\t}\n\t}\n})"
		);
	}

	#[rstest]
	fn test_format_expression_multiple_in_page() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = format!(
			"page!(|| {{ {{ {} }} {{ {} }} }})",
			"count",
			"View::fragment(signal.result().unwrap_or_default().iter().map(|item| View::text(item.clone())).collect::<Vec<_>>())"
		);

		// Act
		let result = formatter.format(&input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|| {\n\t{ count }\n\t{\n\t\tView::fragment(\n\t\t\t\tsignal\n\t\t\t\t\t.result()\n\t\t\t\t\t.unwrap_or_default()\n\t\t\t\t\t.iter()\n\t\t\t\t\t.map(|item| View::text(item.clone()))\n\t\t\t\t\t.collect::<Vec<_>>(),\n\t\t\t)\n\t}\n})"
		);
	}

	// Complex DSL formatting tests

	#[rstest]
	fn test_format_complex_dsl_nested_page_macro() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|signal: Action<Vec<Item>, String>| {
	div {
		{ View::fragment(signal.result().unwrap_or_default().iter().map(|item| { let text = item.text.clone(); page!(|text: String| { span { { text } } })(text) }).collect::<Vec<_>>()) }
	}
})(signal)"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|signal: Action<Vec<Item>, String>| {\n\tdiv {\n\t\t{\n\t\t\tView::fragment(\n\t\t\t\t\tsignal\n\t\t\t\t\t\t.result()\n\t\t\t\t\t\t.unwrap_or_default()\n\t\t\t\t\t\t.iter()\n\t\t\t\t\t\t.map(|item| {\n\t\t\t\t\t\t\tlet text = item.text.clone();\n\t\t\t\t\t\t\tpage!(|text: String| {\n\t\t\t\t\t\t\t\tspan {\n\t\t\t\t\t\t\t\t\t{ text }\n\t\t\t\t\t\t\t\t}\n\t\t\t\t\t\t\t})(text)\n\t\t\t\t\t\t})\n\t\t\t\t\t\t.collect::<Vec<_>>(),\n\t\t\t\t)\n\t\t}\n\t}\n})(signal)"
		);
	}

	#[rstest]
	fn test_format_complex_dsl_conditional() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|signal: Action<Vec<Item>, String>| {
	div {
		if signal.result().is_some() {
			{ View::fragment(signal.result().unwrap_or_default().iter().map(|item| { let text = item.text.clone(); page!(|text: String| { div class="item" { { text } } })(text) }).collect::<Vec<_>>()) }
		} else {
			p { "Loading..." }
		}
	}
})(signal)"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|signal: Action<Vec<Item>, String>| {\n\tdiv {\n\t\tif signal.result().is_some() {\n\t\t\t{\n\t\t\t\tView::fragment(\n\t\t\t\t\t\tsignal\n\t\t\t\t\t\t\t.result()\n\t\t\t\t\t\t\t.unwrap_or_default()\n\t\t\t\t\t\t\t.iter()\n\t\t\t\t\t\t\t.map(|item| {\n\t\t\t\t\t\t\t\tlet text = item.text.clone();\n\t\t\t\t\t\t\t\tpage!(|text: String| {\n\t\t\t\t\t\t\t\t\tdiv\n\t\t\t\t\t\t\t\t\tclass = \"item\"\n\t\t\t\t\t\t\t\t\t{ { text } }\n\t\t\t\t\t\t\t\t})(text)\n\t\t\t\t\t\t\t})\n\t\t\t\t\t\t\t.collect::<Vec<_>>(),\n\t\t\t\t\t)\n\t\t\t}\n\t\t} else {\n\t\t\tp {\n\t\t\t\t\"Loading...\"\n\t\t\t}\n\t\t}\n\t}\n})(signal)"
		);
	}

	#[rstest]
	fn test_format_complex_dsl_for_loop() {
		// Arrange
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|items: Vec<Item>| {
	div class="list" {
		for item in items {
			div class="card" {
				{ View::fragment(item.tags.iter().map(|tag| { let t = tag.clone(); page!(|t: String| { span class="tag" { { t } } })(t) }).collect::<Vec<_>>()) }
			}
		}
	}
})(items)"#;

		// Act
		let result = formatter.format(input).unwrap();

		// Assert
		assert!(result.skipped.is_none(), "formatting should not be skipped");
		assert_eq!(
			result.content,
			"page!(|items: Vec<Item>| {\n\tdiv class=\"list\" {\n\t\tfor item in items {\n\t\t\tdiv class=\"card\" {\n\t\t\t\t{ View::fragment(item.tags.iter().map(|tag| { let t = tag.clone(); page!(|t: String| { span class=\"tag\" { { t } } })(t) }).collect::<Vec<_>>()) }\n\t\t\t}\n\t\t}\n\t}\n})(items)"
		);
	}
}
