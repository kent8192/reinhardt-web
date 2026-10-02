//! Validation for low-level `server_fnset` declarations.

use syn::{Error, GenericArgument, Item, ItemFn, ItemImpl, PathArguments, Result, Type};

use super::parse::FnSetOptions;

pub(crate) struct ValidatedFnSet {
	pub options: FnSetOptions,
	pub function: ItemFn,
	pub resource: Option<Type>,
}

pub(crate) struct ValidatedImplSet {
	pub link: syn::Path,
	pub implementation: ItemImpl,
}

pub(crate) enum ValidatedSet {
	Function(Box<ValidatedFnSet>),
	Implementation(Box<ValidatedImplSet>),
}

pub(crate) fn validate(options: FnSetOptions, item: Item) -> Result<ValidatedSet> {
	if let Item::Impl(implementation) = item {
		if options.name.is_some() || options.actions.is_some() {
			return Err(Error::new_spanned(
				implementation.impl_token,
				"impl-form accepts only `for = function_name`",
			));
		}
		let link = options.link.ok_or_else(|| {
			Error::new_spanned(&implementation.self_ty, "missing required `for` option")
		})?;
		return Ok(ValidatedSet::Implementation(Box::new(ValidatedImplSet {
			link,
			implementation,
		})));
	}
	let Item::Fn(function) = item else {
		return Err(Error::new_spanned(
			item,
			"`server_fnset` can only be applied to a function or inherent impl",
		));
	};
	if let Some(link) = &options.link {
		return Err(Error::new_spanned(
			link,
			"`for` is only valid on impl-form server function sets",
		));
	}
	if options.name.is_none() {
		return Err(Error::new_spanned(
			&function.sig.ident,
			"missing required `name` option",
		));
	}
	if let Some(name) = options.name.as_ref()
		&& let Err(message) = validate_set_name(&name.value())
	{
		return Err(Error::new_spanned(name, message));
	}
	let resource = model_resource(&function);
	if let Some(actions) = options.actions.as_ref().filter(|_| resource.is_none()) {
		return Err(Error::new_spanned(
			actions,
			"`actions` is only supported by model server function sets",
		));
	}
	if resource.is_some() && !cfg!(feature = "model-server-fnset") {
		return Err(Error::new_spanned(
			&function.sig.output,
			"model server function sets require the `model-server-fnset` feature",
		));
	}

	Ok(ValidatedSet::Function(Box::new(ValidatedFnSet {
		options,
		function,
		resource,
	})))
}

fn validate_set_name(name: &str) -> std::result::Result<(), &'static str> {
	if name.is_empty() {
		return Err("server function set name must not be empty");
	}
	if !name
		.bytes()
		.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
	{
		return Err("server function set name may contain only ASCII letters, digits, '-' and '_'");
	}
	Ok(())
}

fn model_resource(function: &ItemFn) -> Option<Type> {
	let syn::ReturnType::Type(_, ty) = &function.sig.output else {
		return None;
	};
	let Type::Path(type_path) = ty.as_ref() else {
		return None;
	};
	let segment = type_path.path.segments.last()?;
	if segment.ident != "ModelServerFnSet" {
		return None;
	}
	let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
		return None;
	};
	match arguments.args.first()? {
		GenericArgument::Type(resource) => Some(resource.clone()),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::validate_set_name;

	#[test]
	fn accepts_safe_single_path_segments() {
		for name in ["article", "Article42", "article-api", "article_api"] {
			assert!(
				validate_set_name(name).is_ok(),
				"expected {name:?} to be valid"
			);
		}
	}

	#[test]
	fn rejects_unsafe_path_content() {
		for name in [
			"../admin",
			"article/admin",
			"article\\admin",
			"article admin",
			"article\tadmin",
			"article?preview=true",
			"article#preview",
			"{tenant}",
			"article%2fadmin",
			"article%3fadmin",
			"article:admin",
			"article*admin",
			"article\0admin",
			"記事",
		] {
			assert!(
				validate_set_name(name).is_err(),
				"expected {name:?} to be rejected"
			);
		}
	}
}
