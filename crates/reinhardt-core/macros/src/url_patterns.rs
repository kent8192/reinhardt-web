//! Shared URL builder validation and target-specific HTTP expression erasure.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{Expr, ItemFn, Pat, PathArguments, ReturnType, Stmt, Type, visit::Visit};

pub(crate) fn url_patterns_impl(args: TokenStream, mut input: ItemFn) -> syn::Result<TokenStream> {
	if !args.is_empty() {
		return Err(syn::Error::new_spanned(
			args,
			"#[url_patterns] does not accept arguments",
		));
	}
	validate_signature(&input)?;
	let [Stmt::Expr(native_expr, None)] = input.block.stmts.as_slice() else {
		return Err(syn::Error::new_spanned(
			&input.block,
			"#[url_patterns] requires one tail builder expression, starting with UnifiedRouter::new() or UnifiedRouter::default()",
		));
	};
	let disabled_expr = erase_server_calls(native_expr)?;

	// Keep the enabled expression intact, including its temporary lifetimes.
	// The caller's cfg removes the other block before resolving handler paths.
	*input.block = syn::parse_quote!({
		#[cfg(all(server, not(all(target_family = "wasm", target_os = "unknown"))))]
		{
			#native_expr
		}
		#[cfg(not(all(server, not(all(target_family = "wasm", target_os = "unknown")))))]
		{
			#disabled_expr
		}
	});
	Ok(quote!(#input))
}

fn validate_signature(input: &ItemFn) -> syn::Result<()> {
	let signature = &input.sig;
	if signature.asyncness.is_some()
		|| signature.unsafety.is_some()
		|| signature.constness.is_some()
		|| signature.abi.is_some()
		|| !signature.inputs.is_empty()
		|| signature.variadic.is_some()
		|| !signature.generics.params.is_empty()
		|| signature.generics.where_clause.is_some()
	{
		return Err(syn::Error::new_spanned(
			signature,
			"#[url_patterns] requires a safe synchronous function without parameters, generics, const, or extern qualifiers",
		));
	}
	if let ReturnType::Type(_, output) = &signature.output
		&& let Type::Path(output) = output.as_ref()
		&& output.qself.is_none()
		&& output
			.path
			.segments
			.last()
			.is_some_and(|segment| segment.ident == "UnifiedRouter" && segment.arguments.is_empty())
	{
		return Ok(());
	}
	Err(syn::Error::new_spanned(
		&signature.output,
		"#[url_patterns] requires an explicit UnifiedRouter return type; qualified paths are supported",
	))
}

fn erase_server_calls(expr: &Expr) -> syn::Result<Expr> {
	match expr {
		Expr::Paren(paren) => {
			let mut paren = paren.clone();
			paren.expr = Box::new(erase_server_calls(&paren.expr)?);
			Ok(Expr::Paren(paren))
		}
		Expr::Group(group) => {
			let mut group = group.clone();
			group.expr = Box::new(erase_server_calls(&group.expr)?);
			Ok(Expr::Group(group))
		}
		Expr::Call(_) if is_unified_router_constructor(expr) => Ok(expr.clone()),
		Expr::MethodCall(call) if call.method == "server" => {
			if call.args.len() != 1 || call.turbofish.is_some() {
				return Err(syn::Error::new_spanned(
					call,
					"#[url_patterns] .server(...) requires exactly one argument and no explicit generic arguments",
				));
			}
			// The complete argument is opaque: native-only setup and captures must
			// disappear together with the closure or configuration function path.
			erase_server_calls(&call.receiver)
		}
		Expr::MethodCall(call) => {
			if ![
				"client",
				"with_prefix",
				"with_namespace",
				"mount_unified",
				"merge",
			]
			.iter()
			.any(|method| call.method == *method)
			{
				return Err(syn::Error::new_spanned(
					&call.method,
					format!(
						"#[url_patterns] does not support the outer builder method `{}`; supported methods are server, client, with_prefix, with_namespace, mount_unified, and merge",
						call.method
					),
				));
			}
			let receiver = erase_server_calls(&call.receiver)?;
			for argument in &call.args {
				reject_nested_server_builder(argument)?;
			}
			let mut call = call.clone();
			call.receiver = Box::new(receiver);
			Ok(Expr::MethodCall(call))
		}
		_ => Err(syn::Error::new_spanned(
			expr,
			"#[url_patterns] requires a direct UnifiedRouter::new() or UnifiedRouter::default() builder chain",
		)),
	}
}

fn unparenthesized(mut expr: &Expr) -> &Expr {
	loop {
		expr = match expr {
			Expr::Paren(paren) => &paren.expr,
			Expr::Group(group) => &group.expr,
			_ => return expr,
		};
	}
}

fn is_unified_router_constructor(expr: &Expr) -> bool {
	let Expr::Call(call) = unparenthesized(expr) else {
		return false;
	};
	let Expr::Path(function) = unparenthesized(&call.func) else {
		return false;
	};
	if !call.args.is_empty()
		|| function.qself.is_some()
		|| function
			.path
			.segments
			.iter()
			.any(|segment| !matches!(segment.arguments, PathArguments::None))
	{
		return false;
	}
	let mut segments = function.path.segments.iter().rev();
	matches!(
		(segments.next(), segments.next()),
		(Some(method), Some(router))
			if router.ident == "UnifiedRouter" && (method.ident == "new" || method.ident == "default")
	)
}

fn is_unified_router_builder(expr: &Expr, aliases: &HashSet<String>) -> bool {
	let expr = unparenthesized(expr);
	if let Expr::Block(block) = expr {
		return is_unified_router_block(&block.block, aliases);
	}
	if is_unified_router_constructor(expr) {
		return true;
	}
	match expr {
		// A procedural macro cannot resolve a helper's return type. Treat call
		// results as potential routers so nested server builders fail on every target.
		Expr::Call(_) => true,
		Expr::If(expression) => expression.else_branch.as_ref().is_some_and(|(_, branch)| {
			is_unified_router_block(&expression.then_branch, aliases)
				&& is_unified_router_builder(branch, aliases)
		}),
		Expr::Match(expression) => {
			!expression.arms.is_empty()
				&& expression
					.arms
					.iter()
					.all(|arm| is_unified_router_builder(&arm.body, aliases))
		}
		Expr::MethodCall(call) => is_unified_router_builder(&call.receiver, aliases),
		Expr::Path(path) => {
			let mut segments = path.path.segments.iter();
			let Some(segment) = segments.next() else {
				return false;
			};
			segments.next().is_none()
				&& segment.arguments.is_empty()
				&& aliases.contains(&segment.ident.to_string())
		}
		_ => false,
	}
}

fn is_unified_router_block(block: &syn::Block, aliases: &HashSet<String>) -> bool {
	let [Stmt::Expr(expr, None)] = block.stmts.as_slice() else {
		return false;
	};
	is_unified_router_builder(expr, aliases)
}

fn is_unified_router_server_function(expr: &Expr) -> bool {
	let Expr::Path(path) = unparenthesized(expr) else {
		return false;
	};
	if let Some(qself) = &path.qself {
		return qself.as_token.is_none()
			&& qself.position == 0
			&& is_unified_router_type(&qself.ty)
			&& path.path.segments.len() == 1
			&& path.path.segments.first().is_some_and(|segment| {
				segment.ident == "server" && matches!(segment.arguments, PathArguments::None)
			});
	}
	if path
		.path
		.segments
		.iter()
		.any(|segment| !matches!(segment.arguments, PathArguments::None))
	{
		return false;
	}
	let mut segments = path.path.segments.iter().rev();
	matches!(
		(segments.next(), segments.next()),
		(Some(method), Some(router))
			if method.ident == "server" && router.ident == "UnifiedRouter"
	)
}

fn is_unified_router_type(ty: &Type) -> bool {
	let Type::Path(path) = ty else {
		return false;
	};
	path.qself.is_none()
		&& path.path.segments.last().is_some_and(|segment| {
			segment.ident == "UnifiedRouter" && matches!(segment.arguments, PathArguments::None)
		})
}

fn is_unified_router_type_with_aliases(ty: &Type, router_type_aliases: &HashSet<String>) -> bool {
	is_unified_router_type(ty)
		|| matches!(ty, Type::Path(path)
		if path.qself.is_none()
			&& path.path.segments.len() == 1
			&& path.path.segments.first().is_some_and(|segment| {
				matches!(segment.arguments, PathArguments::None)
					&& router_type_aliases.contains(&segment.ident.to_string())
			}))
}

fn pattern_binding_names(pattern: &Pat) -> HashSet<String> {
	#[derive(Default)]
	struct BindingNames {
		names: HashSet<String>,
	}

	impl<'ast> Visit<'ast> for BindingNames {
		fn visit_pat_ident(&mut self, binding: &'ast syn::PatIdent) {
			self.names.insert(binding.ident.to_string());
			syn::visit::visit_pat_ident(self, binding);
		}
	}

	let mut bindings = BindingNames::default();
	bindings.visit_pat(pattern);
	bindings.names
}

fn router_bindings_in_pattern(
	pattern: &Pat,
	expr: &Expr,
	aliases: &HashSet<String>,
	router_type_aliases: &HashSet<String>,
) -> HashSet<String> {
	fn collect(
		pattern: &Pat,
		expr: &Expr,
		aliases: &HashSet<String>,
		router_type_aliases: &HashSet<String>,
	) -> HashSet<String> {
		match pattern {
			Pat::Ident(binding) => {
				let mut bindings = HashSet::new();
				if is_unified_router_builder(expr, aliases) {
					bindings.insert(binding.ident.to_string());
				}
				if let Some((_, subpattern)) = &binding.subpat {
					bindings.extend(collect(subpattern, expr, aliases, router_type_aliases));
				}
				bindings
			}
			Pat::Type(binding) => {
				if is_unified_router_type_with_aliases(&binding.ty, router_type_aliases) {
					pattern_binding_names(&binding.pat)
				} else if matches!(&*binding.ty, Type::Path(_)) {
					// An explicit non-router type overrides an opaque initializer.
					HashSet::new()
				} else {
					collect(&binding.pat, expr, aliases, router_type_aliases)
				}
			}
			Pat::Paren(binding) => collect(&binding.pat, expr, aliases, router_type_aliases),
			Pat::Reference(binding) => {
				let expr = match unparenthesized(expr) {
					Expr::Reference(reference) => &reference.expr,
					_ => expr,
				};
				collect(&binding.pat, expr, aliases, router_type_aliases)
			}
			Pat::Tuple(tuple) => {
				let Expr::Tuple(expr_tuple) = unparenthesized(expr) else {
					return HashSet::new();
				};
				if tuple.elems.len() != expr_tuple.elems.len() {
					return HashSet::new();
				}
				tuple
					.elems
					.iter()
					.zip(expr_tuple.elems.iter())
					.flat_map(|(pattern, expr)| {
						collect(pattern, expr, aliases, router_type_aliases)
					})
					.collect()
			}
			Pat::TupleStruct(tuple) => {
				let Expr::Call(call) = unparenthesized(expr) else {
					return HashSet::new();
				};
				if tuple.elems.len() != call.args.len() {
					return HashSet::new();
				}
				tuple
					.elems
					.iter()
					.zip(call.args.iter())
					.flat_map(|(pattern, expr)| {
						collect(pattern, expr, aliases, router_type_aliases)
					})
					.collect()
			}
			Pat::Struct(structure) => {
				let Expr::Struct(expr_struct) = unparenthesized(expr) else {
					return HashSet::new();
				};
				structure
					.fields
					.iter()
					.filter_map(|field| {
						expr_struct
							.fields
							.iter()
							.find(|expr_field| expr_field.member == field.member)
							.map(|expr_field| {
								collect(&field.pat, &expr_field.expr, aliases, router_type_aliases)
							})
					})
					.flatten()
					.collect()
			}
			Pat::Slice(slice) => {
				let Expr::Array(array) = unparenthesized(expr) else {
					return HashSet::new();
				};
				if slice.elems.len() != array.elems.len() {
					return HashSet::new();
				}
				slice
					.elems
					.iter()
					.zip(array.elems.iter())
					.flat_map(|(pattern, expr)| {
						collect(pattern, expr, aliases, router_type_aliases)
					})
					.collect()
			}
			Pat::Or(or) => or
				.cases
				.iter()
				.flat_map(|pattern| collect(pattern, expr, aliases, router_type_aliases))
				.collect(),
			_ => HashSet::new(),
		}
	}

	collect(pattern, expr, aliases, router_type_aliases)
}

#[derive(Clone, Default)]
struct RouterBindings {
	routers: HashSet<String>,
	non_routers: HashSet<String>,
}

impl RouterBindings {
	fn bind(
		&mut self,
		pattern: &Pat,
		mut inferred: HashSet<String>,
		router_type_aliases: &HashSet<String>,
	) {
		let typed = typed_router_bindings(pattern, router_type_aliases);
		for name in pattern_binding_names(pattern) {
			self.routers.remove(&name);
			self.non_routers.remove(&name);
		}
		inferred.extend(typed.routers);
		inferred.retain(|name| !typed.non_routers.contains(name));
		self.routers.extend(inferred);
		self.non_routers.extend(typed.non_routers);
	}

	fn restore_names(&mut self, previous: &Self, names: HashSet<String>) {
		for name in names {
			self.routers.remove(&name);
			self.non_routers.remove(&name);
			if previous.routers.contains(&name) {
				self.routers.insert(name.clone());
			}
			if previous.non_routers.contains(&name) {
				self.non_routers.insert(name);
			}
		}
	}

	fn bind_receiver(&mut self, is_router: bool) {
		self.routers.remove("self");
		self.non_routers.remove("self");
		if is_router {
			self.routers.insert("self".to_owned());
		} else {
			self.non_routers.insert("self".to_owned());
		}
	}
}

fn typed_router_bindings(pattern: &Pat, router_type_aliases: &HashSet<String>) -> RouterBindings {
	struct TypedBindings<'a> {
		bindings: RouterBindings,
		router_type_aliases: &'a HashSet<String>,
	}

	impl TypedBindings<'_> {
		fn collect(&mut self, pattern: &Pat, ty: &Type) {
			match (pattern, ty) {
				(Pat::Paren(pattern), _) => self.collect(&pattern.pat, ty),
				(_, Type::Paren(ty)) => self.collect(pattern, &ty.elem),
				(_, Type::Group(ty)) => self.collect(pattern, &ty.elem),
				(Pat::Tuple(pattern), Type::Tuple(ty)) => {
					for (pattern, ty) in pattern.elems.iter().zip(&ty.elems) {
						self.collect(pattern, ty);
					}
				}
				(Pat::Ident(binding), Type::Path(_)) => {
					let names = if is_unified_router_type_with_aliases(ty, self.router_type_aliases)
					{
						&mut self.bindings.routers
					} else {
						&mut self.bindings.non_routers
					};
					names.insert(binding.ident.to_string());
				}
				_ => {}
			}
		}
	}

	impl<'ast> Visit<'ast> for TypedBindings<'_> {
		fn visit_pat_type(&mut self, binding: &'ast syn::PatType) {
			self.collect(&binding.pat, &binding.ty);
			syn::visit::visit_pat_type(self, binding);
		}
	}

	let mut bindings = TypedBindings {
		bindings: RouterBindings::default(),
		router_type_aliases,
	};
	bindings.visit_pat(pattern);
	bindings.bindings
}

