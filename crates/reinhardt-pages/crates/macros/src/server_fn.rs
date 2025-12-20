//! Server Function Macro Implementation
//!
//! This module implements the `#[server_fn]` procedural macro for generating
//! client-side stubs (WASM) and server-side handlers (non-WASM).
//!
//! ## Architecture
//!
//! The macro performs conditional compilation:
//! - **WASM target**: Generates HTTP client stub
//! - **Non-WASM target**: Generates route handler
//!
//! ## Implementation Phases
//!
//! - Week 2 (Day 1-2): Basic infrastructure, option parsing
//! - Week 3 (Day 1-2): Client stub generation
//! - Week 3 (Day 3-4): Server handler generation
//! - Week 4 (Day 1-2): DI support (`use_inject = true`)

use convert_case::{Case, Casing};
use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{FnArg, ItemFn, Meta, Token, parse_macro_input};

/// Convert snake_case identifier to UpperCamelCase for struct naming
///
/// # Examples
///
/// ```ignore
/// use proc_macro2::Ident;
/// use quote::format_ident;
///
/// let ident = format_ident!("create_user");
/// let pascal = to_pascal_case_ident(&ident);
/// assert_eq!(pascal.to_string(), "CreateUser");
///
/// let ident = format_ident!("get_user_list");
/// let pascal = to_pascal_case_ident(&ident);
/// assert_eq!(pascal.to_string(), "GetUserList");
/// ```
fn to_pascal_case_ident(ident: &proc_macro2::Ident) -> proc_macro2::Ident {
	let pascal_name = ident.to_string().to_case(Case::Pascal);
	quote::format_ident!("{}", pascal_name)
}

/// Options for #[server_fn] macro
///
/// These options are parsed from the attribute arguments.
#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
pub struct ServerFnOptions {
	/// Enable DI functionality with `use_inject = true`
	///
	/// When enabled, parameters marked with `#[inject]` will be resolved
	/// via dependency injection on the server side.
	///
	/// # Example
	///
	/// ```ignore
	/// #[server_fn(use_inject = true)]
	/// async fn get_user(
	///     id: u32,
	///     #[inject] db: Database,
	/// ) -> Result<User, ServerFnError> {
	///     // db is injected automatically
	///     User::find_by_id(&db, id).await
	/// }
	/// ```
	pub use_inject: bool,

	/// Optional custom endpoint path
	///
	/// If not specified, defaults to `/api/server_fn/{function_name}`
	///
	/// # Example
	///
	/// ```ignore
	/// #[server_fn(endpoint = "/api/users/get")]
	/// async fn get_user(id: u32) -> Result<User, ServerFnError> {
	///     // ...
	/// }
	/// ```
	pub endpoint: Option<String>,

	/// Codec: "json" (default), "url", "msgpack"
	///
	/// Determines the serialization format for arguments and return values.
	///
	/// # Example
	///
	/// ```ignore
	/// #[server_fn(codec = "msgpack")]
	/// async fn upload_data(data: Vec<u8>) -> Result<(), ServerFnError> {
	///     // ...
	/// }
	/// ```
	#[darling(default = "default_codec")]
	pub codec: String,
}

fn default_codec() -> String {
	"json".to_string()
}

impl Default for ServerFnOptions {
	fn default() -> Self {
		Self {
			use_inject: false,
			endpoint: None,
			codec: default_codec(),
		}
	}
}

/// Information about #[inject] parameters (Week 4 Day 1-2)
///
/// This struct holds metadata about parameters that should be resolved
/// via dependency injection on the server side.
#[derive(Debug, Clone)]
struct InjectInfo {
	/// Parameter pattern (e.g., `db`, `auth`)
	pat: Box<syn::Pat>,
	/// Parameter type (e.g., `Database`, `AuthContext`)
	ty: Box<syn::Type>,
}

