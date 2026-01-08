use crate::crate_paths::get_reinhardt_apps_crate;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result, parse_macro_input};

/// Configuration from `#[app_config(...)]` attribute
#[derive(Debug, Clone)]
struct AppConfigAttr {
	name: String,
	label: String,
	verbose_name: Option<String>,
}

impl AppConfigAttr {
	/// Parse `#[app_config(...)]` attribute
	fn from_attrs(attrs: &[syn::Attribute], struct_name: &syn::Ident) -> Result<Self> {
		let mut name = None;
		let mut label = None;
		let mut verbose_name = None;

		for attr in attrs {
			if !attr.path().is_ident("app_config") {
				continue;
			}

			attr.parse_nested_meta(|meta| {
				if meta.path.is_ident("name") {
					let value: syn::LitStr = meta.value()?.parse()?;
					name = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("label") {
					let value: syn::LitStr = meta.value()?.parse()?;
					label = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("verbose_name") {
					let value: syn::LitStr = meta.value()?.parse()?;
					verbose_name = Some(value.value());
					Ok(())
				} else {
					Err(meta.error("unsupported app_config attribute"))
				}
			})?;
		}

		let name = name.ok_or_else(|| {
			syn::Error::new_spanned(
				struct_name,
				"app_config attribute requires 'name' parameter",
			)
		})?;

		let label = label.ok_or_else(|| {
			syn::Error::new_spanned(
				struct_name,
				"app_config attribute requires 'label' parameter",
			)
		})?;

		Ok(Self {
			name,
			label,
			verbose_name,
		})
	}
}

/// Derive AppConfig implementation
pub(crate) fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	match derive_impl(input) {
		Ok(tokens) => tokens.into(),
		Err(err) => err.to_compile_error().into(),
	}
}

fn derive_impl(input: DeriveInput) -> Result<TokenStream> {
	let apps_crate = get_reinhardt_apps_crate();

	let struct_name = &input.ident;
	let config = AppConfigAttr::from_attrs(&input.attrs, struct_name)?;

	let name = &config.name;
	let label = &config.label;

	let config_builder = if let Some(verbose_name) = &config.verbose_name {
		quote! {
			#apps_crate::AppConfig::new(#name, #label)
				.with_verbose_name(#verbose_name)
		}
	} else {
		quote! {
			#apps_crate::AppConfig::new(#name, #label)
		}
	};

	let expanded = quote! {
		impl #struct_name {
			/// Create AppConfig instance
			pub fn config() -> #apps_crate::AppConfig {
				#config_builder
			}
		}
	};

	Ok(expanded)
}