fn router_bindings_in_assignment(
	target: &Expr,
	value: &Expr,
	aliases: &HashSet<String>,
) -> HashSet<String> {
	let pairs: Vec<(&Expr, &Expr)> = match (unparenthesized(target), unparenthesized(value)) {
		(Expr::Path(path), _) if path.qself.is_none() => {
			return path
				.path
				.get_ident()
				.filter(|_| is_unified_router_builder(value, aliases))
				.map(|ident| HashSet::from([ident.to_string()]))
				.unwrap_or_default();
		}
		(Expr::Tuple(target), Expr::Tuple(value)) => {
			target.elems.iter().zip(&value.elems).collect()
		}
		(Expr::Array(target), Expr::Array(value)) => {
			target.elems.iter().zip(&value.elems).collect()
		}
		(Expr::Call(target), Expr::Call(value)) => target.args.iter().zip(&value.args).collect(),
		(Expr::Struct(target), Expr::Struct(value)) => target
			.fields
			.iter()
			.filter_map(|field| {
				value
					.fields
					.iter()
					.find(|value| value.member == field.member)
					.map(|value| (&field.expr, &value.expr))
			})
			.collect(),
		_ => Vec::new(),
	};
	pairs
		.into_iter()
		.flat_map(|(target, value)| router_bindings_in_assignment(target, value, aliases))
		.collect()
}