/// Check if an attribute is #[inject] or #[reinhardt::inject]
///
/// # Examples
///
/// ```ignore
/// #[inject] db: Database              // Legacy (causes compiler errors on params)
/// #[reinhardt::inject] db: Database   // Recommended tool attribute
/// ```
fn is_inject_attr(attr: &syn::Attribute) -> bool {
	// Check for bare #[inject] (legacy, causes compiler errors on function params)
	if attr.path().is_ident("inject") {
		return true;
	}

	// Check for #[reinhardt::inject] (recommended tool attribute)
	if let Some(seg0) = attr.path().segments.first()
		&& seg0.ident == "reinhardt"
			&& let Some(seg1) = attr.path().segments.iter().nth(1) {
				return seg1.ident == "inject";
			}

	false
}

/// Detect parameters for dependency injection (Week 4 Day 1-2)
///
/// This function scans function parameters and identifies those that should be
/// injected by the DI system. Detection is based on:
/// 1. Parameters with #[inject] or #[reinhardt::inject] attributes
/// 2. Parameters with Arc<T> type (automatic detection)
///
/// # Examples
///
/// ```ignore
/// async fn handler(
///     id: u32,                    // Regular parameter
///     #[inject] db: Database,     // DI parameter (explicit)
///     site: Arc<AdminSite>,       // DI parameter (auto-detected Arc<T>)
/// ) -> Result<User, Error>
/// ```
fn detect_inject_params(inputs: &Punctuated<FnArg, Token![,]>) -> Vec<InjectInfo> {
	let mut inject_params = Vec::new();

	for input in inputs {
		if let FnArg::Typed(pat_type) = input {
			// Check for explicit #[inject] attribute
			let has_inject_attr = pat_type.attrs.iter().any(is_inject_attr);

			// Check if type is Arc<T> (auto-detect for DI)
			let is_arc_type = if let syn::Type::Path(type_path) = pat_type.ty.as_ref() {
				type_path
					.path
					.segments
					.last()
					.map(|seg| seg.ident == "Arc")
					.unwrap_or(false)
			} else {
				false
			};

			if has_inject_attr || is_arc_type {
				inject_params.push(InjectInfo {
					pat: pat_type.pat.clone(),
					ty: pat_type.ty.clone(),
				});
			}
		}
	}

	inject_params
}

/// Remove #[inject] attributes from function parameters (Week 4 Day 1-2)
///
/// This creates a clean version of the function for server-side compilation.
/// Pattern copied from reinhardt-core/crates/macros/src/routes.rs.
///
/// # Example
///
/// Input:
/// ```ignore
/// async fn handler(id: u32, #[inject] db: Database) -> Result<User, Error>
/// ```
///
/// Output:
/// ```ignore
/// async fn handler(id: u32, db: Database) -> Result<User, Error>
/// ```
fn remove_inject_attrs(func: &ItemFn) -> ItemFn {
	let mut func = func.clone();

	// Remove #[inject] attributes from parameters
	func.sig.inputs = func
		.sig
		.inputs
		.iter()
		.map(|arg| {
			if let FnArg::Typed(pat_type) = arg {
				let mut pat_type = pat_type.clone();
				pat_type.attrs.retain(|attr| !is_inject_attr(attr));
				FnArg::Typed(pat_type)
			} else {
				arg.clone()
			}
		})
		.collect();

	func
}

/// Server function metadata
///
/// This struct holds all the information extracted from the function signature.
struct ServerFnInfo {
	/// Original function
	func: ItemFn,
	/// Parsed options
	options: ServerFnOptions,
}

impl ServerFnInfo {
	/// Parse from macro input
	fn parse(args: Vec<Meta>, func: ItemFn) -> Result<Self, darling::Error> {
		// Convert Meta to NestedMeta for darling compatibility
		let nested: Vec<NestedMeta> = args.into_iter().map(NestedMeta::Meta).collect();
		let options = ServerFnOptions::from_list(&nested)?;

		Ok(Self { func, options })
	}

	/// Get the function name
	fn name(&self) -> &syn::Ident {
		&self.func.sig.ident
	}

	/// Get the function visibility
	fn vis(&self) -> &syn::Visibility {
		&self.func.vis
	}

	/// Get the endpoint path
	///
	/// Returns the custom endpoint if specified, otherwise generates default.
	fn endpoint(&self) -> String {
		self.options
			.endpoint
			.clone()
			.unwrap_or_else(|| format!("/api/server_fn/{}", self.name()))
	}

	/// Get the codec name
	fn codec(&self) -> &str {
		&self.options.codec
	}

