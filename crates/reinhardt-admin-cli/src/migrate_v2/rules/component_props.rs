//! Rule §6.2: migrate `#[derive(Default)] struct *Props` to
//! `#[derive(bon::Builder)]` with `#[builder(default)]` on optional fields.
//!
//! Heuristics (matches the spec §6.2 "Mechanical" strategy):
//!
//! - Target: structs whose ident ends in `Props` and carry
//!   `#[derive(Default)]`.
//! - For each field whose type is `Option<...>`, attach
//!   `#[builder(default)]` — these are always optional.
//! - The first non-`Option` field stays required (no `#[builder(default)]`)
//!   so `Card { item: x }` keeps working out of the box. Every other
//!   non-`Option` field gets `#[builder(default)]`. The developer can
//!   promote any of those back to required by deleting the attribute
//!   during review.

use syn::visit_mut::{self, VisitMut};

use crate::migrate_v2::rewriter::FileRewriter;

/// `component_props` rule entry.
pub struct Rule;

impl FileRewriter for Rule {
	fn name(&self) -> &'static str {
		"component_props"
	}

	fn rewrite(&self, mut file: syn::File) -> syn::File {
		StructVisitor.visit_file_mut(&mut file);
		file
	}
}

struct StructVisitor;

impl VisitMut for StructVisitor {
	fn visit_item_struct_mut(&mut self, s: &mut syn::ItemStruct) {
		let is_props = s.ident.to_string().ends_with("Props");
		if !is_props {
			visit_mut::visit_item_struct_mut(self, s);
			return;
		}
		if !has_derive_default(&s.attrs) {
			visit_mut::visit_item_struct_mut(self, s);
			return;
		}

		replace_derive_default_with_bon_builder(&mut s.attrs);

		if let syn::Fields::Named(fields) = &mut s.fields {
			for (idx, field) in fields.named.iter_mut().enumerate() {
				let is_option = is_option_type(&field.ty);
				let is_first = idx == 0;
				if !is_first || is_option {
					field.attrs.push(syn::parse_quote!(#[builder(default)]));
				}
			}
		}

		visit_mut::visit_item_struct_mut(self, s);
	}
}

fn has_derive_default(attrs: &[syn::Attribute]) -> bool {
	attrs.iter().any(|a| {
		a.path().is_ident("derive") && {
			let mut found = false;
			let _ = a.parse_nested_meta(|m| {
				if m.path.is_ident("Default") {
					found = true;
				}
				Ok(())
			});
			found
		}
	})
}

fn replace_derive_default_with_bon_builder(attrs: &mut [syn::Attribute]) {
	for a in attrs.iter_mut() {
		if a.path().is_ident("derive") {
			*a = syn::parse_quote!(#[derive(bon::Builder)]);
		}
	}
}

fn is_option_type(ty: &syn::Type) -> bool {
	if let syn::Type::Path(p) = ty
		&& let Some(seg) = p.path.segments.last()
	{
		return seg.ident == "Option";
	}
	false
}
