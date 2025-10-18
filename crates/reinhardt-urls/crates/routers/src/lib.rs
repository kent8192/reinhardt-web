pub mod helpers;
pub mod pattern;
pub mod reverse;
pub mod route;
pub mod router;

// Re-export the path! macro for compile-time path validation
pub use reinhardt_routers_macros::path;

pub use helpers::{include_routes, path, re_path, IncludedRouter};
pub use pattern::{PathMatcher, PathPattern};
pub use reverse::{
    reverse,
    reverse_typed,
    reverse_typed_with_params,
    ReverseError,
    ReverseResult,
    UrlParams,
    // Type-safe reversal
    UrlPattern,
    UrlPatternWithParams,
    UrlReverser,
};
pub use route::Route;
pub use router::{DefaultRouter, Router};
