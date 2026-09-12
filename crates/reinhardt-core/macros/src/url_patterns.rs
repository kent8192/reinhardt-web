//! Shared URL builder validation and target-specific HTTP expression erasure.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ItemFn, PathArguments, ReturnType, Stmt, Type, visit::Visit};

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

fn has_unified_router_root(mut expr: &Expr) -> bool {
	loop {
		expr = unparenthesized(expr);
		match expr {
			Expr::MethodCall(call) => expr = &call.receiver,
			_ => return is_unified_router_constructor(expr),
		}
	}
}

fn reject_nested_server_builder(expr: &Expr) -> syn::Result<()> {
	#[derive(Default)]
	struct NestedServerBuilder {
		error: Option<syn::Error>,
	}

	impl<'ast> Visit<'ast> for NestedServerBuilder {
		fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
			if self.error.is_some() {
				return;
			}
			if call.method == "server" && has_unified_router_root(&call.receiver) {
				self.error = Some(syn::Error::new_spanned(
					&call.method,
					"nested UnifiedRouter .server(...) calls are unsupported here; extract the nested builder into a separate #[url_patterns] function",
				));
				return;
			}
			syn::visit::visit_expr_method_call(self, call);
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
	#[case(quote!(UnifiedRouter::new().merge(UnifiedRouter::new().server(configure))))]
	#[case(quote!(UnifiedRouter::new().mount_unified("/", (UnifiedRouter::default()).server(configure))))]
	#[case(quote!(UnifiedRouter::new().client(|client| { use_nested(UnifiedRouter::new().server(configure)); client })))]
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
