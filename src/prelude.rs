//! Convenience re-exports of commonly used types.
//!
//! Provides a unified import surface so consumers can write
//! `use reinhardt::prelude::*;`. The native and wasm-side surfaces are kept
//! separate (gated `#[cfg(native)]` vs. `#[cfg(all(not(native), target_family
//! = "wasm"))]`) for Stage 1 of Issue #4362; a fully target-agnostic prelude
//! built on top of [`crate::exports`] is introduced by Stage 4 (#4367).

// --- Native-side prelude --------------------------------------------------

/// Wasm-side `prelude` shim (Issue #4189).
///
/// The native `prelude` module is `#[cfg(native)]`-gated, but the
/// `--with-pages` scaffold emits `use reinhardt::prelude::*;` in the
/// generated `src/config/urls.rs`, which must compile on
/// `wasm32-unknown-unknown` so wasm SPA consumers can `cargo check --lib`
/// without modifying the scaffolded sources.
#[cfg(wasm)]
mod wasm {
	#[cfg(feature = "client-router")]
	pub use crate::urls::prelude::UnifiedRouter;
}

#[cfg(wasm)]
pub use wasm::*;

/// Convenience re-exports of commonly used types (server-side only).
#[cfg(native)]
mod native {
	// Core types - always available
	pub use crate::{
		Action,
		DefaultRouter,
		DetailView,
		ListView,
		ModelViewSet,
		MultipleObjectMixin,
		ReadOnlyModelViewSet,
		Route,
		Router,
		ServerRouter,
		SingleObjectMixin,
		StatusCode,
		View,
		ViewSet,
		// Routers
		clear_router,
		get_router,
		is_router_registered,
		register_router,
	};

	// ViewResult requires core feature (re-exported from reinhardt_http)
	#[cfg(feature = "core")]
	pub use crate::ViewResult;

	// UnifiedRouter requires client-router feature
	#[cfg(feature = "client-router")]
	pub use crate::UnifiedRouter;

	// External dependencies (via core)
	#[cfg(feature = "core")]
	pub use crate::core::async_trait;
	#[cfg(feature = "core")]
	pub use crate::core::serde::{Deserialize, Serialize};

	// Core feature - types, signals, etc.
	#[cfg(feature = "core")]
	pub use crate::{
		Error, Handler, Middleware, MiddlewareChain, Request, Response, Result, Signal,
		m2m_changed, post_delete, post_save, pre_delete, pre_save,
	};

	// HTTP method macros - always available
	pub use crate::{api_view, delete, get, patch, post, put};

	// Database feature - ORM and Model macros
	#[cfg(feature = "database")]
	pub use crate::{
		Aggregate,
		// Annotations and aggregations
		Annotation,
		CheckConstraint,
		// Common database functions
		Concat,
		CurrentDate,
		DatabaseConnection,
		DenseRank,
		// Query expressions (Django-style F/Q objects)
		F,
		ForeignKeyConstraint,
		Lower,
		Now,
		Q,
		QOperator,
		Rank,
		RowNumber,
		SoftDeletable,
		Timestamped,
		// Transaction management
		Transaction,
		// Constraints
		UniqueConstraint,
		Upper,
		// Window functions (commonly used)
		Window,
		atomic,
		// Model attribute macro for struct-level model definition
		model,
	};

	// Import Model trait directly from reinhardt_db to avoid name shadowing
	// (crate::Model refers to the derive macro, not the trait)
	#[cfg(feature = "database")]
	pub use reinhardt_db::orm::Model;

	// Auth feature
	#[cfg(feature = "auth")]
	pub use crate::{
		AuthBackend,
		Group,
		GroupManager,
		// Object-level permissions
		ObjectPermission,
		ObjectPermissionChecker,
		PasswordHasher,
		Permission,
		SimpleUser,
		// User and group management
		UserManager,
	};

	// DI params - FastAPI-style parameter extraction
	#[cfg(any(feature = "minimal", feature = "standard", feature = "di"))]
	pub use crate::{Body, Cookie, Header, Json, Path, Query};

	// REST feature - serializers, parsers, pagination, throttling, versioning, metadata
	#[cfg(feature = "rest")]
	pub use crate::{
		// Versioning
		AcceptHeaderVersioning,
		// Throttling
		AnonRateThrottle,
		CursorPagination,
		FormParser,
		// Parsers
		JSONParser,
		JsonSerializer,
		LimitOffsetPagination,
		MultiPartParser,
		// Filters
		MultiTermSearch,
		// Pagination
		PageNumberPagination,
		Paginator,
		Parser,
		QueryParameterVersioning,
		ScopedRateThrottle,
		// Serializers
		Serializer,
		// Metadata
		SimpleMetadata,
		Throttle,
		URLPathVersioning,
		UserRateThrottle,
		VersioningMiddleware,
	};

	// Settings feature
	#[cfg(feature = "conf")]
	#[allow(deprecated)] // Re-exports deprecated Settings for backward compatibility
	pub use crate::Settings;

	// Middleware
	#[cfg(any(feature = "standard", feature = "middleware"))]
	pub use crate::LoggingMiddleware;

	// Security middleware
	#[cfg(feature = "middleware-security")]
	pub use crate::SecurityMiddleware;

	// Sessions feature
	#[cfg(all(feature = "sessions", feature = "middleware"))]
	pub use crate::AuthenticationMiddleware;
	#[cfg(feature = "sessions")]
	pub use crate::Session;

	// Cache feature
	#[cfg(feature = "cache")]
	pub use crate::{Cache, InMemoryCache};
}

#[cfg(native)]
pub use native::*;
