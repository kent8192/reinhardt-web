//! Implementation of the `with_di_overrides!` macro.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Expr, Ident, Path, Result, Token, Type};

/// Scope kind written before the type in the macro invocation. Parsed once
/// from the leading keyword; eliminates re-stringifying the kind during
/// expansion.
#[derive(Copy, Clone)]
enum ScopeKind {
	Singleton,
	Request,
	Transient,
}

impl ScopeKind {
	fn parse_ident(kind: &Ident) -> Result<Self> {
		match kind.to_string().as_str() {
			"singleton" => Ok(Self::Singleton),
			"request" => Ok(Self::Request),
			"transient" => Ok(Self::Transient),
			other => Err(Error::new_spanned(
				kind,
				format!(
					"unknown override kind `{other}`; expected one of \
					 `singleton`, `request`, `transient`"
				),
			)),
		}
	}

	fn dependency_scope_path(self) -> TokenStream {
		match self {
			Self::Singleton => quote! { ::reinhardt_testkit::DependencyScope::Singleton },
			Self::Request => quote! { ::reinhardt_testkit::DependencyScope::Request },
			Self::Transient => quote! { ::reinhardt_testkit::DependencyScope::Transient },
		}
	}
}

/// Parsed top-level invocation: a comma-separated list of override items.
struct Invocation {
	items: Vec<OverrideItem>,
}

enum OverrideItem {
	/// `singleton <Type> <expr>` — pre-seed `SingletonScope` with `<expr>`.
	Singleton { ty: Type, value: Expr },
	/// `request <Type> <expr>` — pre-seed request scope with `<expr>`.
	Request { ty: Type, value: Expr },
	/// `<kind> <Type> => <closure>` — install a factory override.
	Factory {
		scope_kind: ScopeKind,
		ty: Type,
		closure: Expr,
	},
}

impl Parse for Invocation {
	fn parse(input: ParseStream) -> Result<Self> {
		let mut items = Vec::new();
		while !input.is_empty() {
			items.push(input.parse::<OverrideItem>()?);
			if input.is_empty() {
				break;
			}
			input.parse::<Token![,]>()?;
		}
		Ok(Self { items })
	}
}

impl Parse for OverrideItem {
	fn parse(input: ParseStream) -> Result<Self> {
		let kind: Ident = input.parse()?;
		let scope_kind = ScopeKind::parse_ident(&kind)?;

		// The parser only handles a plain `Type` here (via `syn::Type::parse`).
		// Generic types with `<T>`, tuple-struct constructors `MyTuple(args)`,
		// function calls `Foo::new(...)`, and other expression forms are NOT
		// accepted as seed values — use the factory form (`=> |ctx| async { ... }`)
		// for those cases.
		let ty: Type = input.parse()?;

		// Three grammars supported by the value/factory parser:
		//   `<kind> <Type> => <closure-expr>`        → install factory
		//   `<kind> <Type> { fields... }`            → seed scope with struct literal
		//   `<kind> <Type>`                          → seed scope with a unit-struct value
		//
		// For value form, only `singleton` and `request` are valid.
		if input.peek(Token![=>]) {
			input.parse::<Token![=>]>()?;
			let closure: Expr = input.parse()?;
			return Ok(OverrideItem::Factory {
				scope_kind,
				ty,
				closure,
			});
		}

		// Value form — parse the expression that constructs the value.
		//
		// `syn::Type::parse` consumes just the path (e.g. `Cfg`) and stops
		// before `{`, so we re-assemble the struct literal manually when the
		// next token is `{`. Otherwise the type itself is taken as a unit-
		// struct path expression — that is the only non-brace form we accept
		// in the value position.
		let value: Expr = if input.peek(syn::token::Brace) {
			let path = type_to_path(&ty)?;
			parse_struct_literal_after_path(input, path)?
		} else {
			// Re-use the type as a path expression. This only covers unit-
			// struct paths like `Foo` (or qualified `module::Foo`) — call
			// tails are not parsed because `syn::Type::parse` already stopped
			// before any `(` or `;`.
			type_to_value_expr(&ty)?
		};

		match scope_kind {
			ScopeKind::Singleton => Ok(OverrideItem::Singleton { ty, value }),
			ScopeKind::Request => Ok(OverrideItem::Request { ty, value }),
			ScopeKind::Transient => Err(Error::new_spanned(
				&kind,
				"`transient` overrides must use the factory form: \
				 `transient <Type> => |ctx| async { ... }`",
			)),
		}
	}
}

