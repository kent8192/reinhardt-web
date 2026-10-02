//! Generates DTO-specific metadata and typed presentation adapters.

use super::{FieldKind, FormItemContext, ident_name_without_raw_prefix};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Attribute, Expr, Lit, Meta, Token, punctuated::Punctuated};

/// Inspect only the literal length minimum used for a safe required hint.
pub(super) fn positive_length_min(attrs: &[Attribute]) -> bool {
	attrs.iter().filter(|attr| attr.path().is_ident("validate")).any(|attr| {
        let Ok(rules) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else { return false };
        rules.iter().any(|rule| {
            let Meta::List(length) = rule else { return false };
            if !length.path.is_ident("length") { return false; }
            let Ok(bounds) = length.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else { return false };
            bounds.iter().any(|bound| {
                matches!(bound, Meta::NameValue(min)
                    if min.path.is_ident("min") && matches!(&min.value, Expr::Lit(value)
                        if matches!(&value.lit, Lit::Int(value) if value.base10_parse::<usize>().is_ok_and(|value| value > 0))))
            })
        })
    })
}

pub(super) fn generate(context: &FormItemContext<'_>) -> TokenStream {
	let pages = context.pages_crate;
	let dto = context.dto_ident;
	let form = context.form_ident;
	let vis = context.dto_vis;
	let field_token = context.field_ident;
	let wrapper = format_ident!("__{}View", form);
	let support = quote!(#pages::__private::client_form);
	let fields: Vec<_> = context
		.fields
		.iter()
		.filter(|field| field.exposes_field_token(vis))
		.collect();
	let descriptors = fields.iter().enumerate().map(|(ordinal, field)| {
        let variant = &field.variant;
        let rust_name = ident_name_without_raw_prefix(&field.name);
        let wire_name = &field.serialized_name;
        let required = context.validate && matches!(field.kind, FieldKind::String) && field.positive_length_min;
        let (widget, control) = match field.kind {
            FieldKind::String | FieldKind::OptionString => (quote!(TextInput), quote!(Text)),
            FieldKind::Scalar | FieldKind::OptionScalar => (quote!(NumberInput), quote!(Number)),
            FieldKind::Bool => (quote!(CheckboxInput), quote!(Checkbox)),
            _ => (quote!(Select), quote!(SelectOne)),
        };
        let optional_choice = matches!(field.kind, FieldKind::OptionBool | FieldKind::OptionEnum);
        let boolean_choice = matches!(field.kind, FieldKind::OptionBool);
        let choices = if let Some(ty) = field.choice_ty() {
            quote! {
                <#ty as #pages::ClientFormChoiceSource>::client_form_choices()
                    .iter().enumerate().map(|(index, choice)| (
                        ::std::format!("choice:{index}"), ::std::string::String::from(choice.label)
                    )).collect()
            }
        } else if boolean_choice {
            quote! {
                #support::BOOLEAN_CHOICES.iter().enumerate().map(|(index, choice)| (
                    ::std::format!("choice:{index}"), ::std::string::String::from(choice.label)
                )).collect()
            }
        } else { quote!(::std::vec::Vec::new()) };
        quote! {
            #support::ViewField {
                key: #field_token::#variant, ordinal: #ordinal, rust_name: #rust_name,
                serialized_name: #wire_name, required: #required,
                presentation: #support::FieldDisplay::new(
                    #rust_name, #support::WidgetKind::#widget, #choices, #optional_choice, #boolean_choice,
                ),
                binding: #support::view_binding(runtime, #field_token::#variant, #pages::component::ControlKind::#control),
            }
        }
    });
	let customization_methods = fields.iter().enumerate().map(|(ordinal, field)| {
        let method_vis = if matches!(field.vis, syn::Visibility::Inherited) { vis } else { &field.vis };
        let method = format_ident!("__reinhardt_field_{}", ident_name_without_raw_prefix(&field.name));
        let kind = match field.kind {
            FieldKind::String | FieldKind::OptionString => quote!(#support::TextKind),
            FieldKind::Scalar | FieldKind::OptionScalar => quote!(#support::NumberKind),
            FieldKind::Bool => quote!(#support::BoolKind),
            FieldKind::OptionBool => quote!(#support::OptionalBoolKind),
            FieldKind::Enum => {
                let ty = field.choice_ty();
                quote!(#support::EnumKind<#ty>)
            }
            FieldKind::OptionEnum => {
                let ty = field.choice_ty();
                quote!(#support::OptionalEnumKind<#ty>)
            }
        };
        quote_spanned! {field.name.span()=>
            #[doc(hidden)]
            #method_vis fn #method(mut self, configure: impl ::core::ops::FnOnce(#support::FieldPresentation<#kind>)
                -> #support::FieldPresentation<#kind>) -> Self {
                self.inner = self.inner.customize(#ordinal, configure);
                self
            }
        }
    });
	quote! {
		#[doc(hidden)]
		#vis struct #wrapper<Deps, Output>
		where Deps: ::core::clone::Clone + ::core::cmp::PartialEq + 'static,
			Output: ::core::clone::Clone + 'static {
			inner: #support::ClientFormView<#form, Deps, #dto, Output>,
		}
		impl<Deps, Output> #wrapper<Deps, Output>
		where Deps: ::core::clone::Clone + ::core::cmp::PartialEq + 'static,
			Output: ::core::clone::Clone + 'static {
			#(#customization_methods)*
			/// Converts the named form view into a retained Page.
			pub fn into_page(self) -> #pages::component::Page { self.inner.into_page() }
		}
		impl<Deps, Output> #pages::component::IntoPage for #wrapper<Deps, Output>
		where Deps: ::core::clone::Clone + ::core::cmp::PartialEq + 'static,
			Output: ::core::clone::Clone + 'static {
			fn into_page(self) -> #pages::component::Page { self.into_page() }
		}
		impl #form {
			#[doc(hidden)]
			pub fn __reinhardt_view_fields<Deps>(runtime: &#pages::UseFormReturn<Self, Deps>)
				-> ::std::vec::Vec<#support::ViewField<#field_token>>
			where Deps: ::core::clone::Clone + ::core::cmp::PartialEq + 'static {
				::std::vec![#(#descriptors),*]
			}

			#[doc(hidden)]
			pub fn __reinhardt_view<Deps, Output>(
				mutation: &#pages::FormServerMutation<Self, Deps, #dto, Output>,
				options: #support::ViewOptions,
			) -> #wrapper<Deps, Output>
			where Deps: ::core::clone::Clone + ::core::cmp::PartialEq + 'static,
				Output: ::core::clone::Clone + 'static {
				let id = options.resolve_id();
				let runtime = mutation.form();
				let fields = Self::__reinhardt_view_fields(&runtime);
				#wrapper { inner: #support::ClientFormView::new(mutation.clone(), runtime, fields, options, id) }
			}
		}
	}
}
