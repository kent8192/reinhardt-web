//! Explicit filesystem context for migration SQL assets.

use super::{Migration, MigrationError, Result, UpgradeResult};
use cap_std::{ambient_authority, fs::Dir};
use same_file::Handle;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Native filesystem context for one migration load or source-upgrade preflight.
///
/// SQL assets are confined to the selected root and cached by file identity for
/// this context's lifetime. Create a new context for the next load to observe
/// deployed asset changes. Only `RunSQL.sql` and `RunSQL.reverse_sql` resolve
/// literal, unqualified `include_str!` expressions; arbitrary Rust is not run.
/// API parity: P0 (native filesystem only), following the existing migration
/// filesystem boundary. This context is unavailable to browser applications.
///
/// ```no_run
/// use reinhardt_db::migrations::SqlAssetContext;
/// use std::path::Path;
/// # fn example() -> reinhardt_db::migrations::Result<()> {
/// let mut context = SqlAssetContext::new(Path::new("migrations"))?;
/// let path = Path::new("migrations/example/0001_guard.rs");
/// context.register_migration_source(path)?;
/// let source = std::fs::read_to_string(path)?;
/// let upgraded = context.upgrade_source(&source, path)?;
/// assert_eq!(upgraded.to_version, 1);
/// # Ok(())
/// # }
/// ```
pub struct SqlAssetContext {
	root: PathBuf,
	directory: Dir,
	assets: HashMap<Handle, String>,
	asset_paths: HashSet<PathBuf>,
	sources: HashSet<Handle>,
}

pub(crate) struct SqlAssetScope<'a> {
	pub(crate) context: &'a mut SqlAssetContext,
	pub(crate) source: &'a Path,
}

impl SqlAssetContext {
	/// Anchor one read scope to an existing migration directory.
	///
	/// Directory invocations use their selected directory; single-file upgrades
	/// use the containing source directory. No ancestor root is inferred.
	pub fn new(root: &Path) -> Result<Self> {
		let root = root.canonicalize()?;
		let directory = Dir::open_ambient_dir(&root, ambient_authority())?;
		Ok(Self {
			root,
			directory,
			assets: HashMap::new(),
			asset_paths: HashSet::new(),
			sources: HashSet::new(),
		})
	}