/// Extracts the `Path` from a `Type::Path`, rejecting any other type form with
/// a span-targeted error.
fn type_to_path(ty: &Type) -> Result<Path> {
	match ty {
		Type::Path(tp) if tp.qself.is_none() => Ok(tp.path.clone()),
		_ => Err(Error::new_spanned(
			ty,
			"expected a plain type path before the struct literal body",
		)),
	}
}

/// Re-interprets a `Type` as a value expression (a path expression). Used for
/// unit-struct seeds like `singleton Cfg` where `Cfg` itself is the value.
fn type_to_value_expr(ty: &Type) -> Result<Expr> {
	let path = type_to_path(ty)?;
	Ok(Expr::Path(syn::ExprPath {
		attrs: Vec::new(),
		qself: None,
		path,
	}))
}

/// Parses `{ fields... }` from `input` and returns it bundled with `path` as a
/// `Expr::Struct`.
fn parse_struct_literal_after_path(input: ParseStream, path: Path) -> Result<Expr> {
	let content;
	let brace_token = syn::braced!(content in input);

	let mut fields = syn::punctuated::Punctuated::<syn::FieldValue, Token![,]>::new();
	let mut rest = None;
	let mut dot2_token = None;

	while !content.is_empty() {
		if content.peek(Token![..]) {
			dot2_token = Some(content.parse::<Token![..]>()?);
			if !content.is_empty() {
				rest = Some(Box::new(content.parse::<Expr>()?));
			}
			break;
		}
		fields.push_value(content.parse::<syn::FieldValue>()?);
		if content.is_empty() {
			break;
		}
		fields.push_punct(content.parse::<Token![,]>()?);
	}

	Ok(Expr::Struct(syn::ExprStruct {
		attrs: Vec::new(),
		qself: None,
		path,
		brace_token,
		fields,
		dot2_token,
		rest,
	}))
}

/// Expands the macro into a call to
/// `reinhardt_testkit::fixtures::di_overrides::injection_context_with_di_overrides`.
pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
	let invocation = syn::parse2::<Invocation>(input)?;

	// Use `Span::mixed_site()` for the macro-injected idents so they cannot
	// unify with user-defined `__scope` / `__builder` bindings at the call
	// site. The outer ident is renamed to `scope_arg` to avoid shadowing the
	// inner `scope_ident` field on `OverrideItem::Factory`.
	let scope_arg = ::syn::Ident::new("__scope", ::proc_macro2::Span::mixed_site());
	let builder_ident = ::syn::Ident::new("__builder", ::proc_macro2::Span::mixed_site());

	let body: Vec<_> = invocation
		.items
		.iter()
		.map(|item| match item {
			OverrideItem::Singleton { ty, value } => quote! {
				#builder_ident.singleton::<#ty>(#value);
			},
			OverrideItem::Request { ty, value } => quote! {
				#builder_ident.request_value::<#ty>(#value);
			},
			OverrideItem::Factory {
				scope_kind,
				ty,
				closure,
			} => {
				let scope_path = scope_kind.dependency_scope_path();
				quote! {
					#builder_ident.factory::<#ty, _, _>(#scope_path, #closure);
				}
			}
		})
		.collect();

	let expanded = quote! {
		{
			::reinhardt_testkit::fixtures::di_overrides
				::injection_context_with_di_overrides(|#scope_arg, #builder_ident| {
					let _ = #scope_arg;
					#(#body)*
				}).await
		}
	};

	Ok(expanded.into_token_stream())
}
