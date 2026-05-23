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

		let out = prettyplease::unparse(&out_ast);
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
