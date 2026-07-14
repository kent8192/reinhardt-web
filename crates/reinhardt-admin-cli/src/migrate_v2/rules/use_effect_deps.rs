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
	attrs: Vec<syn::Attribute>,
}

#[derive(Clone)]
struct GeneratedDepsImport {
	facade: syn::Path,
	attrs: Vec<syn::Attribute>,
}

struct HookVisitor {
	generated_imports: Vec<GeneratedDepsImport>,
	imports: Vec<UseEntry>,
}

fn rewrite_scope(items: &mut Vec<syn::Item>) {
	let imports = collect_imports(items);
	let mut visitor = HookVisitor {
		generated_imports: Vec::new(),
		imports: imports.clone(),
	};
	for item in items.iter_mut() {
		visitor.visit_item_mut(item);
	}
	if !visitor.generated_imports.is_empty() {
		let insertion = items
			.iter()
			.position(|item| matches!(item, syn::Item::Use(_)))
			.unwrap_or(0);
		let mut inserted = 0;
		for generated in visitor.generated_imports {
			if has_deps_import(&imports, &generated.facade, &generated.attrs) {
				continue;
			}
			let attrs = generated.attrs;
			let facade = generated.facade;
			let import = syn::parse2(quote!(#(#attrs)* use #facade::deps;))
				.expect("generated deps import must parse");
			items.insert(insertion + inserted, import);
			inserted += 1;
		}
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
		if let Some((spec, facade, attrs)) = self.resolve_hook(call) {
			if call.args.len() == spec.omitted_arity {
				let callback = call
					.args
					.iter()
					.nth(spec.callback_index)
					.expect("hook callback must exist");
				let deps = dependency_list(callback);
				if deps.is_some() {
					self.record_generated_import(facade.clone(), attrs.clone());
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
				self.record_generated_import(facade, attrs);
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
	fn record_generated_import(&mut self, facade: syn::Path, attrs: Vec<syn::Attribute>) {
		let key = import_key(&facade, &attrs);
		if self
			.generated_imports
			.iter()
			.all(|generated| import_key(&generated.facade, &generated.attrs) != key)
		{
			self.generated_imports
				.push(GeneratedDepsImport { facade, attrs });
		}
	}

	fn resolve_hook(
		&self,
		call: &syn::ExprCall,
	) -> Option<(HookSpec, syn::Path, Vec<syn::Attribute>)> {
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
		if segments.len() == 1 {
			if let Some(entry) = self.imports.iter().find(|entry| entry.bound == *called) {
				let original = entry.original.last()?;
				let spec = hook_spec(&original.to_string())?;
				let facade = facade_from_segments(&entry.original)
					.unwrap_or_else(|| syn::parse_quote!(reinhardt_pages));
				return Some((spec, facade, entry.attrs.clone()));
			}
			let spec = hook_spec(&called.to_string())?;
			return Some((spec, syn::parse_quote!(reinhardt_pages), Vec::new()));
		}
		for (index, segment) in segments.iter().enumerate().take(segments.len() - 1) {
			let Some(entry) = self.imports.iter().find(|entry| entry.bound == *segment) else {
				continue;
			};
			let mut resolved = entry.original.clone();
			resolved.extend(segments.iter().skip(index + 1).cloned());
			let Some(original) = resolved.last() else {
				continue;
			};
			let Some(spec) = hook_spec(&original.to_string()) else {
				continue;
			};
			let facade = facade_from_segments(&resolved)
				.unwrap_or_else(|| syn::parse_quote!(reinhardt_pages));
			return Some((spec, facade, entry.attrs.clone()));
		}
		let spec = hook_spec(&called.to_string())?;
		let facade =
			facade_from_segments(&segments).unwrap_or_else(|| syn::parse_quote!(reinhardt_pages));
		Some((spec, facade, Vec::new()))
	}
}

fn hook_spec(name: &str) -> Option<HookSpec> {
	HOOK_SPECS.iter().copied().find(|spec| spec.name == name)
}

fn facade_from_segments(segments: &[syn::Ident]) -> Option<syn::Path> {
	if segments.is_empty() {
		return None;
	}
	let end = segments
		.iter()
		.position(|ident| ident == "reactive")
		.unwrap_or(segments.len() - 1);
	if end == 0 {
		return None;
	}
	let facade = &segments[..end];
	syn::parse2(quote!(#(#facade)::* )).ok()
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
		let dependency = resolve_alias(&read, &aliases)?;
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
		if call.method == "clone"
			&& call.args.is_empty()
			&& let Some(receiver) = safe_clone_place(&call.receiver)
		{
			aliases.insert(alias.ident.to_string(), receiver);
		}
	}
	aliases
}

fn safe_clone_place(expr: &syn::Expr) -> Option<syn::Expr> {
	match strip_parens(expr) {
		syn::Expr::Path(path) if path.qself.is_none() && !path.path.segments.is_empty() => {
			Some(syn::Expr::Path(path.clone()))
		}
		syn::Expr::Field(field) => {
			let mut field = field.clone();
			field.base = Box::new(safe_clone_place(&field.base)?);
			Some(syn::Expr::Field(field))
		}
		_ => None,
	}
}

fn resolve_alias(name: &str, aliases: &HashMap<String, syn::Expr>) -> Option<syn::Expr> {
	let mut stack = vec![name.to_owned()];
	let expr = aliases.get(name)?;
	resolve_place(expr, aliases, &mut stack)
}

fn resolve_place(
	expr: &syn::Expr,
	aliases: &HashMap<String, syn::Expr>,
	stack: &mut Vec<String>,
) -> Option<syn::Expr> {
	match strip_parens(expr) {
		syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
			let ident = path.path.segments.first()?.ident.to_string();
			let Some(alias) = aliases.get(&ident) else {
				return Some(syn::Expr::Path(path.clone()));
			};
			if stack.last().is_some_and(|current| current == &ident) {
				return Some(syn::Expr::Path(path.clone()));
			}
			if stack.iter().any(|current| current == &ident) {
				return None;
			}
			stack.push(ident);
			let resolved = resolve_place(alias, aliases, stack);
			stack.pop();
			resolved
		}
		syn::Expr::Path(path) if path.qself.is_none() && !path.path.segments.is_empty() => {
			Some(syn::Expr::Path(path.clone()))
		}
		syn::Expr::Field(field) => {
			let mut field = field.clone();
			field.base = Box::new(resolve_place(&field.base, aliases, stack)?);
			Some(syn::Expr::Field(field))
		}
		_ => None,
	}
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
			flatten_use_tree(&item_use.tree, &[], &item_use.attrs, &mut imports);
		}
	}
	imports
}

fn flatten_use_tree(
	tree: &syn::UseTree,
	prefix: &[syn::Ident],
	attrs: &[syn::Attribute],
	imports: &mut Vec<UseEntry>,
) {
	match tree {
		syn::UseTree::Path(path) => {
			let mut next = prefix.to_vec();
			next.push(path.ident.clone());
			flatten_use_tree(&path.tree, &next, attrs, imports);
		}
		syn::UseTree::Name(name) => {
			let mut original = prefix.to_vec();
			original.push(name.ident.clone());
			imports.push(UseEntry {
				original,
				bound: name.ident.clone(),
				attrs: attrs.to_vec(),
			});
		}
		syn::UseTree::Rename(rename) => {
			let mut original = prefix.to_vec();
			original.push(rename.ident.clone());
			imports.push(UseEntry {
				original,
				bound: rename.rename.clone(),
				attrs: attrs.to_vec(),
			});
		}
		syn::UseTree::Group(group) => {
			for tree in &group.items {
				flatten_use_tree(tree, prefix, attrs, imports);
			}
		}
		syn::UseTree::Glob(_) => {}
	}
}

fn import_key(facade: &syn::Path, attrs: &[syn::Attribute]) -> String {
	quote!(#facade #(#attrs)*).to_string()
}

fn has_deps_import(imports: &[UseEntry], facade: &syn::Path, attrs: &[syn::Attribute]) -> bool {
	let key = import_key(facade, attrs);
	imports.iter().any(|entry| {
		entry.bound == "deps"
			&& entry.original.last().is_some_and(|ident| ident == "deps")
			&& facade_from_segments(&entry.original)
				.is_some_and(|entry_facade| import_key(&entry_facade, &entry.attrs) == key)
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

	#[test]
	fn clone_alias_chain_resolves_to_the_outer_dependency() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(count: Signal<i32>) {
    let _ = use_effect({
        let first = count.clone();
        let second = first.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![count]"));
		assert!(!output.contains("compile_error!"));
	}

	#[test]
	fn clone_alias_chain_accepts_a_field_place() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(state: State) {
    let _ = use_effect({
        let first = state.count.clone();
        let second = first.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert!(output.contains("deps![state.count]"));
		assert!(!output.contains("compile_error!"));
	}

	#[test]
	fn side_effecting_or_indexed_clone_receivers_keep_review_markers() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view(signals: Vec<Signal<i32>>) {
    let _ = use_effect({
        let signal = make_signal().clone();
        move || { let _ = signal.get(); }
    });
    let _ = use_effect({
        let signal = signals[0].clone();
        move || { let _ = signal.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 2);
	}

	#[test]
	fn cyclic_clone_aliases_keep_the_review_marker() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::use_effect;
fn view() {
    let _ = use_effect({
        let first = second.clone();
        let second = first.clone();
        move || { let _ = second.get(); }
    });
}
"#,
		));
		assert_eq!(output.matches("compile_error!").count(), 1);
	}

	#[test]
	fn module_alias_hook_call_restores_the_full_facade() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks as hooks;
fn view() { let _ = hooks::use_effect(|| {}); }
"#,
		));
		assert!(output.contains("usereinhardt_pages::deps;"));
		assert!(output.contains("hooks::use_effect(||{},deps![]);"));
	}

	#[test]
	fn module_name_and_self_qualified_hook_calls_restore_the_facade() {
		let output = compact(&rewrite(
			r#"
use reinhardt_pages::reactive::hooks;
fn one() { let _ = hooks::use_effect(|| {}); }
fn two() { let _ = self::hooks::use_effect(|| {}); }
"#,
		));
		assert_eq!(output.matches("usereinhardt_pages::deps;").count(), 1);
		assert_eq!(output.matches("deps![]").count(), 2);
	}

	#[test]
	fn generated_deps_import_inherits_hook_import_cfg() {
		let output = rewrite(
			r#"
#[cfg(wasm)]
use reinhardt_pages::reactive::hooks as hooks;
fn view() { let _ = hooks::use_effect(|| {}); }
"#,
		);
		assert!(output.contains("#[cfg(wasm)]\nuse reinhardt_pages::deps;"));
	}

	#[test]
	fn generated_deps_imports_deduplicate_by_cfg_and_path() {
		let output = rewrite(
			r#"
#[cfg(wasm)]
use reinhardt_pages::reactive::hooks as wasm_hooks;
#[cfg(native)]
use reinhardt_pages::reactive::hooks as native_hooks;
fn view() {
    let _ = wasm_hooks::use_effect(|| {});
    let _ = wasm_hooks::use_layout_effect(|| {});
    let _ = native_hooks::use_effect(|| {});
}
"#,
		);
		assert_eq!(
			output
				.matches("#[cfg(wasm)]\nuse reinhardt_pages::deps;")
				.count(),
			1
		);
		assert_eq!(
			output
				.matches("#[cfg(native)]\nuse reinhardt_pages::deps;")
				.count(),
			1
		);
	}
}