fn router_bindings_in_for_pattern(
	pattern: &Pat,
	expr: &Expr,
	aliases: &HashSet<String>,
	router_type_aliases: &HashSet<String>,
) -> HashSet<String> {
	let items = match unparenthesized(expr) {
		Expr::Array(array) => array.elems.iter().collect::<Vec<_>>(),
		Expr::Tuple(tuple) => tuple.elems.iter().collect::<Vec<_>>(),
		_ => return HashSet::new(),
	};
	items
		.into_iter()
		.flat_map(|item| router_bindings_in_pattern(pattern, item, aliases, router_type_aliases))
		.collect()
}

fn predeclare_router_type_aliases(
	block: &syn::Block,
	outer_aliases: &HashSet<String>,
) -> HashSet<String> {
	let local_aliases = block
		.stmts
		.iter()
		.filter_map(|statement| match statement {
			Stmt::Item(syn::Item::Type(item)) => Some((item.ident.to_string(), item.ty.as_ref())),
			_ => None,
		})
		.collect::<HashMap<_, _>>();
	let mut aliases = outer_aliases.clone();
	for name in local_aliases.keys() {
		aliases.remove(name);
	}

	loop {
		let previous_count = aliases.len();
		for (name, ty) in &local_aliases {
			if is_unified_router_type_with_aliases(ty, &aliases) {
				aliases.insert(name.clone());
			}
		}
		if aliases.len() == previous_count {
			break;
		}
	}

	aliases
}

