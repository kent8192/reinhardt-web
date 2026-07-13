//! Rule §6.1 (4): add an explicit dependency list to hooks that omit it.
//!
//! Tracked reads are migrated to `deps![...]` in source order. When the
//! callback expression cannot be identified, the rule retains the explicit
//! `compile_error!` review marker used by the original v2 migration.

use std::collections::{HashMap, HashSet};

use quote::quote;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};

use crate::migrate_v2::rewriter::FileRewriter;

/// `use_effect_deps` rule entry.
pub struct Rule;

impl FileRewriter for Rule {
	fn name(&self) -> &'static str {
		"use_effect_deps"
	}

	fn rewrite(&self, mut file: syn::File) -> syn::File {
		rewrite_scope(&mut file.items);
		file
	}
}

#[derive(Clone, Copy)]
struct HookSpec {
	name: &'static str,
	callback_index: usize,
	deps_index: usize,
	omitted_arity: usize,
	explicit_arity: usize,
}

const HOOK_SPECS: &[HookSpec] = &[
	HookSpec {
		name: "use_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_layout_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_retained_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_retained_layout_effect",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_memo",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_callback",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_callback_with",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_resource",
		callback_index: 0,
		deps_index: 1,
		omitted_arity: 1,
		explicit_arity: 2,
	},
	HookSpec {
		name: "use_resource_with_key",
		callback_index: 1,
		deps_index: 2,
		omitted_arity: 2,
		explicit_arity: 3,
	},
];

#[derive(Clone)]
struct UseEntry {
	original: Vec<syn::Ident>,
	bound: syn::Ident,
}

struct HookVisitor {
	generated_deps: bool,
	imports: Vec<UseEntry>,
	facade: Option<syn::Path>,
}

