//! # Reinhardt Procedural Macros
//!
//! Provides Django-style decorators as Rust procedural macros.
//!
//! ## Macros
//!
//! - `#[api_view]` - Convert function to API view
//! - `#[action]` - Define custom ViewSet action
//! - `#[get]`, `#[post]`, etc. - HTTP method decorators
//! - `#[permission_required]` - Permission decorator
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_macros::api_view;
//!
//! #[api_view(methods = ["GET", "POST"])]
//! async fn my_view(request: Request) -> Result<Response> {
//!     Ok(Response::ok())
//! }
//! ```

use proc_macro::TokenStream;
use syn::{ItemFn, parse_macro_input};

mod action;
mod api_view;
mod endpoint;
mod injectable_derive;
mod installed_apps;
mod model_derive;
mod path_macro;
mod permission_macro;
mod permissions;
mod query_fields;
mod receiver;
mod routes;
mod schema;

use action::action_impl;
use api_view::api_view_impl;
use endpoint::endpoint_impl;
use injectable_derive::injectable_derive_impl;
use installed_apps::installed_apps_impl;
use model_derive::model_derive_impl;
use path_macro::path_impl;
use permissions::permission_required_impl;
use query_fields::derive_query_fields_impl;
use receiver::receiver_impl;
use routes::{delete_impl, get_impl, patch_impl, post_impl, put_impl};
use schema::derive_schema_impl;

