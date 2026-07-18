//! Implementation of the `server_fnset` attribute macro.

mod codegen;
mod parse;
mod validate;

use proc_macro::TokenStream;
use syn::{Item, parse_macro_input};

use self::parse::FnSetOptions;

pub(crate) fn server_fnset_impl(args: TokenStream, input: TokenStream) -> TokenStream {
	let options = parse_macro_input!(args as FnSetOptions);
	let item = parse_macro_input!(input as Item);

	let validated = match validate::validate(options, item) {
		Ok(function) => function,
		Err(error) => return error.into_compile_error().into(),
	};

	codegen::expand(validated).into()
}
