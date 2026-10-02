//! Attribute option parsing for `server_fnset`.

use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Error, LitStr, Path, Result, Token};

pub(crate) struct FnSetOptions {
	pub name: Option<LitStr>,
	pub actions: Option<Path>,
	pub link: Option<Path>,
}

impl Parse for FnSetOptions {
	fn parse(input: ParseStream<'_>) -> Result<Self> {
		let mut name = None;
		let mut actions = None;
		let mut link = None;

		while !input.is_empty() {
			let key = syn::Ident::parse_any(input)?;
			input.parse::<Token![=]>()?;

			if key == "name" {
				if name.is_some() {
					return Err(Error::new(key.span(), "duplicate `name` option"));
				}
				name = Some(input.parse()?);
			} else if key == "actions" {
				if actions.is_some() {
					return Err(Error::new(key.span(), "duplicate `actions` option"));
				}
				actions = Some(input.parse()?);
			} else if key == "for" {
				if link.is_some() {
					return Err(Error::new(key.span(), "duplicate `for` option"));
				}
				link = Some(input.parse()?);
			} else {
				return Err(Error::new(
					key.span(),
					"unknown server_fnset option; expected `name`, `actions`, or `for`",
				));
			}

			if input.is_empty() {
				break;
			}
			input.parse::<Token![,]>()?;
		}

		Ok(Self {
			name,
			actions,
			link,
		})
	}
}