	/// Check if DI is enabled
	fn use_inject(&self) -> bool {
		self.options.use_inject
	}
}

/// Main entry point for #[server_fn] macro
pub fn server_fn_impl(args: TokenStream, input: TokenStream) -> TokenStream {
	// Parse attribute arguments
	let attr_args = match syn::parse::Parser::parse(
		syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
		args.clone(),
	) {
		Ok(args) => args.into_iter().collect(),
		Err(e) => return e.to_compile_error().into(),
	};

	// Parse function
	let func = parse_macro_input!(input as ItemFn);

	// Parse metadata
	let info = match ServerFnInfo::parse(attr_args, func) {
		Ok(info) => info,
		Err(e) => return e.write_errors().into(),
	};

	// Generate code (stub for now, full implementation in Week 3)
	generate_server_fn(&info).into()
}

/// Generate server function code
///
/// This generates both client and server code with conditional compilation.
///
/// # Implementation Status
///
/// - Week 2: Basic structure (stubs)
/// - Week 3 Day 1-2: Client stub generation
/// - Week 3 Day 3-4: Server handler generation
/// - Week 4 Day 1-2: DI parameter detection ← CURRENT
fn generate_server_fn(info: &ServerFnInfo) -> proc_macro2::TokenStream {
	let func = &info.func;

	// Week 4 Day 1-2: Detect #[inject] parameters if use_inject is enabled
	let inject_params = if info.use_inject() {
		detect_inject_params(&func.sig.inputs)
	} else {
		Vec::new()
	};

	// Week 4 Day 1-2: Remove #[inject] attributes from original function
	// This ensures the server-side code compiles without unknown attributes
	let clean_func = if info.use_inject() && !inject_params.is_empty() {
		remove_inject_attrs(func)
	} else {
		func.clone()
	};

	// Week 3 Day 1-2: Generate client stub (with DI parameter filtering)
	let client_stub = generate_client_stub(info, &inject_params);

	// Week 3 Day 3-4: Generate server handler (with DI resolution)
	let server_handler = generate_server_handler(info, &inject_params);

	quote! {
		// Server-side: Original function (with #[inject] attributes removed)
		#[cfg(not(target_arch = "wasm32"))]
		#clean_func

		// Client-side: HTTP request stub
		#client_stub

		// Server-side: Route handler and registration
		#server_handler
	}
}

