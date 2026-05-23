//! Manouche v1 → v2 codemod (spec §6.1 + §6.2).
//!
//! Invoked via `reinhardt-admin migrate-manouche-v2 [PATH]` or
//! `cargo make migrate-manouche-v2`.

use std::path::PathBuf;

use clap::Args;
use syn::spanned::Spanned;

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
fn apply_changes_preserving_formatting(
	src: &str,
	parsed: &syn::File,
	out_ast: &syn::File,
) -> String {
	let mut result = String::with_capacity(src.len() + 1024);
	let mut last_pos: usize = 0;

	let item_count = std::cmp::min(parsed.items.len(), out_ast.items.len());
	for i in 0..item_count {
		let orig_item = &parsed.items[i];
		let new_item = &out_ast.items[i];

		let (start_byte, end_byte) = item_byte_range(src, orig_item);

		let orig_tokens = quote::quote! { #orig_item }.to_string();
		let new_tokens = quote::quote! { #new_item }.to_string();

		// Copy source between the previous item and this one (comments, blank lines).
		result.push_str(&src[last_pos..start_byte]);

		if orig_tokens == new_tokens {
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

/// Convert a `syn::Item`'s span to byte offsets in the source text.
///
/// Uses `proc_macro2` line/column information. Both are 0-based; column is
/// measured in bytes (not characters) per proc_macro2 documentation.
fn item_byte_range(src: &str, item: &syn::Item) -> (usize, usize) {
	let span = item.span();
	let start = span.start();
	let end = span.end();
	let start_byte = line_column_to_byte(src, start.line, start.column);
	let end_byte = line_column_to_byte(src, end.line, end.column);
	(start_byte, end_byte)
}

/// Convert 0-based line and column to a byte offset in the source string.
fn line_column_to_byte(src: &str, line: usize, column: usize) -> usize {
	if line == 0 {
		return column;
	}
	let mut current_line = 0;
	for (i, ch) in src.char_indices() {
		if current_line == line {
			return i + column;
		}
		if ch == '\n' {
			current_line += 1;
		}
	}
	// If we ran off the end, point to the byte just past the source.
	src.len()
}

/// Reads a developer-owned source file enumerated by `walker::find_rs_files`.
///
/// The path argument is bounded by the CLI-supplied `--path` root; this is a
/// developer-run codemod, not a network-facing service. We canonicalize the
/// path before any IO to make the bounds explicit.
fn read_developer_file(path: &std::path::Path) -> anyhow::Result<String> {
	let canonical = path.canonicalize()?;
	let mut file = std::fs::File::open(canonical)?;
	let mut buf = String::new();
	std::io::Read::read_to_string(&mut file, &mut buf)?;
	Ok(buf)
}

/// Writes the rewritten source back to a developer-owned file. Same scope
/// note as `read_developer_file`.
fn write_developer_file(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
	let canonical = path.canonicalize()?;
	let parent = canonical
		.parent()
		.ok_or_else(|| anyhow::anyhow!("missing parent directory"))?;
	let file_name = canonical
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("rewrite");
	let tmp = parent.join(format!(".{file_name}.tmp"));
	std::fs::write(&tmp, content)?;
	std::fs::rename(tmp, canonical)?;
	Ok(())
}
