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
			let mut seen_first_non_option = false;
			for field in fields.named.iter_mut() {
				let is_option = is_option_type(&field.ty);
				let should_default = if is_option {
					true
				} else if !seen_first_non_option {
					seen_first_non_option = true;
					false
				} else {
					true
				};
				if should_default {
					let has_builder_default = field.attrs.iter().any(|a| {
						a.path().is_ident("builder")
							&& matches!(&a.meta, syn::Meta::List(l) if l.tokens.to_string().contains("default"))
					});
					if !has_builder_default {
						field.attrs.push(syn::parse_quote!(#[builder(default)]));
					}
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

fn replace_derive_default_with_bon_builder(attrs: &mut Vec<syn::Attribute>) {
	// Collect all derives from all #[derive(...)] attributes first,
	// so multiple derive attributes (e.g. #[derive(Clone)] #[derive(Default)])
	// are merged into a single combined attribute.
	let mut derives: Vec<syn::Path> = Vec::new();
	for a in attrs.iter() {
		if a.path().is_ident("derive") {
			let _ = a.parse_nested_meta(|m| {
				if !m.path.is_ident("Default") {
					derives.push(m.path.clone());
				}
				Ok(())
			});
		}
	}
	if !derives.iter().any(|p| {
		p.segments.len() == 2
			&& p.segments[0].ident == "bon"
			&& p.segments[1].ident == "Builder"
	}) {
		derives.push(syn::parse_quote!(bon::Builder));
	}

	let mut new_attrs: Vec<syn::Attribute> = attrs
		.iter()
		.filter(|a| !a.path().is_ident("derive"))
		.cloned()
		.collect();

	let insert_pos = attrs
		.iter()
		.position(|a| a.path().is_ident("derive"))
		.unwrap_or(0)
		.min(new_attrs.len());
	new_attrs.insert(insert_pos, syn::parse_quote!(#[derive(#(#derives),*)]));

	*attrs = new_attrs;
}

fn is_option_type(ty: &syn::Type) -> bool {
	if let syn::Type::Path(p) = ty
		&& let Some(seg) = p.path.segments.last()
	{
		return seg.ident == "Option";
	}
	false
}
