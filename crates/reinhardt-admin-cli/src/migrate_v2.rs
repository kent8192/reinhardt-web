//! Manouche v1 → v2 codemod (spec §6.1 + §6.2).
//!
//! Invoked via `reinhardt-admin migrate-manouche-v2 [PATH]` or
//! `cargo make migrate-manouche-v2`.

use std::path::PathBuf;

use clap::Args;

pub mod rewriter;
pub mod rules;
pub mod walker;

/// Arguments for the `migrate-manouche-v2` subcommand.
#[derive(Args, Debug)]
pub struct MigrateV2Args {
	/// Root path to migrate. Defaults to the current workspace.
	#[arg(default_value = ".")]
	pub path: PathBuf,

	/// Print changes without writing them.
	#[arg(long)]
	pub dry_run: bool,

	/// Comma-separated list of rule names to skip (e.g. `--skip use_effect_deps`).
	#[arg(long, value_delimiter = ',')]
	pub skip: Vec<String>,
}

/// Entry point invoked by `main.rs`.
///
/// File paths are obtained from `walker::find_rs_files`, which enumerates
/// entries rooted at the CLI-supplied `--path` directory via `walkdir`. No
/// remote/HTTP input is involved; this is a developer-run codemod that
/// rewrites files in the developer's own checkout. Semgrep's Actix
/// "path-traversal" pattern flags any `std::fs` call whose path argument is
/// not a string literal, but the only "untrusted" surface here is the
/// developer's own CLI invocation, which is an intentional capability.
pub fn run(args: MigrateV2Args) -> anyhow::Result<()> {
	let all_rules = rules::all();
	let known_rule_names: std::collections::BTreeSet<&'static str> =
		all_rules.iter().map(|r| r.name()).collect();
	let unknown: Vec<&str> = args
		.skip
		.iter()
		.map(String::as_str)
		.filter(|name| !known_rule_names.contains(name))
		.collect();
	if !unknown.is_empty() {
		anyhow::bail!("unknown --skip rule(s): {}", unknown.join(", "));
	}
	let rules: Vec<_> = all_rules
		.into_iter()
		.filter(|r| !args.skip.iter().any(|s| s == r.name()))
		.collect();

	let files = walker::find_rs_files(&args.path)?;
	let mut changed = 0_usize;

	for path in files {
		let src = read_developer_file(&path)?;
		let parsed: syn::File = match syn::parse_file(&src) {
			Ok(f) => f,
			// Skip files we cannot parse (e.g. build scripts with cfg-gated items).
			Err(_) => continue,
		};
		let mut out_ast = parsed.clone();
		for r in &rules {
			out_ast = r.rewrite(out_ast);
		}

		let out = apply_changes_preserving_formatting(&src, &parsed, &out_ast);
		if out != src {
			changed += 1;
			if args.dry_run {
				println!("would rewrite: {}", path.display());
			} else {
				write_developer_file(&path, &out)?;
				println!("rewrote: {}", path.display());
			}
		}
	}

	println!(
		"\nDone. {} file(s) {}.",
		changed,
		if args.dry_run {
			"would change"
		} else {
			"changed"
		}
	);
	Ok(())
}

/// Compares original and transformed ASTs at the item level, replacing only
/// the text spans of changed items. Comments, blank lines, and formatting in
/// unchanged items are preserved from the original source.
///
/// Item boundaries are located by searching for each item's prettyprinted text
/// in the source. `proc_macro2::Span` does not provide real positions outside
/// of proc-macro context, so a text-based approach is used instead.
fn apply_changes_preserving_formatting(
	src: &str,
	parsed: &syn::File,
	out_ast: &syn::File,
) -> String {
	if parsed.items.len() != out_ast.items.len() {
		return prettyplease::unparse(out_ast);
	}

	let mut result = String::with_capacity(src.len() + 1024);
	let mut last_pos: usize = 0;

	let item_count = parsed.items.len();
	for i in 0..item_count {
		let orig_item = &parsed.items[i];
		let new_item = &out_ast.items[i];

		let formatted_orig = format_single_item(orig_item);
		let formatted_new = format_single_item(new_item);

		let (start_byte, end_byte) = find_item_in_source(src, last_pos, &formatted_orig);
		if start_byte < last_pos || start_byte >= end_byte || end_byte > src.len() {
			return prettyplease::unparse(out_ast);
		}

		// Copy source between the previous item and this one (comments, blank lines).
		result.push_str(&src[last_pos..start_byte]);

		if formatted_orig == formatted_new {
			// Item unchanged — keep original source text.
			result.push_str(&src[start_byte..end_byte]);
		} else {
			// Item changed — format the new item via prettyplease.
			result.push_str(&format_single_item(new_item));
		}

		last_pos = end_byte;
	}

	// Copy trailing source after the last item (trailing comments, whitespace).
	if last_pos < src.len() {
		result.push_str(&src[last_pos..]);
	}

	result
}

