//! Schema attribute parsing and field metadata extraction

use syn::{Attribute, Lit, Meta, MetaNameValue};

/// Field-level schema attributes
#[derive(Debug, Default, Clone)]
pub(crate) struct FieldAttributes {
	pub description: Option<String>,
	pub example: Option<String>,
	pub format: Option<String>,
	pub default: bool,
	pub deprecated: bool,
	pub read_only: bool,
	pub write_only: bool,
	pub minimum: Option<i64>,
	pub maximum: Option<i64>,
	pub min_length: Option<usize>,
	pub max_length: Option<usize>,
	pub pattern: Option<String>,
	/// Property name override from `#[serde(rename = "...")]` or `#[schema(rename = "...")]`
	pub rename: Option<String>,
}

impl FieldAttributes {
	pub(crate) fn is_empty(&self) -> bool {
		self.description.is_none()
			&& self.example.is_none()
			&& self.format.is_none()
			&& !self.default
			&& !self.deprecated
			&& !self.read_only
			&& !self.write_only
			&& self.minimum.is_none()
			&& self.maximum.is_none()
			&& self.min_length.is_none()
			&& self.max_length.is_none()
			&& self.pattern.is_none()
			&& self.rename.is_none()
	}
}

/// Extract schema attributes from field attributes
pub(crate) fn extract_field_attributes(attrs: &[Attribute]) -> FieldAttributes {
	let mut field_attrs = FieldAttributes::default();

	for attr in attrs {
		// Check for doc comments (/// or //!)
		if attr.path().is_ident("doc") {
			if let Meta::NameValue(MetaNameValue {
				value: syn::Expr::Lit(syn::ExprLit {
					lit: Lit::Str(lit_str),
					..
				}),
				..
			}) = &attr.meta
			{
				let doc = lit_str.value().trim().to_string();
				if !doc.is_empty() {
					// Append to description if already exists
					if let Some(ref mut desc) = field_attrs.description {
						desc.push(' ');
						desc.push_str(&doc);
					} else {
						field_attrs.description = Some(doc);
					}
				}
			}
			continue;
		}

		// Check for #[serde(...)] attributes to extract rename
		if attr.path().is_ident("serde") {
			if let Ok(meta_list) = attr.meta.require_list() {
				for nested_meta in meta_list
					.parse_args_with(
						syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
					)
					.unwrap_or_default()
				{
					if let Meta::NameValue(nv) = nested_meta
						&& nv.path.is_ident("rename")
						&& let syn::Expr::Lit(syn::ExprLit {
							lit: Lit::Str(lit_str),
							..
						}) = nv.value
					{
						// Only set if not already set by #[schema(rename = "...")]
						if field_attrs.rename.is_none() {
							field_attrs.rename = Some(lit_str.value());
						}
					}
				}
			}
			continue;
		}

		// Check for #[schema(...)] attributes
		if !attr.path().is_ident("schema") {
			continue;
		}

		// Parse nested meta items
		if let Ok(meta_list) = attr.meta.require_list() {
			for nested_meta in meta_list
				.parse_args_with(
					syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
				)
				.unwrap_or_default()
			{
				match nested_meta {
					Meta::NameValue(nv) => {
						let ident = nv
							.path
							.get_ident()
							.expect("Expected identifier")
							.to_string();

						match ident.as_str() {
							"description" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Str(lit_str),
									..
								}) = nv.value
								{
									// Override doc comments if explicit description is provided
									field_attrs.description = Some(lit_str.value());
								}
							}
							"example" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Str(lit_str),
									..
								}) = nv.value
								{
									field_attrs.example = Some(lit_str.value());
								}
							}
							"format" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Str(lit_str),
									..
								}) = nv.value
								{
									field_attrs.format = Some(lit_str.value());
								}
							}
							"minimum" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Int(lit_int),
									..
								}) = nv.value
								{
									field_attrs.minimum = lit_int.base10_parse().ok();
								}
							}
							"maximum" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Int(lit_int),
									..
								}) = nv.value
								{
									field_attrs.maximum = lit_int.base10_parse().ok();
								}
							}
							"min_length" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Int(lit_int),
									..
								}) = nv.value
								{
									field_attrs.min_length = lit_int.base10_parse().ok();
								}
							}
							"max_length" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Int(lit_int),
									..
								}) = nv.value
								{
									field_attrs.max_length = lit_int.base10_parse().ok();
								}
							}
							"pattern" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Str(lit_str),
									..
								}) = nv.value
								{
									field_attrs.pattern = Some(lit_str.value());
								}
							}
							"rename" => {
								if let syn::Expr::Lit(syn::ExprLit {
									lit: Lit::Str(lit_str),
									..
								}) = nv.value
								{
									// Override serde rename if explicit schema rename is provided
									field_attrs.rename = Some(lit_str.value());
								}
							}
							_ => {}
						}
					}
					Meta::Path(path) => {
						if let Some(ident) = path.get_ident() {
							match ident.to_string().as_str() {
								"default" => field_attrs.default = true,
								"deprecated" => field_attrs.deprecated = true,
								"read_only" => field_attrs.read_only = true,
								"write_only" => field_attrs.write_only = true,
								_ => {}
							}
						}
					}
					_ => {}
				}
			}
		}
	}

	field_attrs
}
