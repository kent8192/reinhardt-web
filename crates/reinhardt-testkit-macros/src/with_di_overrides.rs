//! Implementation of the `with_di_overrides!` macro.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Expr, Ident, Path, Result, Token, Type};

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
		scope_ident: Ident,
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
		let kind_str = kind.to_string();
		match kind_str.as_str() {
			"singleton" | "request" | "transient" => {}
			_ => {
				return Err(Error::new_spanned(
					&kind,
					format!(
						"unknown override kind `{kind_str}`; expected one of \
						 `singleton`, `request`, `transient`"
					),
				));
			}
		}

		let ty: Type = input.parse()?;

		// Three grammars:
		//   `<kind> <Type> => <closure-expr>`        → install factory
		//   `<kind> <Type> { fields... }`            → seed scope with struct literal
		//   `<kind> <Type>` (Unit or Tuple type)     → seed scope with `<Type>` or `<Type>(...)`
		//
		// For value form, only `singleton` and `request` are valid.
		if input.peek(Token![=>]) {
			input.parse::<Token![=>]>()?;
			let closure: Expr = input.parse()?;
			return Ok(OverrideItem::Factory {
				scope_ident: kind,
				ty,
				closure,
			});
		}

		// Value form — parse the expression that constructs the value.
		//
		// `syn::Type::parse` consumes just the path (e.g. `Cfg`) and stops
		// before `{`, so we need to re-assemble the struct literal manually
		// when the next token is `{`. Otherwise (for unit / tuple struct
		// constructors, function calls, paths), the type itself is a valid
		// expression seed.
		let value: Expr = if input.peek(syn::token::Brace) {
			let path = type_to_path(&ty)?;
			parse_struct_literal_after_path(input, path)?
		} else {
			// Re-use the type as a path expression. This covers `Foo`,
			// `Foo::new()`, etc. — anything that the user could have written
			// purely as an expression starting with the type's path.
			type_to_value_expr(&ty)?
		};

		match kind_str.as_str() {
			"singleton" => Ok(OverrideItem::Singleton { ty, value }),
			"request" => Ok(OverrideItem::Request { ty, value }),
			"transient" => Err(Error::new_spanned(
				&kind,
				"`transient` overrides must use the factory form: \
				 `transient <Type> => |ctx| async { ... }`",
			)),
			_ => unreachable!("kind already validated"),
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

	let body = invocation.items.iter().map(|item| match item {
		OverrideItem::Singleton { ty, value } => quote! {
			__builder.singleton::<#ty>(#value);
		},
		OverrideItem::Request { ty, value } => quote! {
			__builder.request_value::<#ty>(#value);
		},
		OverrideItem::Factory {
			scope_ident,
			ty,
			closure,
		} => {
			let scope_path = match scope_ident.to_string().as_str() {
				"transient" => quote! { ::reinhardt_di::DependencyScope::Transient },
				"singleton" => quote! { ::reinhardt_di::DependencyScope::Singleton },
				"request" => quote! { ::reinhardt_di::DependencyScope::Request },
				_ => unreachable!(),
			};
			quote! {
				__builder.factory::<#ty, _, _>(#scope_path, #closure);
			}
		}
	});

	let expanded = quote! {
		{
			::reinhardt_testkit::fixtures::di_overrides
				::injection_context_with_di_overrides(|__scope, __builder| {
					let _ = __scope;
					#(#body)*
				}).await
		}
	};

	Ok(expanded.into_token_stream())
}