fn reject_nested_server_builder(expr: &Expr) -> syn::Result<()> {
	#[derive(Default)]
	struct NestedServerBuilder {
		error: Option<syn::Error>,
		bindings: RouterBindings,
		router_type_aliases: HashSet<String>,
		impl_receiver_is_router: Option<bool>,
	}

	impl NestedServerBuilder {
		fn visit_condition(
			&mut self,
			condition: &Expr,
			previous: &mut RouterBindings,
			names: &mut HashSet<String>,
		) {
			if self.error.is_some() {
				return;
			}
			match unparenthesized(condition) {
				Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
					self.visit_condition(&binary.left, previous, names);
					self.visit_condition(&binary.right, previous, names);
				}
				Expr::Let(condition) => {
					// Each initializer runs before its pattern enters scope. Its bindings
					// are then visible to the remaining operands and the branch body.
					self.visit_expr(&condition.expr);
					let inferred = router_bindings_in_pattern(
						&condition.pat,
						&condition.expr,
						&self.bindings.routers,
						&self.router_type_aliases,
					);
					self.visit_pat(&condition.pat);
					let introduced = pattern_binding_names(&condition.pat);
					let new_names = introduced.difference(names).cloned().collect();
					previous.restore_names(&self.bindings, new_names);
					names.extend(introduced);
					self.bindings
						.bind(&condition.pat, inferred, &self.router_type_aliases);
				}
				_ => self.visit_expr(condition),
			}
		}
	}

	impl<'ast> Visit<'ast> for NestedServerBuilder {
		fn visit_block(&mut self, block: &'ast syn::Block) {
			let aliases = self.bindings.clone();
			let router_type_aliases = self.router_type_aliases.clone();
			self.router_type_aliases =
				predeclare_router_type_aliases(block, &self.router_type_aliases);
			let local_names = block
				.stmts
				.iter()
				.filter_map(|statement| {
					let Stmt::Local(local) = statement else {
						return None;
					};
					Some(pattern_binding_names(&local.pat))
				})
				.flatten()
				.collect::<HashSet<_>>();
			syn::visit::visit_block(self, block);

			// Assignments to bindings declared outside this block remain visible after
			// the block. Restore only names introduced by this block, while restoring
			// any outer alias shadowed by a local declaration.
			self.bindings.restore_names(&aliases, local_names);
			self.router_type_aliases = router_type_aliases;
		}

		fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
			if self.error.is_some() {
				return;
			}
			// Assignment evaluates its right-hand side before updating the target.
			self.visit_expr(&assign.right);
			if self.error.is_some() {
				return;
			}
			let inferred =
				router_bindings_in_assignment(&assign.left, &assign.right, &self.bindings.routers);
			// A reassignment cannot change the explicitly declared type of a binding.
			self.bindings.routers.extend(
				inferred
					.into_iter()
					.filter(|name| !self.bindings.non_routers.contains(name)),
			);
			self.visit_expr(&assign.left);
		}

		fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
			if self.error.is_some() {
				return;
			}
			let aliases = self.bindings.clone();
			for input in &closure.inputs {
				self.bindings
					.bind(input, HashSet::new(), &self.router_type_aliases);
			}
			syn::visit::visit_expr_closure(self, closure);
			self.bindings = aliases;
		}

		fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
			if self.error.is_some() {
				return;
			}
			self.visit_expr(&expression.expr);
			if self.error.is_some() {
				return;
			}
			for arm in &expression.arms {
				if self.error.is_some() {
					return;
				}
				let aliases = self.bindings.clone();
				let router_bindings = router_bindings_in_pattern(
					&arm.pat,
					&expression.expr,
					&self.bindings.routers,
					&self.router_type_aliases,
				);
				self.visit_pat(&arm.pat);
				if self.error.is_some() {
					return;
				}
				self.bindings
					.bind(&arm.pat, router_bindings, &self.router_type_aliases);
				if let Some((_, guard)) = &arm.guard {
					self.visit_expr(guard);
				}
				self.visit_expr(&arm.body);
				self.bindings = aliases;
			}
		}

		fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
			if self.error.is_some() {
				return;
			}
			let mut aliases = RouterBindings::default();
			let mut condition_names = HashSet::new();
			self.visit_condition(&expression.cond, &mut aliases, &mut condition_names);
			self.visit_block(&expression.then_branch);
			self.bindings.restore_names(&aliases, condition_names);
			if let Some((_, else_branch)) = &expression.else_branch {
				self.visit_expr(else_branch);
			}
		}

		fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
			if self.error.is_some() {
				return;
			}
			let mut aliases = RouterBindings::default();
			let mut condition_names = HashSet::new();
			self.visit_condition(&expression.cond, &mut aliases, &mut condition_names);
			self.visit_block(&expression.body);
			self.bindings.restore_names(&aliases, condition_names);
		}

		fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
			if self.error.is_some() {
				return;
			}

			// The iterator expression is evaluated before the loop pattern enters
			// scope. For array and tuple literals, track router-valued items in the
			// pattern so aliases are visible only within the loop body.
			let router_bindings = router_bindings_in_for_pattern(
				&expression.pat,
				&expression.expr,
				&self.bindings.routers,
				&self.router_type_aliases,
			);
			self.visit_expr(&expression.expr);
			if self.error.is_some() {
				return;
			}
			let aliases = self.bindings.clone();
			self.bindings
				.bind(&expression.pat, router_bindings, &self.router_type_aliases);
			self.visit_block(&expression.body);
			self.bindings = aliases;
		}

		fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
			if self.error.is_some() {
				return;
			}
			let aliases = std::mem::take(&mut self.bindings);
			for input in &function.sig.inputs {
				if let syn::FnArg::Typed(input) = input {
					self.bindings.bind(
						&Pat::Type(input.clone()),
						HashSet::new(),
						&self.router_type_aliases,
					);
				}
			}
			syn::visit::visit_item_fn(self, function);
			self.bindings = aliases;
		}

		fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
			if self.error.is_some() {
				return;
			}
			let aliases = std::mem::take(&mut self.bindings);
			for input in &function.sig.inputs {
				match input {
					syn::FnArg::Receiver(_) => self
						.bindings
						.bind_receiver(self.impl_receiver_is_router.unwrap_or(false)),
					syn::FnArg::Typed(input) => self.bindings.bind(
						&Pat::Type(input.clone()),
						HashSet::new(),
						&self.router_type_aliases,
					),
				}
			}
			syn::visit::visit_impl_item_fn(self, function);
			self.bindings = aliases;
		}

		fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
			if self.error.is_some() {
				return;
			}
			let previous = self.impl_receiver_is_router;
			self.impl_receiver_is_router = Some(is_unified_router_type_with_aliases(
				&item.self_ty,
				&self.router_type_aliases,
			));
			syn::visit::visit_item_impl(self, item);
			self.impl_receiver_is_router = previous;
		}

		fn visit_trait_item_fn(&mut self, function: &'ast syn::TraitItemFn) {
			if self.error.is_some() {
				return;
			}
			let aliases = std::mem::take(&mut self.bindings);
			for input in &function.sig.inputs {
				if let syn::FnArg::Typed(input) = input {
					self.bindings.bind(
						&Pat::Type(input.clone()),
						HashSet::new(),
						&self.router_type_aliases,
					);
				}
			}
			syn::visit::visit_trait_item_fn(self, function);
			self.bindings = aliases;
		}

		fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
			if self.error.is_some() {
				return;
			}
			let is_router =
				is_unified_router_type_with_aliases(&item.ty, &self.router_type_aliases);
			let name = item.ident.to_string();
			self.router_type_aliases.remove(&name);
			if is_router {
				self.router_type_aliases.insert(name);
			}
			syn::visit::visit_item_type(self, item);
		}

		fn visit_stmt(&mut self, statement: &'ast Stmt) {
			if self.error.is_some() {
				return;
			}
			if let Stmt::Macro(statement_macro) = statement {
				self.error = Some(syn::Error::new_spanned(
					&statement_macro.mac,
					"opaque macros in preserved router arguments are unsupported here; expand the expression before using it",
				));
				return;
			}
			syn::visit::visit_stmt(self, statement);
		}

		fn visit_local(&mut self, local: &'ast syn::Local) {
			let router_bindings = local.init.as_ref().map_or_else(HashSet::new, |init| {
				router_bindings_in_pattern(
					&local.pat,
					&init.expr,
					&self.bindings.routers,
					&self.router_type_aliases,
				)
			});
			// Visit the initializer before the new binding shadows any outer router alias.
			syn::visit::visit_local(self, local);
			if self.error.is_some() {
				return;
			}
			self.bindings
				.bind(&local.pat, router_bindings, &self.router_type_aliases);
		}

		fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
			if self.error.is_some() {
				return;
			}
			if call.method == "server"
				&& is_unified_router_builder(&call.receiver, &self.bindings.routers)
			{
				self.error = Some(syn::Error::new_spanned(
					&call.method,
					"nested UnifiedRouter .server(...) calls are unsupported here; extract the nested builder into a separate #[url_patterns] function",
				));
				return;
			}
			syn::visit::visit_expr_method_call(self, call);
		}

		fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
			if self.error.is_some() {
				return;
			}
			if is_unified_router_server_function(&call.func)
				&& call.args.len() == 2
				&& is_unified_router_builder(&call.args[0], &self.bindings.routers)
			{
				self.error = Some(syn::Error::new_spanned(
					&call.func,
					"nested UnifiedRouter .server(...) calls are unsupported here; extract the nested builder into a separate #[url_patterns] function",
				));
				return;
			}
			syn::visit::visit_expr_call(self, call);
		}

		fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
			if self.error.is_some() {
				return;
			}
			self.error = Some(syn::Error::new_spanned(
				expression,
				"opaque macros in preserved router arguments are unsupported here; expand the expression before using it",
			));
		}
	}

	let mut visitor = NestedServerBuilder::default();
	visitor.visit_expr(expr);
	visitor.error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
	use super::*;
	use proc_macro2::{Delimiter, Group, TokenStream};
	use quote::quote;
	use rstest::rstest;
	use syn::{Expr, ItemFn, parse_quote};

	#[rstest]
	fn expansion_preserves_the_function_and_complete_native_chain() {
		// Arrange
		let input: ItemFn = parse_quote! {
			#[doc = "Shared GitHub endpoints."]
			#[reinhardt::routes]
			pub(crate) fn github_urls() -> reinhardt::UnifiedRouter {
				reinhardt::UnifiedRouter::new()
					.server(|server| server.endpoint(crate::native_handlers::setup))
					.with_namespace("github")
			}
		};
		let expected: ItemFn = parse_quote! {
			#[doc = "Shared GitHub endpoints."]
			#[reinhardt::routes]
			pub(crate) fn github_urls() -> reinhardt::UnifiedRouter {
				#[cfg(all(server, not(all(target_family = "wasm", target_os = "unknown"))))]
				{
					reinhardt::UnifiedRouter::new()
						.server(|server| server.endpoint(crate::native_handlers::setup))
						.with_namespace("github")
				}
				#[cfg(not(all(server, not(all(target_family = "wasm", target_os = "unknown")))))]
				{
					reinhardt::UnifiedRouter::new().with_namespace("github")
				}
			}
		};

		// Act
		let output = url_patterns_impl(TokenStream::new(), input).unwrap();
		let actual: ItemFn = syn::parse2(output).unwrap();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	#[case::empty(quote!(UnifiedRouter::new()), quote!(UnifiedRouter::new()))]
	#[case::default(quote!(::framework::UnifiedRouter::default()), quote!(::framework::UnifiedRouter::default()))]
	#[case::parentheses(quote!((UnifiedRouter::new()).server(configure)), quote!((UnifiedRouter::new())))]
	#[case::outer_parentheses(quote!((UnifiedRouter::new().server(configure))), quote!((UnifiedRouter::new())))]
	#[case::all_preserved_methods(
		quote!(UnifiedRouter::new().with_prefix("/api/").server(first).client(client_config).with_namespace("api").mount_unified("/auth/", auth_urls()).merge(other_urls()).server(second)),
		quote!(UnifiedRouter::new().with_prefix("/api/").client(client_config).with_namespace("api").mount_unified("/auth/", auth_urls()).merge(other_urls()))
	)]
	#[case::captured_closure(
		quote!(UnifiedRouter::new().server({ let owned = String::from("native"); move |server| { consume(owned); server } }).with_prefix("/api/")),
		quote!(UnifiedRouter::new().with_prefix("/api/"))
	)]
	#[case::unrelated_server_method(
		quote!(UnifiedRouter::new().server(native).client(|client| transport.server(client))),
		quote!(UnifiedRouter::new().client(|client| transport.server(client)))
	)]
	#[case::typed_unrelated_call_result(
		quote!(UnifiedRouter::new().client(|client| { let transport: Other = make_transport(); transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let transport: Other = make_transport(); transport.server(client) }))
	)]
	#[case::deferred_unrelated_call_result(
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; transport = make_transport(); transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; transport = make_transport(); transport.server(client) }))
	)]
	#[case::reassigned_unrelated_call_result(
		quote!(UnifiedRouter::new().client(|client| { let mut transport: Other = original; { transport = make_transport(); } transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let mut transport: Other = original; { transport = make_transport(); } transport.server(client) }))
	)]
	#[case::typed_binding_restored_after_shadow(
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; { let transport = UnifiedRouter::new(); consume(transport); } transport = make_transport(); transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; { let transport = UnifiedRouter::new(); consume(transport); } transport = make_transport(); transport.server(client) }))
	)]
	#[case::typed_closure_parameter_assignment(
		quote!(UnifiedRouter::new().client(|client| { consume(|mut transport: Other| { transport = make_transport(); transport.server(client) }); client })),
		quote!(UnifiedRouter::new().client(|client| { consume(|mut transport: Other| { transport = make_transport(); transport.server(client) }); client }))
	)]
	#[case::typed_function_parameter_assignment(
		quote!(UnifiedRouter::new().client(|client| { fn apply(mut transport: Other) { transport = make_transport(); transport.server(); } client })),
		quote!(UnifiedRouter::new().client(|client| { fn apply(mut transport: Other) { transport = make_transport(); transport.server(); } client }))
	)]
	#[case::typed_destructuring_assignment(
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; (transport,) = (make_transport(),); transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; (transport,) = (make_transport(),); transport.server(client) }))
	)]
	#[case::let_chain_else_scope(
		quote!(UnifiedRouter::new().client(|client| { let transport: Other = original; if let Some(transport) = Some(UnifiedRouter::new()) && ready { consume(transport); } else { transport.server(client); } transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let transport: Other = original; if let Some(transport) = Some(UnifiedRouter::new()) && ready { consume(transport); } else { transport.server(client); } transport.server(client) }))
	)]
	#[case::while_let_chain_scope(
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; while let Some(transport) = Some(UnifiedRouter::new()) && ready { consume(transport); } transport = make_transport(); transport.server(client) })),
		quote!(UnifiedRouter::new().client(|client| { let transport: Other; while let Some(transport) = Some(UnifiedRouter::new()) && ready { consume(transport); } transport = make_transport(); transport.server(client) }))
	)]
	#[case::opaque_server_argument(
		quote!(UnifiedRouter::new().server(|server| configure(server, UnifiedRouter::new().server(nested)))),
		quote!(UnifiedRouter::new())
	)]
	fn erasure_only_removes_outer_server_calls(
		#[case] input: TokenStream,
		#[case] expected: TokenStream,
	) {
		// Arrange
		let input: Expr = syn::parse2(input).unwrap();
		let expected: Expr = syn::parse2(expected).unwrap();

		// Act
		let actual = erase_server_calls(&input).unwrap();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn erasure_preserves_invisible_expression_groups() {
		// Arrange
		let input = Expr::Group(syn::ExprGroup {
			attrs: Vec::new(),
			group_token: Default::default(),
			expr: Box::new(parse_quote!(UnifiedRouter::new().server(configure))),
		});
		let mut expected = input.clone();
		let Expr::Group(group) = &mut expected else {
			panic!("fixture must be an expression group");
		};
		group.expr = Box::new(parse_quote!(UnifiedRouter::new()));

		// Act
		let actual = erase_server_calls(&input).unwrap();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	fn constructor_function_accepts_an_invisible_group() {
		// Arrange
		let constructor = Group::new(Delimiter::None, quote!(UnifiedRouter::new));
		let input: Expr = syn::parse2(quote!(#constructor().server(configure))).unwrap();
		let expected: Expr = syn::parse2(quote!(#constructor())).unwrap();

		// Act
		let actual = erase_server_calls(&input).unwrap();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	#[case::async_fn(quote!(async fn urls() -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::unsafe_fn(quote!(unsafe fn urls() -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::const_fn(quote!(const fn urls() -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::extern_fn(quote!(extern "C" fn urls() -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::parameter(quote!(fn urls(value: u8) -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::type_parameter(quote!(fn urls<T>() -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::lifetime(quote!(fn urls<'a>() -> UnifiedRouter { UnifiedRouter::new() }))]
	#[case::where_clause(quote!(fn urls() -> UnifiedRouter where (): Sized { UnifiedRouter::new() }))]
	fn unsupported_signatures_have_a_direct_diagnostic(#[case] input: TokenStream) {
		// Arrange
		let input = syn::parse2(input).unwrap();

		// Act
		let error = url_patterns_impl(TokenStream::new(), input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"#[url_patterns] requires a safe synchronous function without parameters, generics, const, or extern qualifiers"
		);
	}

	#[rstest]
	#[case::missing(quote!(fn urls() { UnifiedRouter::new() }))]
	#[case::other_router(quote!(fn urls() -> ServerRouter { UnifiedRouter::new() }))]
	#[case::alias(quote!(fn urls() -> AppRouter { UnifiedRouter::new() }))]
	#[case::generic(quote!(fn urls() -> UnifiedRouter<()> { UnifiedRouter::new() }))]
	#[case::reference(quote!(fn urls() -> &'static UnifiedRouter { UnifiedRouter::new() }))]
	#[case::qualified_self(quote!(fn urls() -> <App as Routing>::UnifiedRouter { UnifiedRouter::new() }))]
	fn unsupported_return_types_are_rejected(#[case] input: TokenStream) {
		// Arrange
		let input = syn::parse2(input).unwrap();

		// Act
		let error = url_patterns_impl(TokenStream::new(), input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"#[url_patterns] requires an explicit UnifiedRouter return type; qualified paths are supported"
		);
	}

	#[rstest]
	#[case::empty(quote!({}))]
	#[case::let_binding(quote!({ let router = UnifiedRouter::new(); router }))]
	#[case::semicolon(quote!({ UnifiedRouter::new(); }))]
	fn non_tail_bodies_are_rejected(#[case] body: TokenStream) {
		// Arrange
		let input = syn::parse2(quote!(fn urls() -> UnifiedRouter #body)).unwrap();

		// Act
		let error = url_patterns_impl(TokenStream::new(), input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"#[url_patterns] requires one tail builder expression, starting with UnifiedRouter::new() or UnifiedRouter::default()"
		);
	}

	#[rstest]
	#[case::helper(quote!(make_router()))]
	#[case::variable(quote!(router))]
	#[case::return_expr(quote!(return UnifiedRouter::new()))]
	#[case::condition(quote!(if enabled { UnifiedRouter::new() } else { UnifiedRouter::default() }))]
	#[case::match_expr(quote!(match enabled { _ => UnifiedRouter::new() }))]
	#[case::other_constructor(quote!(ServerRouter::new()))]
	#[case::constructor_argument(quote!(UnifiedRouter::new(42)))]
	#[case::constructor_generic(quote!(UnifiedRouter::new::<()>()))]
	#[case::type_generic(quote!(UnifiedRouter::<()>::new()))]
	fn unsupported_roots_are_rejected(#[case] expr: TokenStream) {
		// Arrange
		let expr = syn::parse2(expr).unwrap();

		// Act
		let error = erase_server_calls(&expr).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"#[url_patterns] requires a direct UnifiedRouter::new() or UnifiedRouter::default() builder chain"
		);
	}

	#[rstest]
	#[case(quote!(UnifiedRouter::new().server()))]
	#[case(quote!(UnifiedRouter::new().server(first, second)))]
	#[case(quote!(UnifiedRouter::new().server::<()>(configure)))]
	fn invalid_server_calls_are_rejected(#[case] expr: TokenStream) {
		// Arrange
		let expr = syn::parse2(expr).unwrap();

		// Act
		let error = erase_server_calls(&expr).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"#[url_patterns] .server(...) requires exactly one argument and no explicit generic arguments"
		);
	}

	#[rstest]
	#[case(quote!(UnifiedRouter::new().merge({ let router: Other = original; let router; (router,) = (UnifiedRouter::new(),); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ fn apply((router,): (UnifiedRouter,)) -> UnifiedRouter { router.server(configure) } apply((UnifiedRouter::new(),)) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; if { router = UnifiedRouter::new(); true } && let Some(router) = other { consume(router); } router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; Holder(router) = Holder(UnifiedRouter::new()); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ fn apply(router: UnifiedRouter) -> UnifiedRouter { router.server(crate::native::configure) } apply(UnifiedRouter::new()) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; (router,) = (UnifiedRouter::new(),); router.server(crate::native::configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; [router] = [UnifiedRouter::new()]; router.server(crate::native::configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; Parts { router } = Parts { router: UnifiedRouter::new() }; router.server(crate::native::configure) })))]
	#[case(quote!(UnifiedRouter::new().merge(if let Some(router) = Some(UnifiedRouter::new()) && { consume(router.server(crate::native::configure)); true } { router } else { UnifiedRouter::new() })))]
	#[case(quote!(UnifiedRouter::new().merge(if true && let Some(router) = Some(UnifiedRouter::new()) { router.server(crate::native::configure) } else { UnifiedRouter::new() })))]
	#[case(quote!(UnifiedRouter::new().merge({ while let Some(router) = Some(UnifiedRouter::new()) && true { consume(router.server(crate::native::configure)); } UnifiedRouter::new() })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router: UnifiedRouter = make_router(); router.server(crate::native::configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router: reinhardt::UnifiedRouter = router_value; router.server(crate::native::configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router: UnifiedRouter; router = make_router(); router.server(crate::native::configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let apply = |router: UnifiedRouter| router.server(crate::native::configure); apply(UnifiedRouter::new()) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let apply = |(router): reinhardt::UnifiedRouter| router.server(crate::native::configure); apply(UnifiedRouter::new()) })))]
	#[case(quote!(UnifiedRouter::new().merge(make_router().server(crate::native::configure))))]
	#[case(quote!(UnifiedRouter::new().merge(make_router().with_prefix("/api/").server(crate::native::configure))))]
	#[case(quote!(UnifiedRouter::new().merge(UnifiedRouter::new().server(configure))))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = UnifiedRouter::new(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge(if let router = UnifiedRouter::new() { router.server(configure) } else { unreachable!() })))]
	#[case(quote!(UnifiedRouter::new().mount_unified("/", (UnifiedRouter::default()).server(configure))))]
	#[case(quote!(UnifiedRouter::new().client(|client| { use_nested(UnifiedRouter::new().server(configure)); client })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router: UnifiedRouter = UnifiedRouter::new(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let (router): UnifiedRouter = UnifiedRouter::new(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let mut router: UnifiedRouter = UnifiedRouter::new(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let (router) = UnifiedRouter::new(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = UnifiedRouter::new(); { let router = unrelated; use_it(router); } router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = UnifiedRouter::new(); let router = wrap(router.server(configure)); router })))]
	#[case(quote!(UnifiedRouter::new().merge({ type Router = UnifiedRouter; let router: Router = make_router(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ impl Builder { fn apply(router: UnifiedRouter) -> UnifiedRouter { router.server(configure) } } Builder::apply(UnifiedRouter::new()) })))]
	#[case(quote!(UnifiedRouter::new().merge({ trait Builder { fn apply(router: UnifiedRouter) -> UnifiedRouter { router.server(configure) } } Builder::apply(UnifiedRouter::new()) })))]
	#[case(quote!(UnifiedRouter::new().merge(UnifiedRouter::server(UnifiedRouter::new(), configure))))]
	#[case(quote!(UnifiedRouter::new().merge(<UnifiedRouter>::server(UnifiedRouter::new(), configure))))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = UnifiedRouter::new(); UnifiedRouter::server(router, configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = UnifiedRouter::new(); <UnifiedRouter>::server(router, configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; router = UnifiedRouter::new(); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge(match UnifiedRouter::new() { router => router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let (router,) = (UnifiedRouter::new(),); router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let RouterParts { router } = RouterParts { router: UnifiedRouter::new() }; router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router; { router = UnifiedRouter::new(); } router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge(while let Some(router) = Some(UnifiedRouter::new()) { let _ = router.server(configure); break; })))]
	#[case(quote!(UnifiedRouter::new().merge({ let mut result = UnifiedRouter::new(); for router in [UnifiedRouter::new()] { result = result.merge(router.server(configure)); } result })))]
	#[case(quote!(UnifiedRouter::new().merge(match Some(UnifiedRouter::new()) { Some(router) => router.server(configure), None => UnifiedRouter::new() })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = { UnifiedRouter::new() }; router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router = match flag { true => UnifiedRouter::new(), false => UnifiedRouter::default() }; router.server(configure) })))]
	#[case(quote!(UnifiedRouter::new().merge({ impl SomeLocalTrait for UnifiedRouter { fn apply(self) -> UnifiedRouter { self.server(configure) } } UnifiedRouter::new() })))]
	#[case(quote!(UnifiedRouter::new().merge({ let router: Router = make_router(); type Router = UnifiedRouter; router.server(configure) })))]
	fn nested_builders_require_separate_annotated_functions(#[case] expr: TokenStream) {
		// Arrange
		let expr = syn::parse2(expr).unwrap();

		// Act
		let error = erase_server_calls(&expr).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"nested UnifiedRouter .server(...) calls are unsupported here; extract the nested builder into a separate #[url_patterns] function"
		);
	}

	#[rstest]
	#[case(quote!(UnifiedRouter::new().merge(nested_routes!())))]
	#[case(quote!(UnifiedRouter::new().merge({ nested_routes!(); UnifiedRouter::new() })))]
	#[case(quote!(UnifiedRouter::new().client(client_config!())))]
	#[case(quote!(UnifiedRouter::new().with_prefix(prefix!())))]
	fn opaque_macros_in_preserved_arguments_are_rejected(#[case] expr: TokenStream) {
		// Arrange
		let expr = syn::parse2(expr).unwrap();

		// Act
		let error = erase_server_calls(&expr).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"opaque macros in preserved router arguments are unsupported here; expand the expression before using it"
		);
	}

	#[rstest]
	fn nested_builder_validation_respects_closure_parameter_scope() {
		// Arrange
		let expr: Expr = parse_quote! {
			UnifiedRouter::new().merge({
				let router = UnifiedRouter::new();
				consume(|router: Other| router.server());
				router
			})
		};

		// Act
		let result = erase_server_calls(&expr);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn nested_builder_validation_respects_nested_function_parameter_scope() {
		// Arrange
		let expr: Expr = parse_quote! {
			UnifiedRouter::new().merge({
				let router = UnifiedRouter::new();
				fn configure(router: Other) {
					router.server();
				}
				configure(Other);
				router
			})
		};

		// Act
		let result = erase_server_calls(&expr);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn nested_builder_validation_does_not_inherit_nested_function_outer_aliases() {
		// Arrange
		let expr: Expr = parse_quote! {
			UnifiedRouter::new().merge({
				let router = UnifiedRouter::new();
				fn configure() {
					router.server();
				}
				configure();
				router
			})
		};

		// Act
		let result = erase_server_calls(&expr);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn nested_builder_validation_preserves_aliases_across_opaque_reassignment() {
		// Arrange
		let expr: Expr = parse_quote! {
			UnifiedRouter::new().merge({
				let mut router = UnifiedRouter::new();
				router = normalize(router);
				router.server(configure);
				router
			})
		};

		// Act
		let error = erase_server_calls(&expr).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"nested UnifiedRouter .server(...) calls are unsupported here; extract the nested builder into a separate #[url_patterns] function"
		);
	}

	#[rstest]
	fn nested_builder_validation_respects_match_arm_scope() {
		// Arrange
		let expr: Expr = parse_quote! {
			UnifiedRouter::new().merge(match unrelated {
				router => router.server()
			})
		};

		// Act
		let result = erase_server_calls(&expr);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn attribute_arguments_are_rejected() {
		// Arrange
		let input = parse_quote!(
			fn urls() -> UnifiedRouter {
				UnifiedRouter::new()
			}
		);

		// Act
		let error = url_patterns_impl(quote!(server), input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"#[url_patterns] does not accept arguments"
		);
	}

	#[rstest]
	#[case("websocket")]
	#[case("grpc")]
	#[case("custom")]
	fn unsupported_outer_methods_are_rejected(#[case] method: &str) {
		// Arrange
		let method = syn::Ident::new(method, proc_macro2::Span::call_site());
		let expr = parse_quote!(UnifiedRouter::new().#method(configure));

		// Act
		let error = erase_server_calls(&expr).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			format!(
				"#[url_patterns] does not support the outer builder method `{method}`; supported methods are server, client, with_prefix, with_namespace, mount_unified, and merge"
			)
		);
	}
}