/// Generate client-side HTTP request stub (Week 3 Day 1-2)
///
/// This generates an async function that:
/// 1. Serializes function arguments to JSON
/// 2. Sends HTTP POST request to the endpoint
/// 3. Deserializes the response
///
/// Example expansion:
/// ```ignore
/// // Input:
/// #[server_fn]
/// async fn get_user(id: u32) -> Result<User, ServerFnError> { ... }
///
/// // Expands to (on WASM):
/// pub async fn get_user(id: u32) -> Result<User, ServerFnError> {
///     #[derive(serde::Serialize)]
///     struct Args { id: u32 }
///
///     let url = "/api/server_fn/get_user";
///     let args = Args { id };
///     let response = gloo_net::http::Request::post(url)
///         .json(&args)?
///         .send()
///         .await?;
///     response.json().await
/// }
/// ```
fn generate_client_stub(
	info: &ServerFnInfo,
	_inject_params: &[InjectInfo],
) -> proc_macro2::TokenStream {
	let name = info.name();
	let vis = info.vis();
	let endpoint = info.endpoint();
	let codec = info.codec();
	let func = &info.func;
	let sig = &func.sig;

	// Extract function parameters, excluding #[inject] parameters
	// Client-side doesn't need DI parameters - they're resolved on the server
	let params: Vec<_> = sig
		.inputs
		.iter()
		.filter_map(|arg| {
			if let syn::FnArg::Typed(pat_type) = arg {
				// Skip #[inject] parameters
				let has_inject = pat_type.attrs.iter().any(is_inject_attr);
				if !has_inject { Some(pat_type) } else { None }
			} else {
				None
			}
		})
		.collect();

	// Extract parameter names and types for Args struct
	let param_names: Vec<_> = params.iter().map(|p| &p.pat).collect();

	let param_types: Vec<_> = params.iter().map(|p| &p.ty).collect();

	// Generate unique struct name to avoid conflicts
	let args_struct_name = {
		let pascal_name = to_pascal_case_ident(name);
		quote::format_ident!("{}Args", pascal_name)
	};

	// Generate codec-specific serialization and deserialization code
	let (content_type, serialize_code, deserialize_code) = match codec {
		"json" => (
			"application/json",
			quote! {
				let __body = ::serde_json::to_string(&__args)
					.map_err(|e| crate::server_fn::ServerFnError::serialization(e.to_string()))?;
			},
			quote! {
				__response
					.json()
					.await
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))
			},
		),
		"url" => (
			"application/x-www-form-urlencoded",
			quote! {
				let __body = ::serde_urlencoded::to_string(&__args)
					.map_err(|e| crate::server_fn::ServerFnError::serialization(e.to_string()))?;
			},
			quote! {
				let __text = __response.text().await
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))?;
				::serde_json::from_str(&__text)
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))
			},
		),
		"msgpack" => (
			"application/msgpack",
			quote! {
				let __body_bytes = ::rmp_serde::to_vec(&__args)
					.map_err(|e| crate::server_fn::ServerFnError::serialization(e.to_string()))?;
				// Convert to base64 for transport over HTTP text body
				let __body = ::base64::Engine::encode(&::base64::engine::general_purpose::STANDARD, &__body_bytes);
			},
			quote! {
				let __text = __response.text().await
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))?;
				let __bytes = ::base64::Engine::decode(&::base64::engine::general_purpose::STANDARD, &__text)
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))?;
				::rmp_serde::from_slice(&__bytes)
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))
			},
		),
		// Default to json for unknown codecs
		_ => (
			"application/json",
			quote! {
				let __body = ::serde_json::to_string(&__args)
					.map_err(|e| crate::server_fn::ServerFnError::serialization(e.to_string()))?;
			},
			quote! {
				__response
					.json()
					.await
					.map_err(|e| crate::server_fn::ServerFnError::deserialization(e.to_string()))
			},
		),
	};

	quote! {
		#[cfg(target_arch = "wasm32")]
		#vis #sig {
			use ::serde::{Serialize, Deserialize};

			// Argument struct for serialization
			#[derive(Serialize)]
			struct #args_struct_name {
				#(#param_names: #param_types),*
			}

			let __endpoint = #endpoint;
			let __args = #args_struct_name {
				#(#param_names),*
			};

			// Serialize arguments based on codec
			#serialize_code

			// Send HTTP POST request using gloo-net
			let __response = ::gloo_net::http::Request::post(__endpoint)
				.header("Content-Type", #content_type)
				.body(__body)
				.send()
				.await
				.map_err(|e| crate::server_fn::ServerFnError::network(e.to_string()))?;

			// Check HTTP status
			if !__response.ok() {
				let __status = __response.status();
				let __message = __response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
				return Err(crate::server_fn::ServerFnError::server(__status, __message));
			}

			// Deserialize response based on codec
			#deserialize_code
		}
	}
}