fn rewrite_scope(items: &mut Vec<syn::Item>) {
	let imports = collect_imports(items);
	let mut visitor = HookVisitor {
		generated_deps: false,
		imports: imports.clone(),
		facade: None,
	};
	for item in items.iter_mut() {
		visitor.visit_item_mut(item);
	}
	if visitor.generated_deps && !imports_bare_deps(&imports) {
		let facade = visitor
			.facade
			.unwrap_or_else(|| syn::parse_quote!(reinhardt_pages));
		let import =
			syn::parse2(quote!(use #facade::deps;)).expect("generated deps import must parse");
		let insertion = items
			.iter()
			.position(|item| matches!(item, syn::Item::Use(_)))
			.unwrap_or(0);
		items.insert(insertion, import);
	}
	for item in items.iter_mut() {
		if let syn::Item::Mod(module) = item
			&& let Some((_, nested)) = &mut module.content
		{
			rewrite_scope(nested);
		}
	}
}

impl VisitMut for HookVisitor {
	fn visit_item_mod_mut(&mut self, _module: &mut syn::ItemMod) {}

	fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
		if let Some((spec, facade)) = self.resolve_hook(call) {
			if call.args.len() == spec.omitted_arity {
				let callback = call
					.args
					.iter()
					.nth(spec.callback_index)
					.expect("hook callback must exist");
				let deps = dependency_list(callback);
				if deps.is_some() {
					self.generated_deps = true;
					self.facade.get_or_insert(facade);
				}
				call.args
					.insert(spec.deps_index, deps.unwrap_or_else(review_marker));
			} else if call.args.len() == spec.explicit_arity
				&& let Some(deps) = tuple_dependency_list(
					call.args
						.iter()
						.nth(spec.deps_index)
						.expect("hook dependencies must exist"),
				) {
				self.generated_deps = true;
				self.facade.get_or_insert(facade);
				*call
					.args
					.iter_mut()
					.nth(spec.deps_index)
					.expect("hook dependencies must exist") = deps;
			}
		}
		visit_mut::visit_expr_call_mut(self, call);
	}
}

impl HookVisitor {
	fn resolve_hook(&self, call: &syn::ExprCall) -> Option<(HookSpec, syn::Path)> {
		let syn::Expr::Path(path) = &*call.func else {
			return None;
		};
		let segments: Vec<_> = path
			.path
			.segments
			.iter()
			.map(|segment| segment.ident.clone())
			.collect();
		let called = segments.last()?;
		if segments.len() > 1 {
			let spec = hook_spec(&called.to_string())?;
			return Some((spec, facade_from_segments(&segments)));
		}
		if let Some(entry) = self.imports.iter().find(|entry| entry.bound == *called) {
			let original = entry.original.last()?;
			let spec = hook_spec(&original.to_string())?;
			return Some((spec, facade_from_segments(&entry.original)));
		}
		Some((
			hook_spec(&called.to_string())?,
			syn::parse_quote!(reinhardt_pages),
		))
	}
}

fn hook_spec(name: &str) -> Option<HookSpec> {
	HOOK_SPECS.iter().copied().find(|spec| spec.name == name)
}

fn facade_from_segments(segments: &[syn::Ident]) -> syn::Path {
	let end = segments
		.iter()
		.position(|ident| ident == "reactive")
		.unwrap_or(segments.len() - 1);
	let facade = &segments[..end];
	syn::parse2(quote!(#(#facade)::* )).expect("hook facade must parse")
}

fn tuple_dependency_list(deps: &syn::Expr) -> Option<syn::Expr> {
	let syn::Expr::Tuple(tuple) = deps else {
		return None;
	};
	let dependencies = tuple.elems.iter().map(strip_clone);
	Some(
		syn::parse2(quote!(deps![#(#dependencies),*]))
			.expect("generated dependency list must parse"),
	)
}

fn strip_clone(expr: &syn::Expr) -> &syn::Expr {
	if let syn::Expr::MethodCall(call) = expr
		&& call.method == "clone"
		&& call.args.is_empty()
	{
		&call.receiver
	} else {
		expr
	}
}

fn dependency_list(callback: &syn::Expr) -> Option<syn::Expr> {
	let callback = strip_parens(callback);
	let (closure, aliases) = match callback {
		syn::Expr::Closure(closure) => (closure, HashMap::new()),
		syn::Expr::Block(block) => {
			let closure = block.block.stmts.last().and_then(stmt_closure)?;
			(
				closure,
				clone_aliases(&block.block.stmts[..block.block.stmts.len() - 1]),
			)
		}
		_ => return None,
	};
	let mut reads = OuterTrackedReadVisitor::default();
	reads.visit_expr(&closure.body);
	if reads.nested_tracked_read {
		return None;
	}
	let mut dependencies = Vec::new();
	let mut seen = HashSet::new();
	for read in reads.reads {
		let dependency = aliases.get(&read)?;
		let key = quote!(#dependency).to_string();
		if seen.insert(key) {
			dependencies.push(dependency);
		}
	}
	Some(
		syn::parse2(quote!(deps![#(#dependencies),*]))
			.expect("generated dependency list must parse"),
	)
}

fn review_marker() -> syn::Expr {
	syn::parse_quote! {
		compile_error!(
			"manouche-v2 codemod: add explicit deps list here, e.g. `deps![count]`"
		)
	}
}

#[derive(Default)]
struct OuterTrackedReadVisitor {
	reads: Vec<String>,
	seen: HashSet<String>,
	nested_tracked_read: bool,
}

impl<'ast> Visit<'ast> for OuterTrackedReadVisitor {
	fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
		let mut nested = AnyTrackedReadVisitor::default();
		nested.visit_expr_closure(closure);
		self.nested_tracked_read |= nested.found;
	}

	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		let method = call.method.to_string();
		let tracked_read = (matches!(method.as_str(), "get" | "into_value")
			&& call.args.is_empty())
			|| (method == "with" && call.args.len() == 1);
		if tracked_read
			&& let Some(ident) = base_ident_of(&call.receiver)
			&& self.seen.insert(ident.to_string())
		{
			self.reads.push(ident.to_string());
		}
		visit::visit_expr_method_call(self, call);
	}
}

#[derive(Default)]
struct AnyTrackedReadVisitor {
	found: bool,
}

impl<'ast> Visit<'ast> for AnyTrackedReadVisitor {
	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		if is_tracked_read(call) {
			self.found = true;
		}
		visit::visit_expr_method_call(self, call);
	}
}

fn is_tracked_read(call: &syn::ExprMethodCall) -> bool {
	let method = call.method.to_string();
	(matches!(method.as_str(), "get" | "into_value") && call.args.is_empty())
		|| (method == "with" && call.args.len() == 1)
}

fn strip_parens(mut expr: &syn::Expr) -> &syn::Expr {
	while let syn::Expr::Paren(paren) = expr {
		expr = &paren.expr;
	}
	expr
}

fn stmt_closure(stmt: &syn::Stmt) -> Option<&syn::ExprClosure> {
	let syn::Stmt::Expr(expr, _) = stmt else {
		return None;
	};
	let syn::Expr::Closure(closure) = strip_parens(expr) else {
		return None;
	};
	Some(closure)
}

fn clone_aliases(stmts: &[syn::Stmt]) -> HashMap<String, syn::Expr> {
	let mut aliases = HashMap::new();
	for stmt in stmts {
		let syn::Stmt::Local(local) = stmt else {
			continue;
		};
		let syn::Pat::Ident(alias) = &local.pat else {
			continue;
		};
		let Some(init) = &local.init else {
			continue;
		};
		let syn::Expr::MethodCall(call) = strip_parens(&init.expr) else {
			continue;
		};
		if call.method == "clone" && call.args.is_empty() {
			aliases.insert(alias.ident.to_string(), (*call.receiver).clone());
		}
	}
	aliases
}

fn base_ident_of(expr: &syn::Expr) -> Option<&syn::Ident> {
	match expr {
		syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
			Some(&path.path.segments[0].ident)
		}
		syn::Expr::MethodCall(call) => base_ident_of(&call.receiver),
		syn::Expr::Field(field) => base_ident_of(&field.base),
		syn::Expr::Paren(paren) => base_ident_of(&paren.expr),
		syn::Expr::Reference(reference) => base_ident_of(&reference.expr),
		syn::Expr::Try(try_expr) => base_ident_of(&try_expr.expr),
		_ => None,
	}
}

fn collect_imports(items: &[syn::Item]) -> Vec<UseEntry> {
	let mut imports = Vec::new();
	for item in items {
		if let syn::Item::Use(item_use) = item {
			flatten_use_tree(&item_use.tree, &[], &mut imports);
		}
	}
	imports
}

fn flatten_use_tree(tree: &syn::UseTree, prefix: &[syn::Ident], imports: &mut Vec<UseEntry>) {
	match tree {
		syn::UseTree::Path(path) => {
			let mut next = prefix.to_vec();
			next.push(path.ident.clone());
			flatten_use_tree(&path.tree, &next, imports);
		}
		syn::UseTree::Name(name) => {
			let mut original = prefix.to_vec();
			original.push(name.ident.clone());
			imports.push(UseEntry {
				original,
				bound: name.ident.clone(),
			});
		}
		syn::UseTree::Rename(rename) => {
			let mut original = prefix.to_vec();
			original.push(rename.ident.clone());
			imports.push(UseEntry {
				original,
				bound: rename.rename.clone(),
			});
		}
		syn::UseTree::Group(group) => {
			for tree in &group.items {
				flatten_use_tree(tree, prefix, imports);
			}
		}
		syn::UseTree::Glob(_) => {}
	}
}

fn imports_bare_deps(imports: &[UseEntry]) -> bool {
	imports.iter().any(|entry| {
		entry.original.last().is_some_and(|ident| ident == "deps") && entry.bound == "deps"
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn rewrite(source: &str) -> String {
		let file = syn::parse_file(source).expect("test source must parse");
		prettyplease::unparse(&Rule.rewrite(file))
	}

	fn compact(source: &str) -> String {
		source.chars().filter(|ch| !ch.is_whitespace()).collect()
	}

	#[test]
	fn hook_metadata_covers_all_supported_arities() {
		let actual: Vec<_> = HOOK_SPECS
			.iter()
			.map(|spec| {
				(
					spec.name,
					spec.callback_index,
					spec.deps_index,
					spec.omitted_arity,
					spec.explicit_arity,
				)
			})
			.collect();
		assert_eq!(
			actual,
			vec![
				("use_effect", 0, 1, 1, 2),
				("use_layout_effect", 0, 1, 1, 2),
				("use_retained_effect", 0, 1, 1, 2),
				("use_retained_layout_effect", 0, 1, 1, 2),
				("use_memo", 0, 1, 1, 2),
				("use_callback", 0, 1, 1, 2),
				("use_callback_with", 0, 1, 1, 2),
				("use_resource", 0, 1, 1, 2),
				("use_resource_with_key", 1, 2, 2, 3),
			]
		);
	}

	#[test]
	fn keyed_resource_adds_or_converts_third_dependency_argument() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::use_resource_with_key;
fn view(signal: Signal<i32>) {
    let _ = use_resource_with_key("a", || async { Ok::<_, ()>(1) });
    let _ = use_resource_with_key("b", || async { Ok::<_, ()>(2) }, ());
    let _ = use_resource_with_key("c", || async { Ok::<_, ()>(3) }, (signal.clone(),));
}
"#,
		));
		assert!(output.contains("use_resource_with_key(\"a\",||async{Ok::<_,()>(1)},deps![]);"));
		assert!(output.contains("use_resource_with_key(\"b\",||async{Ok::<_,()>(2)},deps![]);"));
		assert!(
			output.contains("use_resource_with_key(\"c\",||async{Ok::<_,()>(3)},deps![signal]);")
		);
	}

	#[test]
	fn explicit_dependency_matrix_converts_empty_single_and_multiple_tuples() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn view(a: Signal<i32>, b: Signal<i32>) {
    let _ = use_effect(|| {}, ());
    let _ = use_effect(|| {}, (a.clone(),));
    let _ = use_effect(|| {}, (a.clone(), b.clone()));
}
"#,
		));
		assert!(output.contains("use_effect(||{},deps![]);"));
		assert!(output.contains("use_effect(||{},deps![a]);"));
		assert!(output.contains("use_effect(||{},deps![a,b]);"));
	}

	#[test]
	fn block_clone_alias_maps_back_to_outer_dependency() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn view(original: Signal<i32>) {
    let _ = use_effect({
        let alias = original.clone();
        move || { let _ = alias.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![original]"));
		assert!(!output.contains("deps![alias]"));
	}

	#[test]
	fn unsafe_omitted_forms_keep_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect(move || { let _ = count.get(); });
    let _ = use_effect(build_callback());
    let _ = use_effect({
        let alias = count.clone();
        move || Some(move || { let _ = alias.get(); })
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 3);
		assert_eq!(output.matches("deps![count]").count(), 3);
		assert!(!output.contains("deps![alias]"));
	}

	#[test]
	fn imports_deps_in_each_inline_module_scope() {
		let output = rewrite(
			r#"
use reinhardt_pages::reactive::hooks::use_effect;
fn root() { let _ = use_effect(|| {}); }
mod nested {
    use reinhardt_pages::reactive::hooks::use_effect;
    fn view() { let _ = use_effect(|| {}); }
}
"#,
		);
		assert_eq!(output.matches("use reinhardt_pages::deps;").count(), 2);
	}

	#[test]
	fn qualified_hook_preserves_full_facade_path() {
		let output = compact(&rewrite(
			r#"
fn view() { let _ = reinhardt::pages::use_effect(|| {}); }
"#,
		));
		assert!(output.contains("usereinhardt::pages::deps;"));
		assert!(!output.contains("usereinhardt::deps;"));
	}

	#[test]
	fn renamed_deps_import_does_not_satisfy_bare_macro_import() {
		let output = rewrite(
			r#"
use reinhardt_pages::deps as hook_deps;
use reinhardt_pages::use_effect;
fn view() { let _ = use_effect(|| {}); }
"#,
		);
		assert!(output.contains("use reinhardt_pages::deps as hook_deps;"));
		assert!(output.contains("use reinhardt_pages::deps;"));
	}
}
