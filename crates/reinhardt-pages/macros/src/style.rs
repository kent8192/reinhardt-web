mod codegen;

use proc_macro2::TokenStream;
use syn::{Expr, ItemStatic, StaticMutability, Type};

use self::codegen::generate_style_items;

pub(crate) fn expand_style_def(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
	if !args.is_empty() {
		return Err(syn::Error::new_spanned(
			args,
			"style_def does not accept arguments",
		));
	}

	let item: ItemStatic = syn::parse2(item)?;
	if !matches!(item.mutability, StaticMutability::None) {
		return Err(syn::Error::new_spanned(
			&item.mutability,
			"style_def requires an immutable static",
		));
	}

	let Type::Path(style_type) = item.ty.as_ref() else {
		return Err(syn::Error::new_spanned(
			&item.ty,
			"style_def requires one unqualified style type identifier",
		));
	};
	if style_type.qself.is_some()
		|| style_type.path.leading_colon.is_some()
		|| style_type.path.segments.len() != 1
		|| !style_type.path.segments[0].arguments.is_empty()
	{
		return Err(syn::Error::new_spanned(
			&item.ty,
			"style_def requires one unqualified style type identifier",
		));
	}

	let Expr::Macro(initializer) = item.expr.as_ref() else {
		return Err(syn::Error::new_spanned(
			&item.expr,
			"style_def requires a direct bare style! initializer",
		));
	};
	if !initializer.mac.path.is_ident("style") {
		return Err(syn::Error::new_spanned(
			&initializer.mac.path,
			"style_def requires a direct bare style! initializer",
		));
	}

	validate_attributes(&item.attrs)?;
	let style_type_ident = &style_type.path.segments[0].ident;
	let package_name = std::env::var("CARGO_PKG_NAME").map_err(|_| {
		syn::Error::new_spanned(style_type_ident, "style_def requires CARGO_PKG_NAME")
	})?;
	let package_version = std::env::var("CARGO_PKG_VERSION").map_err(|_| {
		syn::Error::new_spanned(style_type_ident, "style_def requires CARGO_PKG_VERSION")
	})?;
	let compiled = reinhardt_manouche::compile_style(
		initializer.mac.tokens.clone(),
		&reinhardt_manouche::StyleCompileContext {
			package_name: &package_name,
			package_version: &package_version,
			style_type_name: &style_type_ident.to_string(),
		},
	)
	.map_err(reinhardt_manouche::StyleDiagnostic::into_syn_error)?;

	generate_style_items(&item, style_type_ident, &compiled)
}

pub(crate) fn expand_standalone_style(input: TokenStream) -> syn::Result<TokenStream> {
	let ast = reinhardt_manouche::parser::parse_style(input)?;
	reinhardt_manouche::validator::validate_style(&ast)
		.map_err(reinhardt_manouche::StyleDiagnostic::into_syn_error)?;
	Err(syn::Error::new(
		ast.span,
		"style! must be the initializer of an immutable static annotated with #[style_def]",
	))
}

fn validate_attributes(attributes: &[syn::Attribute]) -> syn::Result<()> {
	for attribute in attributes {
		let supported = [
			"doc", "cfg", "cfg_attr", "allow", "warn", "deny", "forbid", "expect",
		]
		.iter()
		.any(|name| attribute.path().is_ident(name));
		if !supported {
			return Err(syn::Error::new_spanned(
				attribute,
				"unsupported attribute on style definition",
			));
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use quote::quote;
	use rstest::rstest;

	use super::{expand_standalone_style, expand_style_def};

	#[rstest]
	fn canonical_envelope_generates_the_style_api() {
		// Arrange
		let item = quote! {
			pub(crate) static STYLES: PollCardStyles = style! {
				vars { accent: Color = red; }
				.card { color: vars.accent; }
			};
		};

		// Act
		let output = expand_style_def(quote!(), item).expect("canonical envelope should expand");
		let file = syn::parse2::<syn::File>(output).expect("generated output should be Rust");

		// Assert
		assert_eq!(file.items.len(), 6);
	}

	#[rstest]
	#[case(
		quote!(configured),
		quote!(static STYLES: CardStyles = style! { .card { color: red; } };),
		"style_def does not accept arguments"
	)]
	#[case(
		quote!(),
		quote!(static mut STYLES: CardStyles = style! { .card { color: red; } };),
		"style_def requires an immutable static"
	)]
	#[case(
		quote!(),
		quote!(static STYLES: crate::CardStyles = style! { .card { color: red; } };),
		"style_def requires one unqualified style type identifier"
	)]
	#[case(
		quote!(),
		quote!(static STYLES: CardStyles = crate::style! { .card { color: red; } };),
		"style_def requires a direct bare style! initializer"
	)]
	#[case(
		quote!(),
		quote!(static STYLES: CardStyles = make_styles();),
		"style_def requires a direct bare style! initializer"
	)]
	fn invalid_envelopes_report_exact_messages(
		#[case] args: proc_macro2::TokenStream,
		#[case] item: proc_macro2::TokenStream,
		#[case] expected: &str,
	) {
		// Arrange and Act
		let error = expand_style_def(args, item).expect_err("envelope should be rejected");

		// Assert
		assert_eq!(error.to_string(), expected);
	}

	#[rstest]
	fn standalone_style_validates_then_reports_the_context_error() {
		// Arrange
		let input = quote! { .card { color: red; } };

		// Act
		let error =
			expand_standalone_style(input).expect_err("standalone style should be rejected");

		// Assert
		assert_eq!(
			error.to_string(),
			"style! must be the initializer of an immutable static annotated with #[style_def]"
		);
	}
}