/// Generate server handler and registration function (Week 3 Day 3-4)
///
/// This generates a route handler that:
/// 1. Deserializes JSON request body to function arguments
/// 2. Calls the original server function
/// 3. Serializes the response to JSON
/// 4. Handles errors appropriately
///
/// Example expansion:
/// ```ignore
/// // Input:
/// #[server_fn]
/// async fn get_user(id: u32) -> Result<User, ServerFnError> { ... }
///
/// // Expands to (on server):
/// pub async fn __server_fn_handler_get_user(
///     body: String,
/// ) -> Result<String, String> {
///     #[derive(serde::Deserialize)]
///     struct Args { id: u32 }
///
///     let args: Args = serde_json::from_str(&body)?;
///     let result = get_user(args.id).await?;
///     Ok(serde_json::to_string(&result)?)
/// }
///
/// pub fn register_server_fn_get_user<R>(router: R) -> R
/// where
///     R: ServerFnRouter,
/// {
///     router.post("/api/server_fn/get_user", __server_fn_handler_get_user)
/// }
/// ```
fn generate_server_handler(
	info: &ServerFnInfo,
	inject_params: &[InjectInfo],
) -> proc_macro2::TokenStream {
	let name = info.name();
	let endpoint = info.endpoint();
	let codec = info.codec();
	let func = &info.func;
	let sig = &func.sig;

	// Extract function parameters, separating regular and #[inject] parameters
	let regular_params: Vec<_> = sig
		.inputs
		.iter()
		.filter_map(|arg| {
			if let syn::FnArg::Typed(pat_type) = arg {
				let has_inject = pat_type.attrs.iter().any(is_inject_attr);
				if !has_inject { Some(pat_type) } else { None }
			} else {
				None
			}
		})
		.collect();

	let regular_param_names: Vec<_> = regular_params.iter().map(|p| &p.pat).collect();
	let regular_param_types: Vec<_> = regular_params.iter().map(|p| &p.ty).collect();

	// Extract inject parameter names and types
	let inject_param_names: Vec<_> = inject_params.iter().map(|p| &p.pat).collect();
	let inject_param_types: Vec<_> = inject_params.iter().map(|p| &p.ty).collect();

	// Generate unique names to avoid conflicts
	let handler_name = quote::format_ident!("__server_fn_handler_{}", name);
	let register_fn_name = quote::format_ident!("register_server_fn_{}", name);
	let args_struct_name = {
		let pascal_name = to_pascal_case_ident(name);
		quote::format_ident!("{}Args", pascal_name)
	};

	// Extract return type inner (T from Result<T, E>)
	// We assume the return type is Result<T, ServerFnError>
	let return_type = match &sig.output {
		syn::ReturnType::Type(_, ty) => ty,
		syn::ReturnType::Default => {
			return quote! {
				compile_error!("Server functions must return Result<T, ServerFnError>");
			};
		}
	};

	// Generate DI resolution code (Week 4 Day 4)
	// Pattern copied from reinhardt-core/crates/macros/src/use_inject.rs
	let di_resolution = if !inject_params.is_empty() {
		quote! {
			// Get DI context from request
			let __di_ctx = __req.get_di_context::<::std::sync::Arc<::reinhardt_di::InjectionContext>>()
				.ok_or_else(|| "DI context not set. Ensure the router is configured with .with_di_context()".to_string())?;

			// Resolve each #[inject] parameter using reinhardt_di::Injected<T>
			#(
				let #inject_param_names: #inject_param_types =
					::reinhardt_di::Injected::<#inject_param_types>::resolve(&__di_ctx)
						.await
						.map_err(|e| format!("Dependency injection failed for {}: {:?}", stringify!(#inject_param_types), e))?
						.into_inner();
			)*
		}
	} else {
		quote! {}
	};

	// Build function call with both regular and inject parameters
	let function_call_params = if inject_params.is_empty() {
		quote! {
			#(args.#regular_param_names),*
		}
	} else {
		quote! {
			#(args.#regular_param_names,)*
			#(#inject_param_names),*
		}
	};

	// Generate codec-specific deserialization code for server
	let deserialize_code = match codec {
		"json" => quote! {
			let args: #args_struct_name = ::serde_json::from_str(&body)
				.map_err(|e| format!("Failed to deserialize arguments: {}", e))?;
		},
		"url" => quote! {
			let args: #args_struct_name = ::serde_urlencoded::from_str(&body)
				.map_err(|e| format!("Failed to deserialize arguments: {}", e))?;
		},
		"msgpack" => quote! {
			// Decode base64 to bytes
			let bytes = ::base64::Engine::decode(&::base64::engine::general_purpose::STANDARD, &body)
				.map_err(|e| format!("Failed to decode base64: {}", e))?;
			// Deserialize from msgpack bytes
			let args: #args_struct_name = ::rmp_serde::from_slice(&bytes)
				.map_err(|e| format!("Failed to deserialize arguments: {}", e))?;
		},
		// Default to json for unknown codecs
		_ => quote! {
			let args: #args_struct_name = ::serde_json::from_str(&body)
				.map_err(|e| format!("Failed to deserialize arguments: {}", e))?;
		},
	};

	// Generate codec-specific serialization code for server response
	let serialize_response_code = match codec {
		"json" => quote! {
			::serde_json::to_string(&value)
				.map_err(|e| format!("Failed to serialize response: {}", e))
		},
		"url" => quote! {
			// For URL-encoded codec, response is still JSON
			::serde_json::to_string(&value)
				.map_err(|e| format!("Failed to serialize response: {}", e))
		},
		"msgpack" => quote! {
			// Serialize to msgpack bytes
			let bytes = ::rmp_serde::to_vec(&value)
				.map_err(|e| format!("Failed to serialize response: {}", e))?;
			// Encode as base64 for HTTP transport
			Ok(::base64::Engine::encode(&::base64::engine::general_purpose::STANDARD, &bytes))
		},
		// Default to json for unknown codecs
		_ => quote! {
			::serde_json::to_string(&value)
				.map_err(|e| format!("Failed to serialize response: {}", e))
		},
	};

	// Generate handler signature based on whether DI is needed
	let (handler_signature, body_extraction) = if !inject_params.is_empty() {
		// When we have inject params, handler receives Request to extract DI context
		(
			quote! {
				pub async fn #handler_name(__req: ::reinhardt_http::Request) -> ::std::result::Result<::std::string::String, ::std::string::String>
			},
			quote! {
				// Extract body from request
				let body = __req.read_body()
					.map_err(|e| format!("Failed to read body: {}", e))?;
				let body = ::std::string::String::from_utf8(body.to_vec())
					.map_err(|e| format!("Body is not valid UTF-8: {}", e))?;
			},
		)
	} else {
		// No DI needed, handler receives body directly
		(
			quote! {
				pub async fn #handler_name(body: ::std::string::String) -> ::std::result::Result<::std::string::String, ::std::string::String>
			},
			quote! {},
		)
	};

	quote! {
		#[cfg(not(target_arch = "wasm32"))]
		/// Server-side handler function
		///
		/// This function is called by the router when the endpoint receives a request.
		/// It deserializes the request body, calls the server function, and serializes the response.
		#handler_signature {
			use ::serde::{Deserialize, Serialize};

			// Argument struct for deserialization (only regular parameters)
			#[derive(Deserialize)]
			struct #args_struct_name {
				#(#regular_param_names: #regular_param_types),*
			}

			// Extract body if needed (when using DI)
			#body_extraction

			// Deserialize request body based on codec
			#deserialize_code

			// Resolve #[inject] parameters via DI (Week 4 Day 4)
			#di_resolution

			// Call the original server function with both regular and injected parameters
			let result: #return_type = #name(#function_call_params).await;

			// Handle Result and serialize
			match result {
				Ok(value) => {
					#serialize_response_code
				}
				Err(e) => {
					// Serialize the error as ServerFnError
					let error_json = ::serde_json::to_string(&e)
						.map_err(|e| format!("Failed to serialize error: {}", e))?;
					Err(error_json)
				}
			}
		}

		#[cfg(not(target_arch = "wasm32"))]
		/// Register this server function with a router
		///
		/// This function should be called during application startup to register
		/// the server function handler with the HTTP router.
		///
		/// # Example
		///
		/// ```ignore
		/// use axum::{Router, routing::post};
		///
		/// let app = Router::new()
		///     .route("/api/server_fn/get_user", post(register_server_fn_get_user));
		/// ```
		pub fn #register_fn_name() -> &'static str {
			#endpoint
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_server_fn_options_default() {
		let options = ServerFnOptions::default();
		assert_eq!(options.use_inject, false);
		assert_eq!(options.endpoint, None);
		assert_eq!(options.codec, "json");
	}

	#[test]
	fn test_server_fn_options_parse() {
		use darling::FromMeta;
		use darling::ast::NestedMeta;
		use syn::parse_quote;

		let attr: syn::Attribute =
			parse_quote!(#[server_fn(use_inject = true, endpoint = "/custom")]);
		let meta_list = attr.meta.require_list().unwrap();
		let nested: Vec<NestedMeta> = NestedMeta::parse_meta_list(meta_list.tokens.clone())
			.unwrap()
			.into_iter()
			.collect();
		let options = ServerFnOptions::from_list(&nested).unwrap();

		assert_eq!(options.use_inject, true);
		assert_eq!(options.endpoint, Some("/custom".to_string()));
		assert_eq!(options.codec, "json");
	}
}
