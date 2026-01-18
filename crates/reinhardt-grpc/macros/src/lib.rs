//! Procedural macros for Reinhardt gRPC DI integration

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod crate_paths;
mod grpc_handler;

/// Attribute macro for gRPC handlers with dependency injection support
///
/// This macro enables the use of `#[inject]` parameters in gRPC service methods,
/// allowing automatic dependency resolution from the `InjectionContext`.
///
/// # Parameters
///
/// Regular parameters are passed through as-is. Parameters marked with `#[inject]`
/// are automatically resolved from the DI context.
///
/// # Requirements
///
/// 1. The function must have a `tonic::Request<T>` parameter
/// 2. The request must have an `InjectionContext` in its extensions
/// 3. All injected types must implement `Injectable`
/// 4. The function must be `async`
///
/// # Error Handling
///
/// If dependency injection fails, the function returns `tonic::Status::internal`
/// with an error message describing the failure.
#[proc_macro_attribute]
pub fn grpc_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let input = parse_macro_input!(item as syn::ItemFn);

	grpc_handler::expand_grpc_handler(input)
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}