/// Format a single `syn::Item` using `prettyplease`.
///
/// Wraps the item in a temporary `syn::File` so `prettyplease::unparse` can
/// format it. Trailing whitespace added by prettyplease is trimmed.
fn format_single_item(item: &syn::Item) -> String {
	let file = syn::File {
		shebang: None,
		attrs: vec![],
		items: vec![item.clone()],
	};
	let formatted = prettyplease::unparse(&file);
	formatted.trim_end().to_string()
}

/// Find an item's byte range in the source by searching for its normalized
/// token text. Returns `(search_from, search_from)` when the item cannot be
/// located so the caller can fall back to full-file formatting.
fn find_item_in_source(src: &str, search_from: usize, item_tokens: &str) -> (usize, usize) {
	let anchor = item_tokens
		.lines()
		.find(|l| {
			let trimmed = l.trim();
			!trimmed.is_empty()
				&& !trimmed.starts_with("//")
				&& !trimmed.starts_with("#[")
				&& !trimmed.starts_with("///")
		})
		.unwrap_or("");

	if anchor.is_empty() {
		return (search_from, search_from);
	}

	let rest = &src[search_from..];
	let start = match rest.find(anchor) {
		Some(pos) => search_from + pos,
		None => return (search_from, search_from),
	};
	let start = find_item_start_with_prefix(src, search_from, start);

	let after_start = &src[start..];
	let end_offset = find_item_end_offset(after_start);
	(start, start + end_offset)
}

fn find_item_start_with_prefix(src: &str, search_from: usize, item_start: usize) -> usize {
	let mut start = line_start(src, item_start);
	while start > search_from {
		let previous_end = start.saturating_sub(1);
		let previous_start = line_start(src, previous_end);
		let line = &src[previous_start..start];
		let trimmed = line.trim();
		if trimmed.starts_with("#[") || trimmed.starts_with("///") {
			start = previous_start;
		} else {
			break;
		}
	}
	start
}

fn line_start(src: &str, pos: usize) -> usize {
	src[..pos].rfind('\n').map_or(0, |idx| idx + 1)
}

/// Find the byte offset past the end of an item starting at `src[0]`.
/// Handles block-delimited items (tracking `{}` nesting) and
/// semicolon-terminated items. Skips string/char literals, comments,
/// and nested block constructs so that braces/semicolons inside them
/// do not corrupt the boundary detection.
fn find_item_end_offset(src: &str) -> usize {
	let mut brace_depth: i32 = 0;
	let mut has_block = false;
	let bytes = src.as_bytes();
	let len = bytes.len();
	let mut i = 0;

	while i < len {
		let ch = bytes[i];

		// Skip line comments
		if ch == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
			while i < len && bytes[i] != b'\n' {
				i += 1;
			}
			continue;
		}

		// Skip block comments
		if ch == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
			i += 2;
			while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
				i += 1;
			}
			if i + 1 < len {
				i += 2; // skip "*/"
			}
			continue;
		}

		// Skip raw string literals: r"..." r#"..."# etc.
		if ch == b'r' && i + 1 < len {
			let next = bytes[i + 1];
			if next == b'"' || next == b'#' {
				let hash_count = if next == b'"' {
					0
				} else {
					let mut count = 0;
					let mut j = i + 1;
					while j < len && bytes[j] == b'#' {
						count += 1;
						j += 1;
					}
					if j < len && bytes[j] == b'"' {
						i = j; // position at the opening quote
						count
					} else {
						i += 1;
						continue;
					}
				};
				i += 1; // skip opening quote
				while i < len {
					if bytes[i] == b'"' {
						// Check if followed by the right number of hashes
						let mut h = 0;
						let mut j = i + 1;
						while j < len && bytes[j] == b'#' && h < hash_count {
							h += 1;
							j += 1;
						}
						if h == hash_count {
							i = j;
							break;
						}
					}
					if bytes[i] == b'\\' && i + 1 < len {
						i += 2; // skip escaped char
					} else {
						i += 1;
					}
				}
				continue;
			}
		}

		// Skip regular string literals
		if ch == b'"' {
			i += 1;
			while i < len {
				if bytes[i] == b'"' {
					i += 1;
					break;
				}
				if bytes[i] == b'\\' && i + 1 < len {
					i += 2; // skip escaped char
				} else {
					i += 1;
				}
			}
			continue;
		}

		// Skip byte literals: b'x'
		if ch == b'b' && i + 1 < len && bytes[i + 1] == b'\'' {
			i += 2; // skip "b'"
			while i < len {
				if bytes[i] == b'\'' {
					i += 1;
					break;
				}
				if bytes[i] == b'\\' && i + 1 < len {
					i += 2;
				} else {
					i += 1;
				}
			}
			continue;
		}

		// Skip char literals ('x') vs lifetimes ('ident)
		if ch == b'\'' {
			i += 1; // skip opening quote
			if i < len {
				// Lifetime: ' followed by letter or underscore (e.g. 'a, 'static)
				if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
					while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
						i += 1;
					}
				} else {
					// Char literal: skip the character (or escape sequence)
					if bytes[i] == b'\\' && i + 1 < len {
						i += 2;
					} else {
						i += 1;
					}
					// Skip closing quote
					if i < len && bytes[i] == b'\'' {
						i += 1;
					}
				}
			}
			continue;
		}

		match ch {
			b'{' => {
				brace_depth += 1;
				has_block = true;
			}
			b'}' => {
				brace_depth -= 1;
				if brace_depth == 0 && has_block {
					return i + 1;
				}
			}
			b';' if brace_depth == 0 => {
				return i + 1;
			}
			_ => {}
		}
		i += 1;
	}

	src.len()
}

