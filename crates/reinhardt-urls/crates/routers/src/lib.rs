//! # Reinhardt Routers
//!
//! URL routing for Reinhardt framework.
//!
//! ## Planned Features
//! TODO: Namespace-based URL reversal - Hierarchical route naming (`"v1:users:detail"`)
//! TODO: Nested namespace resolution
//! TODO: URL reversal with namespace support
//! TODO: Route introspection - Runtime route analysis and debugging
//! TODO: OpenAPI integration - Automatic OpenAPI schema generation from routes
//! TODO: Route visualization - Generate route maps for documentation

pub mod cache;
pub mod converters;
pub mod helpers;
pub mod pattern;
pub mod reverse;
pub mod route;
pub mod router;
pub mod script_prefix;
pub mod simple;
pub mod unified_router;

// Re-export the path! macro for compile-time path validation
pub use reinhardt_routers_macros::path;

pub use cache::RouteCache;
pub use converters::{
    Converter, ConverterError, ConverterResult, IntegerConverter, SlugConverter, UuidConverter,
};
pub use helpers::{IncludedRouter, include_routes, path, re_path};
pub use pattern::{PathMatcher, PathPattern};
pub use reverse::{
    ReverseError,
    ReverseResult,
    UrlParams,
    // Type-safe reversal
    UrlPattern,
    UrlPatternWithParams,
    UrlReverser,
    reverse,
    reverse_typed,
    reverse_typed_with_params,
};
pub use route::Route;
pub use router::{DefaultRouter, Router};
pub use script_prefix::{clear_script_prefix, get_script_prefix, set_script_prefix};
pub use simple::SimpleRouter;
pub use unified_router::{
    UnifiedRouter, clear_router, get_router, is_router_registered, register_router,
};
