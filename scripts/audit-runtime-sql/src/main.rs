//! Conservative SQL candidate discovery; reviewed provenance lives in the inventory.
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
	collections::{BTreeMap, BTreeSet},
	env, fs,
	path::Path,
	process::{Command, ExitCode},
};
use syn::{
	spanned::Spanned,
	visit::{self, Visit},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Site {
	id: String,
	path: String,
	symbol: String,
	kind: String,
	line: usize,
	fingerprint: String,
	conditions: Vec<String>,
	test_only: bool,
	category: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
	#[serde(flatten)]
	site: Site,
	ownership: String,
	status: String,
	operation: String,
	representation: String,
	feature_reachability: Vec<String>,
	rationale: String,
	backends: Vec<String>,
	evidence: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	workaround: Option<Workaround>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Workaround {
	issue: String,
	reason: String,
	replacement: String,
	removal_condition: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Inventory {
	schema_version: u32,
	entries: Vec<Entry>,
}

fn category(text: &str) -> Option<&'static str> {
	let word = text
		.trim_start()
		.split(|c: char| !c.is_ascii_alphabetic())
		.next()?
		.to_ascii_uppercase();
	match word.as_str() {
		"SELECT" | "INSERT" | "UPDATE" | "DELETE" | "WITH" | "REPLACE" => Some("dml"),
		"CREATE" | "ALTER" | "DROP" | "TRUNCATE" | "COMMENT" => Some("schema"),
		"BEGIN" | "COMMIT" | "ROLLBACK" | "SAVEPOINT" | "RELEASE" | "SET" | "START" => {
			Some("control")
		}
		"PRAGMA" | "VACUUM" | "ANALYZE" | "SHOW" | "EXPLAIN" | "GRANT" | "REVOKE" => {
			Some("vendor-administration")
		}
		_ => None,
	}
}
fn digest(tokens: &str) -> String {
	format!("{:x}", Sha256::digest(tokens.as_bytes()))
}
fn test_attributes(attrs: &[syn::Attribute]) -> bool {
	attrs.iter().any(|a| {
		let path = a
			.path()
			.segments
			.last()
			.map(|s| s.ident.to_string())
			.unwrap_or_default();
		if matches!(path.as_str(), "test" | "rstest") {
			return true;
		}
		if !a.path().is_ident("cfg") {
			return false;
		}
		// Only predicates that imply `test` count; not(test) and any(test, feature) do not.
		fn implies_test(meta: &syn::Meta) -> bool {
			match meta {
				syn::Meta::Path(p) => p.is_ident("test"),
				syn::Meta::List(l) if l.path.is_ident("all") || l.path.is_ident("any") => {
					let Ok(ms) = l.parse_args_with(
						syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
					) else {
						return false;
					};
					if l.path.is_ident("all") {
						ms.iter().any(implies_test)
					} else {
						!ms.is_empty() && ms.iter().all(implies_test)
					}
				}
				_ => false,
			}
		}
		match &a.meta {
			syn::Meta::List(l) => l.parse_args::<syn::Meta>().is_ok_and(|m| implies_test(&m)),
			_ => false,
		}
	})
}
fn query_name(name: &str) -> bool {
	matches!(
		name,
		"query"
			| "query_as"
			| "query_scalar"
			| "query_with"
			| "query_as_with"
			| "query_scalar_with"
			| "raw_sql"
	)
}
fn execution_wrapper_name(name: &str) -> bool {
	matches!(
		name,
		"execute"
			| "execute_raw"
			| "fetch_one"
			| "fetch_all"
			| "fetch_optional"
			| "execute_with_context"
			| "fetch_one_with_context"
			| "fetch_all_with_context"
			| "fetch_optional_with_context"
			| "fetch_all_with_values"
			| "fetch_one_with_values"
			| "fetch_optional_with_values"
			| "fetch_stream"
			| "fetch_stream_with_context"
			| "execute_generated"
			| "fetch_one_generated"
			| "fetch_all_generated"
			| "fetch_optional_generated"
			| "fetch_stream_generated"
			| "execute_in_savepoint"
			| "fetch_all_in_savepoint"
			| "execute_generated_in_savepoint"
			| "fetch_all_generated_in_savepoint"
			| "uncached_postgres_query"
			| "query" | "query_as"
			| "query_scalar"
	)
}
fn imported_query_aliases(tree: &syn::UseTree, aliases: &mut BTreeSet<String>) {
	match tree {
		syn::UseTree::Path(path) => imported_query_aliases(&path.tree, aliases),
		syn::UseTree::Group(group) => {
			for tree in &group.items {
				imported_query_aliases(tree, aliases);
			}
		}
		syn::UseTree::Rename(rename) if query_name(&rename.ident.to_string()) => {
			aliases.insert(rename.rename.to_string());
		}
		_ => (),
	}
}
struct Scanner<'a> {
	path: &'a str,
	symbols: Vec<String>,
	conditions: Vec<String>,
	test_only: bool,
	occurrences: BTreeMap<(String, String), usize>,
	query_aliases: BTreeSet<String>,
	provenance: Vec<String>,
	sites: Vec<Site>,
}
impl Scanner<'_> {
	fn record(&mut self, kind: &str, span: Span, tokens: impl ToTokens, cat: &str) {
		let symbol = self.symbols.join("::");
		let ordinal = self
			.occurrences
			.entry((symbol.clone(), kind.into()))
			.or_default();
		*ordinal += 1;
		self.sites.push(Site {
			id: format!("{}::{symbol}::{kind}#{ordinal}", self.path),
			path: self.path.into(),
			symbol,
			kind: kind.into(),
			line: span.start().line,
			fingerprint: digest(&format!(
				"{}:{}",
				self.provenance.last().map_or("", String::as_str),
				tokens.to_token_stream()
			)),
			conditions: self.conditions.clone(),
			test_only: self.test_only,
			category: cat.into(),
		});
	}
	fn scope(&mut self, name: String, attrs: &[syn::Attribute], f: impl FnOnce(&mut Self)) {
		let was_test = self.test_only;
		let n = self.conditions.len();
		self.test_only |= test_attributes(attrs);
		self.conditions.extend(
			attrs
				.iter()
				.filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
				.map(|a| a.to_token_stream().to_string()),
		);
		self.symbols.push(name);
		f(self);
		self.symbols.pop();
		self.conditions.truncate(n);
		self.test_only = was_test;
	}
	fn macro_tokens(&mut self, tokens: TokenStream) {
		for token in tokens {
			match token {
				TokenTree::Group(g) => self.macro_tokens(g.stream()),
				TokenTree::Literal(l) => {
					if let Ok(s) = syn::parse_str::<syn::LitStr>(&l.to_string()) {
						if let Some(c) = category(&s.value()) {
							self.record("macro-sql-literal", l.span(), &s, c);
						}
						if s.value().ends_with(".sql") {
							self.record("sql-asset-reference", l.span(), &s, "schema");
						}
					}
				}
				_ => (),
			}
		}
	}
}
impl<'ast> Visit<'ast> for Scanner<'_> {
	fn visit_item_mod(&mut self, n: &'ast syn::ItemMod) {
		self.scope(n.ident.to_string(), &n.attrs, |s| {
			visit::visit_item_mod(s, n)
		});
	}
	fn visit_item_fn(&mut self, n: &'ast syn::ItemFn) {
		self.provenance
			.push(digest(&n.block.to_token_stream().to_string()));
		self.scope(n.sig.ident.to_string(), &n.attrs, |s| {
			visit::visit_item_fn(s, n)
		});
		self.provenance.pop();
	}
	fn visit_impl_item_fn(&mut self, n: &'ast syn::ImplItemFn) {
		self.provenance
			.push(digest(&n.block.to_token_stream().to_string()));
		self.scope(n.sig.ident.to_string(), &n.attrs, |s| {
			visit::visit_impl_item_fn(s, n)
		});
		self.provenance.pop();
	}
	fn visit_trait_item_fn(&mut self, n: &'ast syn::TraitItemFn) {
		if let Some(body) = &n.default {
			self.provenance
				.push(digest(&body.to_token_stream().to_string()));
		}
		self.scope(n.sig.ident.to_string(), &n.attrs, |s| {
			visit::visit_trait_item_fn(s, n)
		});
		if n.default.is_some() {
			self.provenance.pop();
		}
	}
	fn visit_item_trait(&mut self, n: &'ast syn::ItemTrait) {
		self.scope(n.ident.to_string(), &n.attrs, |s| {
			visit::visit_item_trait(s, n)
		});
	}
	fn visit_item_impl(&mut self, n: &'ast syn::ItemImpl) {
		let ty = n.self_ty.to_token_stream().to_string();
		let name = n.trait_.as_ref().map_or(ty.clone(), |(_, tr, _)| {
			format!("{ty} as {}", tr.to_token_stream())
		});
		self.scope(name, &n.attrs, |s| visit::visit_item_impl(s, n));
	}
	fn visit_item_const(&mut self, n: &'ast syn::ItemConst) {
		self.scope(n.ident.to_string(), &n.attrs, |s| {
			visit::visit_item_const(s, n)
		});
	}
	fn visit_item_static(&mut self, n: &'ast syn::ItemStatic) {
		self.scope(n.ident.to_string(), &n.attrs, |s| {
			visit::visit_item_static(s, n)
		});
	}
	fn visit_expr_lit(&mut self, n: &'ast syn::ExprLit) {
		if let syn::Lit::Str(s) = &n.lit
			&& let Some(c) = category(&s.value())
		{
			self.record("sql-literal", n.span(), n, c);
		}
		visit::visit_expr_lit(self, n);
	}
	fn visit_expr_call(&mut self, n: &'ast syn::ExprCall) {
		if let syn::Expr::Path(p) = &*n.func {
			let name = p
				.path
				.segments
				.last()
				.map(|s| s.ident.to_string())
				.unwrap_or_default();
			if query_name(&name) || self.query_aliases.contains(&name) {
				self.record("executor", n.span(), n, "review-required");
			} else if execution_wrapper_name(&name) {
				self.record("execution-wrapper", n.span(), n, "review-required");
			} else if matches!(
				name.as_str(),
				"cust" | "cust_with_values" | "cust_with_expr" | "cust_with_exprs"
			) {
				self.record("custom-fragment", n.span(), n, "review-required");
			}
		}
		visit::visit_expr_call(self, n);
	}
	fn visit_expr_method_call(&mut self, n: &'ast syn::ExprMethodCall) {
		let name = n.method.to_string();
		let renderer = name == "to_sql"
			|| (name == "to_string"
				&& n.args
					.to_token_stream()
					.to_string()
					.contains("QueryBuilder"));
		if renderer || execution_wrapper_name(&name) {
			self.record(
				if renderer {
					"possible-inline-render"
				} else {
					"execution-wrapper"
				},
				n.span(),
				n,
				"review-required",
			);
		}
		visit::visit_expr_method_call(self, n);
	}
	fn visit_macro(&mut self, n: &'ast syn::Macro) {
		let segments = n
			.path
			.segments
			.iter()
			.map(|s| s.ident.to_string())
			.collect::<Vec<_>>();
		if segments.first().is_some_and(|s| s == "async_stream")
			&& segments
				.last()
				.is_some_and(|s| matches!(s.as_str(), "stream" | "try_stream"))
		{
			// These macros contain a Rust block, including yield expressions. Parse
			// that body instead of treating its executor calls as opaque tokens.
			let tokens = &n.tokens;
			if let Ok(body) = syn::parse2::<syn::Block>(quote::quote!({ #tokens })) {
				visit::visit_block(self, &body);
				return;
			}
		}
		self.macro_tokens(n.tokens.clone());
		visit::visit_macro(self, n);
	}
	// Documentation strings and attributes are not executable SQL candidates.
	fn visit_attribute(&mut self, _: &'ast syn::Attribute) {}
}
fn scan_source(path: &str, source: &str) -> Result<Vec<Site>, String> {
	let file = syn::parse_file(source).map_err(|e| format!("{path}: {e}"))?;
	let test_only = path.starts_with("tests/")
		|| path.contains("/tests/")
		|| path.ends_with("/tests.rs")
		|| path.contains("/benches/");
	let mut scanner = Scanner {
		path,
		symbols: vec![],
		conditions: vec![],
		test_only,
		occurrences: BTreeMap::new(),
		query_aliases: BTreeSet::new(),
		provenance: vec![],
		sites: vec![],
	};
	struct Imports(BTreeSet<String>);
	impl<'ast> Visit<'ast> for Imports {
		fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
			imported_query_aliases(&item.tree, &mut self.0);
		}
	}
	let mut imports = Imports(BTreeSet::new());
	imports.visit_file(&file);
	scanner.query_aliases = imports.0;
	scanner.visit_file(&file);
	Ok(scanner.sites)
}
fn scan(root: &Path) -> Result<Vec<Site>, String> {
	let output = Command::new("git")
		.args([
			"ls-files",
			"--cached",
			"--others",
			"--exclude-standard",
			"-z",
		])
		.current_dir(root)
		.output()
		.map_err(|e| e.to_string())?;
	if !output.status.success() {
		return Err("git ls-files failed".into());
	}
	let mut sites = vec![];
	let paths: BTreeSet<_> = output
		.stdout
		.split(|b| *b == 0)
		.filter(|p| !p.is_empty())
		.map(|p| String::from_utf8_lossy(p).into_owned())
		.collect();
	for path in paths {
		if !(path.ends_with(".rs") || path.ends_with(".sql")) {
			continue;
		}
		let source = match fs::read_to_string(root.join(&path)) {
			Ok(source) => source,
			// The index still contains files removed from the working tree.
			// Registry validation must flag their old sites as stale.
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
			Err(error) => return Err(format!("{path}: {error}")),
		};
		if path.ends_with(".rs") {
			match scan_source(&path, &source) {
				Ok(found) => sites.extend(found),
				Err(_) if path.contains("/tests/") || path.starts_with("tests/") => {
					sites.push(Site {
						id: format!("{path}::invalid-syntax-fixture"),
						path: path.clone(),
						symbol: "fixture".into(),
						kind: "invalid-syntax-fixture".into(),
						line: 1,
						fingerprint: digest(&source),
						conditions: vec![],
						test_only: true,
						category: "review-required".into(),
					});
				}
				Err(error) => return Err(error),
			}
		} else {
			sites.push(Site {
				id: format!("{path}::sql-asset"),
				path: path.clone(),
				symbol: "asset".into(),
				kind: "sql-asset".into(),
				line: 1,
				fingerprint: digest(&source),
				conditions: vec![],
				test_only: path.starts_with("tests/") || path.contains("/tests/"),
				category: "schema".into(),
			});
		}
	}
	sites.sort_by(|a, b| a.id.cmp(&b.id));
	Ok(sites)
}
fn validate(inventory: &Inventory, actual: &[Site], complete: bool) -> Result<(), String> {
	if inventory.schema_version != 1 {
		return Err("unsupported inventory schema".into());
	}
	let mut expected = BTreeMap::new();
	for e in &inventory.entries {
		if expected.insert(&e.site.id, &e.site).is_some() {
			return Err(format!("duplicate site {}", e.site.id));
		}
		if !matches!(
			e.ownership.as_str(),
			"framework" | "application-raw" | "test-only" | "renderer" | "non-sql"
		) || !matches!(
			e.status.as_str(),
			"pending" | "typed" | "excluded" | "workaround"
		) || e.rationale.is_empty()
		{
			return Err(format!("invalid classification {}", e.site.id));
		}

		if e.backends.is_empty()
			|| e.feature_reachability.is_empty()
			|| !matches!(
				e.operation.as_str(),
				"dml"
					| "schema" | "introspection"
					| "control" | "vendor-administration"
					| "caller-defined"
					| "not-executed"
					| "review-required"
			) || !matches!(
			e.representation.as_str(),
			"typed-query" | "vendor-sql" | "caller-raw" | "not-executed" | "review-required"
		) || (e.status != "pending" && e.evidence.is_empty())
			|| (e.ownership == "framework" && e.status == "excluded")
			|| (e.ownership != "framework" && e.status != "excluded")
			|| (e.status != "pending" && e.backends.iter().any(|b| b == "review-required"))
			|| (e.status != "pending"
				&& (e.operation == "review-required"
					|| e.representation == "review-required"
					|| e.feature_reachability
						.iter()
						.any(|feature| feature == "review-required")))
		{
			return Err(format!("incomplete provenance {}", e.site.id));
		}
		if e.status == "workaround" {
			let w = e
				.workaround
				.as_ref()
				.ok_or_else(|| format!("missing workaround {}", e.site.id))?;
			let issue_number = w
				.issue
				.strip_prefix("https://github.com/kent8192/reinhardt-web/issues/");
			if !issue_number.is_some_and(|number| {
				!number.is_empty()
					&& number.chars().all(|c| c.is_ascii_digit())
					&& number.parse::<u64>().is_ok_and(|n| n > 0)
			}) || [
				w.reason.as_str(),
				w.replacement.as_str(),
				w.removal_condition.as_str(),
			]
			.iter()
			.any(|s| s.is_empty())
				|| e.evidence.is_empty()
			{
				return Err(format!("incomplete workaround {}", e.site.id));
			}
		}
		if complete && e.ownership == "framework" && e.status == "pending" {
			return Err(format!("pending runtime site {}", e.site.id));
		}
	}
	let actual: BTreeMap<_, _> = actual.iter().map(|s| (&s.id, s)).collect();
	for (id, s) in &actual {
		let Some(old) = expected.get(id) else {
			return Err(format!("unclassified candidate {id}"));
		};
		// Line movement alone does not invalidate a reviewed identity.
		let mut old = (*old).clone();
		old.line = s.line;
		if old != **s {
			return Err(format!(
				"changed candidate {id}; review its provenance and refresh the inventory"
			));
		}
	}
	for id in expected.keys() {
		if !actual.contains_key(id) {
			return Err(format!("stale candidate {id}"));
		}
	}
	Ok(())
}
fn has_workaround_marker(source: &str, line: usize, issue: &str) -> bool {
	let lines: Vec<_> = source.lines().collect();
	let end = line.min(lines.len());
	lines[end.saturating_sub(12)..end].iter().any(|line| {
		let comment = line.trim_start();
		comment.starts_with("//")
			&& comment.contains("Workaround:")
			&& comment.split_whitespace().any(|word| word == issue)
	})
}
fn validate_source_markers(
	root: &Path,
	inventory: &Inventory,
	actual: &[Site],
) -> Result<(), String> {
	let lines: BTreeMap<_, _> = actual
		.iter()
		.map(|site| (site.id.as_str(), site.line))
		.collect();
	for entry in &inventory.entries {
		if entry.status != "workaround" {
			continue;
		}
		let source = fs::read_to_string(root.join(&entry.site.path)).map_err(|e| e.to_string())?;
		let issue = &entry
			.workaround
			.as_ref()
			.ok_or("missing workaround metadata")?
			.issue;
		if !has_workaround_marker(
			&source,
			*lines
				.get(entry.site.id.as_str())
				.ok_or("missing actual workaround site")?,
			issue,
		) {
			return Err(format!(
				"missing source Workaround marker {}",
				entry.site.id
			));
		}
	}
	Ok(())
}
fn run() -> Result<(), String> {
	let args: Vec<_> = env::args().skip(1).collect();
	if args.len() < 2 {
		return Err(
			"usage: audit-runtime-sql <scan|check|complete> <repository-root> [inventory]".into(),
		);
	}
	let sites = scan(Path::new(&args[1]))?;
	if args[0] == "scan" {
		println!(
			"{}",
			serde_json::to_string_pretty(&sites).map_err(|e| e.to_string())?
		);
		return Ok(());
	}
	if !matches!(args[0].as_str(), "check" | "complete") {
		return Err("unknown audit mode".into());
	}
	let file = args.get(2).ok_or("missing inventory path")?;
	let inventory = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
		.map_err(|e| e.to_string())?;
	let sites: Vec<_> = sites.into_iter().filter(|site| !site.test_only).collect();
	validate(&inventory, &sites, args[0] == "complete")?;
	validate_source_markers(Path::new(&args[1]), &inventory, &sites)?;
	println!("runtime SQL audit: {} classified candidates", sites.len());
	Ok(())
}
fn main() -> ExitCode {
	match run() {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("runtime SQL audit: {e}");
			ExitCode::FAILURE
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	#[rstest]
	fn removed_tracked_sources_disappear_from_the_working_tree_scan() {
		// Arrange: the index still lists a deleted runtime source.
		let directory = tempfile::tempdir().unwrap();
		let root = directory.path();
		assert!(
			Command::new("git")
				.args(["init", "--quiet"])
				.current_dir(root)
				.status()
				.unwrap()
				.success()
		);
		fs::write(
			root.join("runtime.rs"),
			"fn run() { sqlx::query(\"SELECT 1\"); }",
		)
		.unwrap();
		assert!(
			Command::new("git")
				.args(["add", "runtime.rs"])
				.current_dir(root)
				.status()
				.unwrap()
				.success()
		);
		assert_eq!(scan(root).unwrap().len(), 2);
		fs::remove_file(root.join("runtime.rs")).unwrap();
		// Act / Assert: stale entries are checked by the registry, not treated as I/O failures.
		assert!(scan(root).unwrap().is_empty());
	}

	#[rstest]
	#[case("SELECT * FROM t", Some("dml"))]
	#[case(" UPDATE t SET v = ?", Some("dml"))]
	#[case("PRAGMA foreign_keys = ON", Some("vendor-administration"))]
	#[case("an ordinary message", None)]
	fn identifies_sql(#[case] source: &str, #[case] expected: Option<&str>) {
		assert_eq!(category(source), expected);
	}
	#[rstest]
	fn distinguishes_conditional_tests_and_macros() {
		let source = r##"#[cfg(all(test, feature="db"))] mod tests { fn fixture() { sqlx::query("SELECT 1"); } }
            #[cfg(any(test, feature="db"))] fn runtime() { let q=format!("DELETE FROM {}", "t"); sqlx::query(&q); }
            #[cfg(not(test))] fn native() { include_str!("migration.sql"); }"##;
		let sites = scan_source("crates/example/src/lib.rs", source).unwrap();
		let executors: Vec<_> = sites
			.iter()
			.filter(|s| s.kind == "executor")
			.map(|s| (s.symbol.as_str(), s.test_only))
			.collect();
		assert_eq!(executors, [("tests::fixture", true), ("runtime", false)]);
		assert_eq!(
			sites
				.iter()
				.filter(|s| s.kind == "macro-sql-literal")
				.count(),
			1
		);
		assert_eq!(
			sites
				.iter()
				.filter(|s| s.kind == "sql-asset-reference")
				.count(),
			1
		);
	}
	#[rstest]
	fn finds_import_alias_candidates_and_wrapper_calls() {
		let sites = scan_source(
			"crates/example/src/lib.rs",
			"use sqlx::query as lookup; fn run() { lookup(sql); conn.execute(sql); Expr::cust_with_values(\"a = ?\", [1]); }",
		)
		.unwrap();
		assert_eq!(
			sites.iter().map(|s| s.kind.as_str()).collect::<Vec<_>>(),
			["executor", "execution-wrapper", "custom-fragment"]
		);
	}
	#[rstest]
	fn shipped_fixture_helpers_remain_runtime_candidates() {
		// Arrange: a fixture is a callable shipped helper, even when rstest generates it.
		let source = "#[fixture] pub fn database() { sqlx::query(\"SELECT 1\"); }";
		// Act
		let shipped =
			scan_source("crates/reinhardt-testkit/src/fixtures/database.rs", source).unwrap();
		let test_only = scan_source("crates/example/tests/fixtures/database.rs", source).unwrap();
		// Assert
		assert_eq!(shipped.len(), 2);
		assert!(shipped.iter().all(|site| !site.test_only));
		assert!(test_only.iter().all(|site| site.test_only));
	}
	#[rstest]
	fn finds_owned_contextual_and_streaming_execution_wrappers() {
		// Arrange: SQL is generated elsewhere, so these wrappers contain no SQL literal.
		let source = "fn run() { engine.fetch_one_with_values(built); db.execute_with_context(sql, values, context); db.fetch_stream(sql, values, size); db.fetch_stream_with_context(sql, values, size, context); db.execute_generated(built, context); db.fetch_one_generated(built, context); db.fetch_all_generated(built, context); db.fetch_optional_generated(built, context); db.fetch_stream_generated(built, size, context); }";
		// Act
		let sites = scan_source("crates/example/src/lib.rs", source).unwrap();
		// Assert: changes to the caller's provenance remain auditable.
		assert_eq!(sites.len(), 9);
		assert!(sites.iter().all(|site| site.kind == "execution-wrapper"));
		assert!(sites.iter().all(|site| !site.test_only));
		let changed = scan_source(
			"crates/example/src/lib.rs",
			&source.replace(
				"engine.fetch_one_with_values(built)",
				"engine.fetch_one_with_values(other)",
			),
		)
		.unwrap();
		assert_ne!(sites[0].fingerprint, changed[0].fingerprint);
	}
	#[rstest]
	fn finds_postgres_signature_bypass_in_stream_and_direct_calls() {
		// Arrange: an unnamed query helper may supply either immediate or streaming execution.
		let source = "async fn rows(sql: String, arguments: Args) { async_stream::stream! { let query = generated::uncached_postgres_query(connection, &sql, arguments).await; yield query.fetch(pool).await; } } async fn one(sql: String, arguments: Args) { let query = generated::uncached_postgres_query(connection, &sql, arguments).await; query.fetch_one(pool).await; }";
		// Act
		let sites = scan_source("crates/example/src/lib.rs", source).unwrap();
		// Assert: helper provenance survives even when fetch has no query constructor.
		assert_eq!(sites.len(), 3);
		assert!(sites.iter().all(|site| site.kind == "execution-wrapper"));
		assert!(sites.iter().all(|site| !site.test_only));
	}
	#[rstest]
	fn async_stream_bodies_expose_executors_and_caller_provenance() {
		// Arrange: no SQL literal is needed to expose a generated execution path.
		let source = "fn rows(sql: String, arguments: Args) { async_stream::stream! { let query = sqlx::query_with(&sql, arguments); yield query.fetch_one(pool).await; } } #[cfg(test)] fn fixture() { async_stream::try_stream! { yield sqlx::query(\"SELECT 1\").fetch_one(pool).await?; } }";
		// Act
		let sites = scan_source("crates/example/src/lib.rs", source).unwrap();
		let changed = scan_source(
			"crates/example/src/lib.rs",
			&source.replace("arguments);", "other);"),
		)
		.unwrap();
		// Assert: runtime body, test cfg and enclosing function fingerprint survive parsing.
		assert_eq!(sites.iter().filter(|s| s.symbol == "rows").count(), 2);
		assert_eq!(
			sites
				.iter()
				.filter(|s| s.symbol == "rows" && s.kind == "executor" && !s.test_only)
				.count(),
			1
		);
		assert_eq!(
			sites
				.iter()
				.filter(|s| s.symbol == "fixture" && s.test_only)
				.count(),
			3
		);
		assert_ne!(sites[0].fingerprint, changed[0].fingerprint);
	}
	#[rstest]
	fn associated_execution_and_savepoint_calls_remain_auditable() {
		// Arrange: UFCS and method syntax describe the same execution responsibility.
		let source = "fn run() { OrmExecutor::execute_generated(db, built, context); db.execute_generated_in_savepoint(built, context); OrmExecutor::fetch_all_in_savepoint(db, sql, values); db.fetch_all_generated_in_savepoint(built, context); }";
		// Act
		let sites = scan_source("crates/example/src/lib.rs", source).unwrap();
		// Assert
		assert_eq!(sites.len(), 4);
		assert!(
			sites
				.iter()
				.all(|s| s.kind == "execution-wrapper" && !s.test_only)
		);
		let changed = scan_source(
			"crates/example/src/lib.rs",
			&source.replace("db, built", "db, other"),
		)
		.unwrap();
		assert_ne!(sites[0].fingerprint, changed[0].fingerprint);
	}
	#[rstest]
	fn trait_defaults_track_named_function_provenance_and_test_conditions() {
		// Arrange: the executor call stays identical while its generated SQL changes.
		let source = "trait Store { fn fetch(&self) { let sql = Query::select().column(\"old\"); db.fetch_all(sql); } #[cfg(test)] fn fixture() { db.execute(sql); } }";
		// Act
		let sites = scan_source("crates/example/src/lib.rs", source).unwrap();
		let changed = scan_source(
			"crates/example/src/lib.rs",
			&source.replace("column(\"old\")", "column(\"new\")"),
		)
		.unwrap();
		// Assert
		assert_eq!(sites.len(), 2);
		assert_eq!(sites[0].symbol, "Store::fetch");
		assert!(!sites[0].test_only);
		assert_ne!(sites[0].fingerprint, changed[0].fingerprint);
		assert_eq!(sites[1].symbol, "Store::fixture");
		assert!(sites[1].test_only);
		assert_eq!(sites[1].fingerprint, changed[1].fingerprint);
	}
	#[rstest]
	fn detects_stale_and_changed_entries_but_allows_line_movement() {
		let sites = scan_source(
			"crates/example/src/lib.rs",
			"fn run() { sqlx::query(\"SELECT 1\"); }",
		)
		.unwrap();
		let mut entries: Vec<_> = sites
			.iter()
			.cloned()
			.map(|site| Entry {
				site,
				ownership: "framework".into(),
				status: "pending".into(),
				operation: "review-required".into(),
				representation: "review-required".into(),
				feature_reachability: vec!["review-required".into()],
				rationale: "Awaiting migration".into(),
				backends: vec!["review-required".into()],
				evidence: vec![],
				workaround: None,
			})
			.collect();
		entries[0].site.line += 10;
		let mut inv = Inventory {
			schema_version: 1,
			entries,
		};
		assert_eq!(validate(&inv, &sites, false), Ok(()));
		assert_eq!(
			validate(&inv, &sites, true),
			Err(format!("pending runtime site {}", sites[0].id))
		);
		inv.entries[0].site.fingerprint = "changed".into();
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!(
				"changed candidate {}; review its provenance and refresh the inventory",
				sites[0].id
			))
		);
		inv.entries[0].site.fingerprint = sites[0].fingerprint.clone();
		inv.entries.pop();
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!("unclassified candidate {}", sites[1].id))
		);
	}
	#[rstest]
	#[case(
		"// Workaround: https://github.com/kent8192/reinhardt-web/issues/6508\nfn run() {}",
		true
	)]
	#[case(
		"// https://github.com/kent8192/reinhardt-web/issues/6508\nfn run() {}",
		false
	)]
	#[case(
		"// Workaround: https://github.com/kent8192/reinhardt-web/issues/6499\nfn run() {}",
		false
	)]
	#[case(
		"let message = \"Workaround: https://github.com/kent8192/reinhardt-web/issues/6508\";\nfn run() {}",
		false
	)]
	fn requires_exact_source_workaround_comment(#[case] source: &str, #[case] expected: bool) {
		assert_eq!(
			has_workaround_marker(
				source,
				2,
				"https://github.com/kent8192/reinhardt-web/issues/6508"
			),
			expected
		);
	}
	#[rstest]
	fn rejects_duplicate_stale_and_invalid_workaround_entries() {
		let sites = scan_source(
			"crates/example/src/lib.rs",
			"fn run() { sqlx::query(sql); }",
		)
		.unwrap();
		let entry = Entry {
			site: sites[0].clone(),
			ownership: "framework".into(),
			status: "workaround".into(),
			operation: "vendor-administration".into(),
			representation: "vendor-sql".into(),
			feature_reachability: vec!["sqlite-backend".into()],
			rationale: "Connection-local inspection".into(),
			backends: vec!["sqlite".into()],
			evidence: vec!["Pool behavior regressions".into()],
			workaround: Some(Workaround {
				issue: "https://github.com/kent8192/reinhardt-web/issues/6508".into(),
				reason: "No typed operation".into(),
				replacement: "Typed inspection".into(),
				removal_condition: "Result contract covered".into(),
			}),
		};
		let mut inv = Inventory {
			schema_version: 1,
			entries: vec![entry.clone()],
		};
		assert_eq!(validate(&inv, &sites, true), Ok(()));
		let reviewed = inv.entries[0].clone();
		inv.entries[0].feature_reachability.clear();
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!("incomplete provenance {}", sites[0].id))
		);
		inv.entries[0] = reviewed.clone();
		inv.entries[0].representation = "review-required".into();
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!("incomplete provenance {}", sites[0].id))
		);
		inv.entries[0] = reviewed;

		inv.entries.push(entry.clone());
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!("duplicate site {}", sites[0].id))
		);
		inv.entries.pop();
		assert_eq!(
			validate(&inv, &[], false),
			Err(format!("stale candidate {}", sites[0].id))
		);
		inv.entries[0]
			.workaround
			.as_mut()
			.unwrap()
			.issue
			.push_str("garbage");
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!("incomplete workaround {}", sites[0].id))
		);
		inv.entries[0].workaround = None;
		assert_eq!(
			validate(&inv, &sites, false),
			Err(format!("missing workaround {}", sites[0].id))
		);
	}
	#[rstest]
	fn tracks_query_provenance_even_when_executor_arguments_are_unchanged() {
		let first = scan_source(
			"crates/example/src/lib.rs",
			"fn run() { let sql = typed_statement(); sqlx::query(&sql); }",
		)
		.unwrap();
		let moved = scan_source(
			"crates/example/src/lib.rs",
			"\n\nfn run() { let sql = typed_statement(); sqlx::query(&sql); }",
		)
		.unwrap();
		let changed = scan_source(
			"crates/example/src/lib.rs",
			"fn run() { let sql = unreviewed_statement(); sqlx::query(&sql); }",
		)
		.unwrap();
		assert_eq!(first[0].id, changed[0].id);
		assert_eq!(first[0].fingerprint, moved[0].fingerprint);
		assert_ne!(first[0].fingerprint, changed[0].fingerprint);
	}
}
