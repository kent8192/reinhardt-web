//! Generates typed bindings absent from the primitive ControlBinding constructors.

use super::{EditableField, FieldKind};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(super) fn additional_arm(
	field: &EditableField,
	field_ident: &Ident,
	pages: &TokenStream,
) -> Option<TokenStream> {
	let name = &field.name;
	let variant = &field.variant;
	let support = quote!(#pages::__private::client_form);
	let (kind, binding) = match field.kind {
		FieldKind::OptionScalar => {
			let error = field.number_error_ident();
			(
				quote!(Number),
				quote!(#support::optional_number_binding(self.#name, self.#error)),
			)
		}
		FieldKind::OptionBool => (
			quote!(SelectOne),
			quote!(#support::optional_choice_binding(self.#name, &#support::BOOLEAN_CHOICES)),
		),
		FieldKind::Enum | FieldKind::OptionEnum => {
			let ty = field.choice_ty()?;
			let choices = quote!(<#ty as #pages::ClientFormChoiceSource>::client_form_choices());
			let function = if matches!(field.kind, FieldKind::Enum) {
				quote!(#support::choice_binding)
			} else {
				quote!(#support::optional_choice_binding)
			};
			(quote!(SelectOne), quote!(#function(self.#name, #choices)))
		}
		_ => return None,
	};
	Some(quote! {
		(#field_ident::#variant, #pages::component::ControlKind::#kind) => {
			::core::option::Option::Some(#binding)
		},
	})
}