	/// Register every migration source before resolving any assets in the batch.
	///
	/// A file cannot serve as both a migration source and an SQL asset, including
	/// symlink and hard-link aliases of the same file.
	pub fn register_migration_source(&mut self, path: &Path) -> Result<()> {
		let identity = Handle::from_path(path).map_err(|error| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to register migration source {}: {error}",
				path.display()
			)))
		})?;
		if self.assets.contains_key(&identity) {
			return Err(MigrationError::InvalidMigration(format!(
				"{} is both a migration source and an SQL asset",
				path.display()
			)));
		}
		self.sources.insert(identity);
		Ok(())
	}

	/// Whether this file identity, or a file beneath this directory, was resolved.
	///
	/// Use this after batch parsing to classify discovered SQL files and read-only
	/// asset links without treating them as standalone migrations.
	pub fn is_referenced_asset(&self, path: &Path) -> Result<bool> {
		let canonical = path.canonicalize()?;
		if canonical.is_dir() {
			return Ok(self
				.asset_paths
				.iter()
				.any(|asset| asset.starts_with(&canonical)));
		}
		Ok(self.assets.contains_key(&Handle::from_path(path)?))
	}

	/// Reconstruct strict migration semantics using explicit source coordinates.
	///
	/// `source_path` stays visible rather than being canonicalized: relative
	/// includes use that path's parent, matching Rust for symlinked source files.
	/// `app_label` and `name` remain authoritative migration identity fields.
	pub fn extract_migration_metadata(
		&mut self,
		ast: &syn::File,
		source_path: &Path,
		app_label: &str,
		name: &str,
	) -> Result<Migration> {
		self.register_migration_source(source_path)?;
		super::ast_parser::extract_migration_metadata_with_assets(
			ast,
			app_label,
			name,
			&mut Some(SqlAssetScope {
				context: self,
				source: source_path,
			}),
		)
	}

	/// Upgrade source representation while preserving include expressions.
	///
	/// The same asset reads validate both sides of semantic comparison; returned
	/// source retains its original includes and no SQL asset is written.
	pub fn upgrade_source(&mut self, source: &str, source_path: &Path) -> Result<UpgradeResult> {
		self.register_migration_source(source_path)?;
		super::source_format::upgrade_source_with_assets(
			source,
			&mut Some(SqlAssetScope {
				context: self,
				source: source_path,
			}),
		)
	}

	pub(crate) fn read(&mut self, source: &Path, field: &str, include: &str) -> Result<String> {
		let failure = |reason: String| {
			MigrationError::InvalidMigration(format!(
				"{}: {field}: SQL asset {include:?}: {reason}",
				source.display()
			))
		};
		let relative = Path::new(include);
		if relative.is_absolute() {
			return Err(failure("absolute include paths are not supported".into()));
		}
		let parent = source
			.parent()
			.ok_or_else(|| failure("missing source directory".into()))?;
		let target = parent
			.join(relative)
			.canonicalize()
			.map_err(|error| failure(format!("cannot resolve asset: {error}")))?;
		let anchored = target.strip_prefix(&self.root).map_err(|_| {
			failure(format!(
				"resolved target {} escapes migration root {}",
				target.display(),
				self.root.display()
			))
		})?;
		// Open through the anchored directory so a replaced intermediate symlink
		// cannot turn the validated target into an unrestricted ambient read.
		if !self
			.directory
			.metadata(anchored)
			.map_err(|error| failure(format!("cannot inspect asset: {error}")))?
			.is_file()
		{
			return Err(failure("asset must be a regular UTF-8 text file".into()));
		}
		let file = self
			.directory
			.open(anchored)
			.map_err(|error| failure(format!("cannot open asset: {error}")))?;
		if !file
			.metadata()
			.map_err(|error| failure(error.to_string()))?
			.is_file()
		{
			return Err(failure("asset must be a regular UTF-8 text file".into()));
		}
		let mut identity =
			Handle::from_file(file.into_std()).map_err(|error| failure(error.to_string()))?;
		if self.sources.contains(&identity) {
			return Err(failure(
				"file is both a migration source and an SQL asset".into(),
			));
		}
		if let Some(contents) = self.assets.get(&identity) {
			self.asset_paths.insert(target);
			return Ok(contents.clone());
		}
		let mut contents = String::new();
		identity
			.as_file_mut()
			.read_to_string(&mut contents)
			.map_err(|error| failure(format!("cannot read UTF-8 asset: {error}")))?;
		self.asset_paths.insert(target);
		self.assets.insert(identity, contents.clone());
		Ok(contents)
	}
}

pub(crate) fn parse_sql_payload(
	expression: &syn::Expr,
	field: &str,
	scope: &mut Option<SqlAssetScope<'_>>,
) -> Result<String> {
	use syn::{Expr, Lit, Token, punctuated::Punctuated};
	let coordinate = scope
		.as_ref()
		.map(|scope| format!("{}: ", scope.source.display()))
		.unwrap_or_default();
	let unsupported = || {
		MigrationError::InvalidMigration(format!("{coordinate}{field} is unsupported or malformed"))
	};
	let mut expression = expression;
	loop {
		match expression {
			Expr::Paren(paren) => expression = &paren.expr,
			Expr::Group(group) => expression = &group.expr,
			Expr::MethodCall(call)
				if call.args.is_empty()
					&& call
						.turbofish
						.as_ref()
						.is_none_or(|args| args.args.is_empty())
					&& (call.method == "to_owned"
						|| call.method == "to_string"
						|| call.method == "into") =>
			{
				expression = &call.receiver;
			}
			_ => break,
		}
	}
	match expression {
		Expr::Lit(syn::ExprLit {
			lit: Lit::Str(value),
			..
		}) => Ok(value.value()),
		Expr::Macro(expression) => {
			if !expression.mac.path.is_ident("include_str") {
				return Err(unsupported());
			}
			let arguments = expression
				.mac
				.parse_body_with(Punctuated::<syn::LitStr, Token![,]>::parse_terminated)
				.map_err(|_| {
					MigrationError::InvalidMigration(format!(
						"{coordinate}{field}: include_str! requires exactly one literal relative string path"
					))
				})?;
			if arguments.len() != 1 {
				return Err(unsupported());
			}
			let include = arguments[0].value();
			let scope = scope.as_mut().ok_or_else(|| MigrationError::InvalidMigration(format!(
				"{field}: include_str!({include:?}) requires an explicit source path and migration root (SqlAssetContext)")))?;
			scope.context.read(scope.source, field, &include)
		}
		_ => Err(unsupported()),
	}
}
