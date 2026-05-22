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
pub fn run(args: MigrateV2Args) -> anyhow::Result<()> {
	let _ = args;
	anyhow::bail!("not implemented yet")
}