/// Reads a developer-owned source file enumerated by `walker::find_rs_files`.
///
/// The path argument is bounded by the CLI-supplied `--path` root; this is a
/// developer-run codemod, not a network-facing service. We canonicalize the
/// path before any IO to make the bounds explicit.
fn read_developer_file(path: &std::path::Path) -> anyhow::Result<String> {
	let canonical = path.canonicalize()?;
	let mut file = std::fs::File::open(canonical)?; // nosemgrep: path-traversal false positive — developer CLI bounded by --path root
	let mut buf = String::new();
	std::io::Read::read_to_string(&mut file, &mut buf)?;
	Ok(buf)
}

/// Writes the rewritten source back to a developer-owned file. Same scope
/// note as `read_developer_file`.
///
/// Uses a unique temp file in the target's parent directory so that the
/// final `rename` is atomic (same filesystem). Process ID and a random
/// suffix prevent collisions under concurrent invocations.
fn write_developer_file(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
	let canonical = path.canonicalize()?;
	let parent = canonical
		.parent()
		.ok_or_else(|| anyhow::anyhow!("no parent directory for {}", canonical.display()))?;
	let file_name = canonical
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("rewrite");
	let random_suffix: u32 = {
		use std::time::{SystemTime, UNIX_EPOCH};
		let nanos = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_default()
			.subsec_nanos();
		nanos ^ std::process::id()
	};
	let tmp = parent.join(format!(".{file_name}.{random_suffix:x}.tmp")); // nosemgrep: path-traversal false positive — developer CLI bounded by --path root
	if let Err(e) = std::fs::write(&tmp, content) {
		let _ = std::fs::remove_file(&tmp);
		return Err(e.into());
	}
	if let Err(e) = std::fs::rename(&tmp, canonical) {
		let _ = std::fs::remove_file(&tmp);
		return Err(e.into());
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	/// Verify that when no AST items change, the output is identical to input.
	#[rstest]
	fn no_changes_output_identical() {
		// Arrange
		let src = "//! Module doc comment.\n\nuse std::collections::HashMap;\n\n/// A struct.\npub struct Foo {\n    x: i32,\n}\n";
		let parsed: syn::File = syn::parse_file(src).unwrap();
		let out_ast = parsed.clone();

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert
		assert_eq!(result, src);
	}

	/// Comments between items are preserved even when other items change.
	#[rstest]
	fn comments_between_items_preserved() {
		// Arrange
		let src = "//! Module doc.\n\n// Comment before struct\npub struct Foo {\n    x: i32,\n}\n\n// Comment between items\npub struct Bar {\n    y: String,\n}\n";
		let parsed: syn::File = syn::parse_file(src).unwrap();
		let mut out_ast = parsed.clone();

		// Simulate a change to the first item only (rename Foo to Foo2).
		if let syn::Item::Struct(s) = &mut out_ast.items[0] {
			s.ident = syn::Ident::new("Foo2", s.ident.span());
		}

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert: changed item reflects new name, comments preserved.
		assert!(
			result.contains("pub struct Foo2"),
			"changed item not updated"
		);
		assert!(result.contains("//! Module doc."), "module doc lost");
		assert!(
			result.contains("// Comment before struct"),
			"comment before struct lost"
		);
		assert!(
			result.contains("// Comment between items"),
			"inter-item comment lost"
		);
		assert!(result.contains("pub struct Bar"), "unchanged item lost");
	}

	/// Blank lines between items survive the codemod.
	#[rstest]
	fn blank_lines_between_items_preserved() {
		// Arrange
		let src = "use std::io;\n\n\nuse std::fs;\n\n\n\nuse std::path;\n";
		let parsed: syn::File = syn::parse_file(src).unwrap();
		let mut out_ast = parsed.clone();

		// Change the second use statement.
		if let syn::Item::Use(u) = &mut out_ast.items[1] {
			// Replace `use std::fs` with `use std::fs::File`.
			*u = syn::parse_quote!(
				use std::fs::File;
			);
		}

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert: blank lines between items preserved, only changed item replaced.
		assert!(result.contains("use std::io;"), "first use lost");
		assert!(
			result.contains("use std::fs::File;"),
			"changed use not updated"
		);
		assert!(result.contains("use std::path;"), "third use lost");
		// The blank line count should be preserved between unchanged items.
		assert!(
			result.contains("use std::io;\n\n\n"),
			"blank lines after first use altered"
		);
		assert!(
			result.contains("\n\n\nuse std::path;"),
			"blank lines before third use altered"
		);
	}

	/// Module-level `//!` doc comments are preserved.
	#[rstest]
	fn module_doc_comment_preserved() {
		// Arrange
		let src = "//! Crate-level documentation.\n//! Second line.\n\npub fn foo() {}\n";
		let parsed: syn::File = syn::parse_file(src).unwrap();
		let mut out_ast = parsed.clone();

		// Change the function.
		if let syn::Item::Fn(f) = &mut out_ast.items[0] {
			f.sig.ident = syn::Ident::new("bar", f.sig.ident.span());
		}

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert
		assert!(
			result.contains("//! Crate-level documentation."),
			"module doc lost"
		);
		assert!(result.contains("//! Second line."), "second doc line lost");
		assert!(result.contains("pub fn bar"), "renamed function missing");
	}

	/// When only 1 of multiple items changes, the other items stay untouched
	/// including their original formatting.
	#[rstest]
	fn only_changed_item_replaced() {
		// Arrange
		let src = "pub const A: i32 = 1;\npub const B: i32 = 2;\npub const C: i32 = 3;\n";
		let parsed: syn::File = syn::parse_file(src).unwrap();
		let mut out_ast = parsed.clone();

		// Change only the middle item.
		if let syn::Item::Const(c) = &mut out_ast.items[1] {
			c.ident = syn::Ident::new("B_CHANGED", c.ident.span());
		}

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert
		assert!(
			result.contains("pub const A: i32 = 1;"),
			"first item altered"
		);
		assert!(
			result.contains("pub const B_CHANGED"),
			"changed item not updated"
		);
		assert!(
			result.contains("pub const C: i32 = 3;"),
			"third item altered"
		);
		// Verify that only B was changed — A and C are verbatim from source.
		let a_idx = result.find("pub const A").unwrap();
		let b_idx = result.find("pub const B_CHANGED").unwrap();
		let c_idx = result.find("pub const C").unwrap();
		assert!(a_idx < b_idx && b_idx < c_idx, "item order changed");
	}

	#[rstest]
	fn item_count_mismatch_falls_back_to_full_unparse() {
		// Arrange
		let src = "pub fn a() {}\n";
		let parsed: syn::File = syn::parse_file(src).unwrap();
		let mut out_ast = parsed.clone();
		out_ast.items.push(syn::parse_quote!(
			pub fn b() {}
		));

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert
		assert_eq!(result, prettyplease::unparse(&out_ast));
	}

	#[rstest]
	fn missing_item_mapping_falls_back_to_full_unparse() {
		// Arrange
		let parsed_src = "pub fn original() {}\n";
		let src = "pub fn different() {}\n";
		let parsed: syn::File = syn::parse_file(parsed_src).unwrap();
		let out_ast = parsed.clone();

		// Act
		let result = apply_changes_preserving_formatting(src, &parsed, &out_ast);

		// Assert
		assert_eq!(result, prettyplease::unparse(&out_ast));
	}
}