/// Decorator for function-based API views
///
/// # Example
///
/// ```ignore
/// #[api_view(methods = ["GET", "POST"])]
/// async fn my_view(request: Request) -> Result<Response> {
///     match request.method.as_str() {
///         "GET" => Ok(Response::json(&data)?),
///         "POST" => Ok(Response::created()),
///         _ => Ok(Response::method_not_allowed()),
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn api_view(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	api_view_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Decorator for ViewSet custom actions
///
/// # Example
///
/// ```ignore
/// impl MyViewSet {
///     #[action(methods = ["POST"], detail = true)]
///     async fn activate(&self, request: Request, pk: i64) -> Result<Response> {
///         // Custom action implementation
///         Ok(Response::ok())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn action(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	action_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// GET method decorator
///
/// # Example
///
/// ```ignore
/// #[get("/users")]
/// async fn list_users(request: Request) -> Result<Response> {
///     Ok(Response::json(&users)?)
/// }
/// ```
#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	get_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// POST method decorator
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	post_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// PUT method decorator
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	put_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// PATCH method decorator
#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	patch_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// DELETE method decorator
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	delete_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Permission required decorator
///
/// # Example
///
/// ```ignore
/// #[permission_required("users.view_user")]
/// async fn view_user(request: Request) -> Result<Response> {
///     Ok(Response::ok())
/// }
/// ```
#[proc_macro_attribute]
pub fn permission_required(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	permission_required_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Define installed applications with compile-time validation
///
/// This macro creates a type-safe list of installed applications and validates
/// that referenced applications exist at compile time.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::installed_apps;
///
/// installed_apps! {
///     auth: "reinhardt.contrib.auth",
///     contenttypes: "reinhardt.contrib.contenttypes",
///     sessions: "reinhardt.contrib.sessions",
///     myapp: "apps.myapp",
/// }
///
// Use in settings
/// let apps = InstalledApp::all_apps();
/// ```
///
/// # Compile-time Validation
///
/// The macro will fail to compile if:
/// - A referenced `reinhardt.contrib.*` module doesn't exist
/// - The app path syntax is invalid
///
#[proc_macro]
pub fn installed_apps(input: TokenStream) -> TokenStream {
	installed_apps_impl(input.into())
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Validate URL patterns at compile time
///
/// This macro validates URL pattern syntax at compile time, catching common errors
/// before they reach runtime. It supports both simple parameters and Django-style
/// typed parameters.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::path;
///
// Simple parameter
/// let pattern = path!("polls/{id}/");
///
// Typed parameter (Django-style)
/// let pattern = path!("polls/{<int:question_id>}/");
///
// Multiple parameters
/// let pattern = path!("users/{user_id}/posts/{post_id}/");
/// ```
///
/// # Compile-time Validation
///
/// The macro will fail to compile if:
/// - Braces are not properly matched (e.g., `{id` or `id}`)
/// - Parameter names are empty (e.g., `{}`)
/// - Parameter names contain invalid characters
/// - Type specifiers are invalid (valid: `int`, `str`, `uuid`, `slug`, `path`)
/// - Django-style parameters are used outside braces (e.g., `<int:id>` instead of `{<int:id>}`)
///
/// # Supported Type Specifiers
///
/// - `int` - Integer values
/// - `str` - String values
/// - `uuid` - UUID values
/// - `slug` - Slug strings (alphanumeric, hyphens, underscores)
/// - `path` - Path segments (can include slashes)
///
#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
	path_impl(input.into())
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Connect a receiver function to a signal automatically
///
/// This macro provides Django-style `@receiver` decorator functionality for Rust.
/// It automatically registers the function as a signal receiver at startup.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::receiver;
/// use reinhardt_signals::post_save;
///
/// #[receiver(signal = post_save::<User>())]
/// async fn on_user_saved(instance: Arc<User>) -> Result<(), SignalError> {
///     println!("User saved: {:?}", instance);
///     Ok(())
/// }
/// ```
///
/// # With Sender Filtering
///
/// ```ignore
/// #[receiver(signal = post_save::<Article>(), sender = "Blog")]
/// async fn on_blog_article_saved(instance: Arc<Article>) -> Result<(), SignalError> {
///     println!("Blog article saved");
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn receiver(args: TokenStream, input: TokenStream) -> TokenStream {
	receiver_impl(args, input)
}

/// Automatic dependency injection macro
///
/// This macro enables FastAPI-style dependency injection using parameter attributes.
/// Parameters marked with `#[inject]` will be automatically resolved from the
/// `InjectionContext`. Can be used with any function, not just endpoints.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::use_injection;
/// use reinhardt_di::{Injectable, InjectionContext};
///
/// #[derive(Clone, Default)]
/// struct Database;
///
/// #[derive(Clone, Default)]
/// struct Config;
///
/// #[use_injection]
/// async fn my_handler(
///     #[inject] db: Database,
///     #[inject] config: Config,
///     regular_param: String,
/// ) -> Result<String> {
///     // db and config are automatically injected
///     Ok(format!("Handler with db and config"))
/// }
///
// Works with any function
/// #[use_injection]
/// async fn process_data(
///     #[inject] db: Database,
///     data: Vec<u8>,
/// ) -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// # Cache Control
///
/// You can disable caching for specific dependencies:
///
/// ```ignore
/// #[use_injection]
/// async fn handler(
///     #[inject] db: Database,              // Cached (default)
///     #[inject(cache = false)] fresh: Data,  // Not cached
/// ) -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// # Generated Code
///
/// The macro transforms the function by:
/// 1. Removing `#[inject]` parameters from the signature
/// 2. Adding an `InjectionContext` parameter
/// 3. Injecting dependencies at the start of the function
///
#[proc_macro_attribute]
pub fn use_injection(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	endpoint_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Alias for `use_injection` for FastAPI-style naming
///
/// See [`use_injection`] for documentation.
#[proc_macro_attribute]
pub fn endpoint(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	endpoint_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Derive macro for type-safe field lookups
///
/// Automatically generates field accessor methods for models, enabling
/// compile-time validated field lookups.
///
/// # Example
///
/// ```ignore
/// use reinhardt_orm::prelude::*;
///
/// #[derive(Model, QueryFields)]
/// struct User {
///     id: i64,
///     email: String,
///     age: i32,
///     created_at: DateTime,
/// }
///
// Type-safe queries with compile-time validation
/// QuerySet::<User>::new()
///     .filter(User::email().lower().contains("example.com"))
///     .filter(User::age().gte(18))
///     .filter(User::created_at().year().eq(2025));
///
// These would cause compile errors:
// User::age().contains(18);     // ERROR: contains() only for String
// User::email().year();          // ERROR: year() only for DateTime
// User::emai();                  // ERROR: field doesn't exist
/// ```
///
/// # Generated Methods
///
/// For each field in the struct, the macro generates a static method that
/// returns a `Field<Model, FieldType>`. The field type determines which
/// lookup methods are available:
///
/// - String fields: `lower()`, `upper()`, `trim()`, `contains()`, etc.
/// - Numeric fields: `abs()`, `ceil()`, `floor()`, `round()`
/// - DateTime fields: `year()`, `month()`, `day()`, `hour()`, etc.
/// - All fields: `eq()`, `ne()`, `gt()`, `gte()`, `lt()`, `lte()`
///
#[proc_macro_derive(QueryFields)]
pub fn derive_query_fields(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);

	derive_query_fields_impl(input).into()
}

/// Derive macro for automatic OpenAPI schema generation
///
/// Automatically implements the `ToSchema` trait for structs and enums,
/// generating OpenAPI 3.0 schemas from Rust type definitions.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::Schema;
/// use reinhardt_openapi::ToSchema;
///
/// #[derive(Schema)]
/// struct User {
///     /// User's unique identifier
///     id: i64,
///     /// User's email address
///     email: String,
///     /// Optional phone number
///     phone: Option<String>,
///     /// List of roles
///     roles: Vec<String>,
/// }
///
// Schema is automatically generated
/// let schema = User::schema();
/// ```
///
/// # Supported Types
///
/// - Primitives: `String`, `i32`, `i64`, `f32`, `f64`, `bool`
/// - `Option<T>`: Makes fields optional in the schema
/// - `Vec<T>`: Generates array schemas
/// - Custom types implementing `ToSchema`
///
/// # Features
///
/// - Automatic field metadata extraction
/// - Documentation comments become field descriptions
/// - Required/optional field detection
/// - Nested schema support
/// - Enum variant handling
///
#[proc_macro_derive(Schema)]
pub fn derive_schema(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);

	derive_schema_impl(input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Derive macro for automatic Injectable implementation with field injection
///
/// Automatically implements the `Injectable` trait for structs, injecting dependencies
/// for fields marked with `#[inject]`. Non-injected fields use `Default::default()`.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::Injectable;
/// use reinhardt_di::{Injectable, InjectionContext};
///
/// #[derive(Clone, Default)]
/// struct Database {
///     connection_string: String,
/// }
///
/// #[derive(Clone, Default)]
/// struct RedisCache {
///     host: String,
/// }
///
/// #[derive(Clone, Injectable)]
/// struct UserViewSet {
///     #[inject]
///     db: Database,
///     #[inject]
///     cache: RedisCache,
///     name: String,  // Uses Default::default()
/// }
///
/// // Automatically generated:
/// // impl Injectable for UserViewSet {
/// //     async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
/// //         let db = Depends::<Database>::resolve(ctx, true).await?;
/// //         let cache = Depends::<RedisCache>::resolve(ctx, true).await?;
/// //         Ok(Self {
/// //             db,
/// //             cache,
/// //             name: Default::default(),
/// //         })
/// //     }
/// // }
/// ```
///
/// # Cache Control
///
/// You can disable caching for specific dependencies:
///
/// ```ignore
/// #[derive(Clone, Injectable)]
/// struct MyService {
///     #[inject]
///     db: Database,              // Cached (default)
///     #[inject(cache = false)]
///     fresh_data: FreshData,     // Not cached
/// }
/// ```
///
/// # Requirements
///
/// - Struct must have named fields
/// - Non-injected fields must implement `Default`
/// - Struct must be `Clone` (required by `Injectable` trait)
/// - All `#[inject]` field types must implement `Injectable`
///
#[proc_macro_derive(Injectable, attributes(inject))]
pub fn derive_injectable(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);

	injectable_derive_impl(input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Derive macro for automatic Model implementation and migration registration
///
/// Automatically implements the `Model` trait and registers the model with the global
/// ModelRegistry for automatic migration generation.
///
/// # Example
///
/// ```ignore
/// use reinhardt_macros::Model;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Model, Serialize, Deserialize)]
/// #[model(app_label = "blog", table_name = "posts")]
/// struct Post {
///     #[field(primary_key = true)]
///     id: i64,
///
///     #[field(max_length = 200)]
///     title: String,
///
///     #[field(null = true)]
///     content: Option<String>,
/// }
/// ```
///
/// # Model Attributes
///
/// - `app_label`: Application label (default: "default")
/// - `table_name`: Database table name (default: struct name in snake_case)
///
/// # Field Attributes
///
/// - `primary_key`: Mark field as primary key (required for exactly one field)
/// - `max_length`: Maximum length for String fields (required for String)
/// - `null`: Allow NULL values (default: inferred from Option<T>)
/// - `blank`: Allow blank values in forms
/// - `unique`: Enforce uniqueness constraint
/// - `default`: Default value
/// - `db_column`: Custom database column name
/// - `editable`: Whether field is editable (default: true)
///
/// # Supported Types
///
/// - `i32` → IntegerField
/// - `i64` → BigIntegerField
/// - `String` → CharField (requires max_length)
/// - `bool` → BooleanField
/// - `DateTime<Utc>` → DateTimeField
/// - `Date` → DateField
/// - `Time` → TimeField
/// - `f32`, `f64` → FloatField
/// - `Option<T>` → Sets null=true automatically
///
/// # Requirements
///
/// - Struct must have named fields
/// - Struct must implement `Serialize` and `Deserialize`
/// - Exactly one field must be marked with `primary_key = true`
/// - String fields must specify `max_length`
///
#[proc_macro_derive(Model, attributes(model, field))]
pub fn derive_model(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);

	model_derive_impl(input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}
