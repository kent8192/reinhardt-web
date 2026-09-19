//! Additive update-only payload and native patch generation.

use super::*;

pub(super) fn payload_patch(
	model: &Ident,
	fields: &[FieldInfo],
	config: &ModelFormConfig,
	selected: Option<&[Ident]>,
) -> TokenStream {
	let core = get_reinhardt_core_crate();
	let forms = get_reinhardt_forms_crate();
	let native_cfg = if forms.is_some() {
		quote!(#[cfg(not(all(target_family = "wasm", target_os = "unknown")))])
	} else {
		quote!(#[cfg(any())])
	};
	let enabled = if forms.is_some() {
		quote!()
	} else {
		quote!(#[cfg(all(target_family = "wasm", target_os = "unknown"))])
	};
	let forms = forms.unwrap_or_else(|| quote!(::reinhardt_forms));
	let payload = quote::format_ident!("{}ModelFormData", model);
	let cleaned = quote::format_ident!("Cleaned{}ModelFormData", model);
	let schema = quote::format_ident!("{}FormSchema", model);
	let editable: Vec<_> = fields
		.iter()
		.filter(|field| {
			is_model_form_editable(field, fields)
				&& selected.is_none_or(|names| names.contains(&field.name))
		})
		.collect();
	let names: Vec<_> = editable.iter().map(|field| &field.name).collect();
	let primary_keys = editable.iter().filter(|field| field.config.primary_key).map(|field| {
		let name = &field.name;
		let wire = ident_to_wire_name(name);
		quote! {
			if self.#name.is_some() {
				let mut errors = #core::validators::ValidationErrors::new();
				errors.add(#wire, #core::validators::ValidationError::Custom("Primary keys cannot be patched".to_owned()));
				return ::core::result::Result::Err(errors.into());
			}
		}
	});
	let needs_context = config.validate.is_some();
	let validate_context = config.validate.as_ref().map(|validator| {
		quote! {
			if let ::core::option::Option::Some(existing) = existing {
				let mut merged = cleaned.clone();
				#(
					if merged.#names.is_none() {
						merged.#names = existing.#names.clone();
					}
				)*
				#validator(&merged)?;
			}
		}
	});
	quote! {
		#native_cfg
		impl<P: #core::model_form::ModelFormPolicy> #payload<P> {
			fn __reinhardt_clean_patch(mut self) -> ::core::result::Result<#cleaned<P>, #core::validators::ValidationErrors> {
				#forms::model_form::clean_generated_patch_payload::<#schema, P, _>(&mut self)?;
				::core::result::Result::Ok(#cleaned::from_validated_raw(self))
			}
		}
		#[cfg(all(target_family = "wasm", target_os = "unknown"))]
		impl<P: #core::model_form::ModelFormPolicy> #payload<P> {
			fn __reinhardt_clean_patch(self) -> ::core::result::Result<#cleaned<P>, #core::validators::ValidationErrors> {
				self.__reinhardt_clean_and_validate(false, false, &[], ::core::option::Option::None, true)
			}
		}
		#enabled
		impl<P: #core::model_form::ModelFormPolicy> #core::model_form::ModelFormPatchPayload for #payload<P> {
			type Cleaned = #cleaned<P>;
			type Context = Self;
			fn clean_and_validate_patch(self, existing: ::core::option::Option<&Self>) -> ::core::result::Result<Self::Cleaned, #core::model_form::PatchValidationError> {
				if !self.__reinhardt_defaulted_fields.is_empty() {
					return ::core::result::Result::Err(#core::model_form::PatchValidationError::DefaultedValues);
				}
				#(#primary_keys)*
				if #needs_context && existing.is_none() {
					return ::core::result::Result::Err(#core::model_form::PatchValidationError::ExistingValuesRequired);
				}
				let cleaned = self.__reinhardt_clean_patch()?;
				#validate_context
				::core::result::Result::Ok(cleaned)
			}
		}
	}
}
