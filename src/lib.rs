//! # Reinhardt
//!
//! A full-stack API framework for Rust, inspired by Django and Django REST Framework.
//!
//! Reinhardt provides a complete, batteries-included solution for building production-ready
//! REST APIs with Rust. It follows Rust's composition patterns instead of Python's inheritance
//! model, making full use of traits, generics, and zero-cost abstractions.
//!
//! ## Core Principles
//!
//! - **Composition over Inheritance**: Uses Rust's trait system for composable behavior
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Zero-Cost Abstractions**: High-level ergonomics without runtime overhead
//! - **Async-First**: Built on tokio and async/await from the ground up
//!
//! ## Feature Flags
//!
//! Reinhardt provides flexible feature flags to control compilation and reduce binary size.
//!
//! ### Presets
//!
//! - `minimal` - Core functionality only (routing, DI, params)
//! - `standard` (default) - Balanced for most projects
//! - `full` - All features enabled
//! - `api-only` - REST API without templates/forms
//! - `graphql-server` - GraphQL-focused setup
//! - `websocket-server` - WebSocket-centric setup
//! - `cli-tools` - CLI and background jobs
//! - `test-utils` - Testing utilities
//!
//! ### Fine-grained Control
//!
//! Fine-grained feature flags for precise control over included functionality:
//!
//! #### Authentication ✅
//! - `auth-jwt` - JWT authentication
//! - `auth-session` - Session-based authentication
//! - `auth-oauth` - OAuth2 support
//! - `auth-token` - Token authentication
//!
//! #### Database Backends ✅
//! - `db-postgres` - PostgreSQL support
//! - `db-mysql` - MySQL support
//! - `db-sqlite` - SQLite support
//! - `db-mongodb` - MongoDB support
//! - `db-cockroachdb` - CockroachDB support (distributed transactions)
//!
//! #### Serialization ✅
//! - `serialize-json` - JSON serialization (via `serde_json`)
//! - `serialize-xml` - XML serialization (via `quick-xml` and `serde-xml-rs`)
//! - `serialize-yaml` - YAML serialization (via `serde_yaml`)
//!
//! #### Middleware ✅
//! - `middleware-cors` - CORS (Cross-Origin Resource Sharing) middleware
//! - `middleware-compression` - Response compression (Gzip, Brotli)
//! - `middleware-security` - Security headers (HSTS, XSS Protection, etc.)
//! - `middleware-rate-limit` - Rate limiting and throttling
//!
//! See [docs/FEATURE_FLAGS.md](../docs/FEATURE_FLAGS.md) for detailed documentation.
//!
//! ## Quick Example
//!
//! ```rust,ignore
//! use reinhardt::prelude::*;
//! use serde::{Serialize, Deserialize};
//!
//! // Define your model (using composition, not inheritance)
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct User {
//!     id: Option<i64>,
//!     username: String,
//!     email: String,
//! }
//!
//! // Implement Model trait
//! impl Model for User {
//!     type PrimaryKey = i64;
//!     fn table_name() -> &'static str { "users" }
//!     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
//!     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
//! }
//!
//! // Create a ViewSet (no inheritance needed!)
//! let users_viewset = ModelViewSet::<User, JsonSerializer<User>>::new("users");
//!
//! // Set up routing
//! let mut router = DefaultRouter::new();
//! router.register_viewset("users", users_viewset);
//!
//! // Add middleware using composition
//! let app = MiddlewareChain::new(Arc::new(router))
//!     .with_middleware(Arc::new(LoggingMiddleware::new()))
//!     .with_middleware(Arc::new(CorsMiddleware::permissive()));
//! ```

// Module re-exports following Django's structure
pub mod apps;
#[cfg(feature = "conf")]
pub mod conf;
#[cfg(feature = "contrib")]
pub mod contrib;
#[cfg(feature = "core")]
pub mod core;
#[cfg(feature = "database")]
pub mod db;
#[cfg(feature = "di")]
pub mod di;
#[cfg(feature = "forms")]
pub mod forms;
pub mod http;
pub mod middleware;
#[cfg(feature = "rest")]
pub mod rest;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "shortcuts")]
pub mod shortcuts;
#[cfg(feature = "tasks")]
pub mod tasks;
#[cfg(feature = "templates")]
pub mod template;
#[cfg(feature = "test")]
pub mod test;
pub mod urls;
pub mod utils;
pub mod views;

// Re-export app types from reinhardt-apps
pub use reinhardt_apps::{AppConfig, AppError, AppResult, Apps};

// Re-export settings from dedicated crate
pub use reinhardt_settings::{
	AdvancedSettings, CacheSettings, CorsSettings, DatabaseConfig, EmailSettings, LoggingSettings,
	MediaSettings, MiddlewareConfig, SessionSettings, Settings, SettingsError, StaticSettings,
	TemplateConfig,
};

// Re-export core types
pub use reinhardt_apps::{Error, Handler, Middleware, MiddlewareChain, Request, Response, Result};

// Re-export ORM
pub use reinhardt_orm::{
	DatabaseBackend, DatabaseConnection, Model, QuerySet, SoftDeletable, SoftDelete, Timestamped,
	Timestamps,
};

// Re-export database pool
#[cfg(feature = "database")]
pub use reinhardt_db::pool::{ConnectionPool, PoolConfig, PoolError};

// Re-export serializers
pub use reinhardt_serializers::{Deserializer, JsonSerializer, Serializer};

// Re-export viewsets
pub use reinhardt_viewsets::{
	Action, ActionType, CreateMixin, DestroyMixin, GenericViewSet, ListMixin, ModelViewSet,
	ReadOnlyModelViewSet, RetrieveMixin, UpdateMixin, ViewSet,
};

// Re-export routers
pub use reinhardt_routers::{
	DefaultRouter, PathMatcher, PathPattern, Route, Router, UnifiedRouter, clear_router,
	get_router, is_router_registered, register_router,
};

// Re-export auth
pub use reinhardt_auth::{
	AllowAny, AnonymousUser, Argon2Hasher, AuthBackend, Claims, IsAdminUser, IsAuthenticated,
	JwtAuth, PasswordHasher, Permission, SimpleUser, User,
};

// Re-export middleware
#[cfg(feature = "sessions")]
pub use reinhardt_middleware::AuthenticationMiddleware;
pub use reinhardt_middleware::{CorsMiddleware, LoggingMiddleware};

// Re-export HTTP types (additional commonly used types)
pub use reinhardt_http::Extensions;
// Re-export StatusCode from hyper (already used in reinhardt_http)
pub use hyper::StatusCode;

// Re-export pagination
#[cfg(any(feature = "standard", feature = "reinhardt-pagination"))]
pub use reinhardt_pagination::{
	CursorPagination, LimitOffsetPagination, PageNumberPagination, PaginatedResponse, Paginator,
};

// Re-export filters
#[cfg(any(feature = "standard", feature = "reinhardt-filters"))]
pub use reinhardt_filters::{
	FieldOrderingExt, FilterBackend, FilterError, FilterResult, MultiTermSearch,
};

// Re-export throttling
#[cfg(any(feature = "standard", feature = "reinhardt-throttling"))]
pub use reinhardt_throttling::{AnonRateThrottle, ScopedRateThrottle, Throttle, UserRateThrottle};

// Re-export signals
#[cfg(any(feature = "standard", feature = "reinhardt-signals"))]
pub use reinhardt_signals::{
	M2MAction, M2MChangeEvent, Signal, m2m_changed, post_delete, post_save, pre_delete, pre_save,
};

// Re-export core utilities
// Note: reinhardt_types provides Handler, Middleware, etc. which are already re-exported via reinhardt_apps

// Re-export validators
#[cfg(feature = "core")]
pub use reinhardt_core::validators::{
	CreditCardValidator, EmailValidator, IBANValidator, IPAddressValidator, PhoneNumberValidator,
	UrlValidator, ValidationError as ValidatorError, ValidationResult, Validator,
};

// Re-export views
pub use reinhardt_views::{
	Context, DetailView, ListView, MultipleObjectMixin, SingleObjectMixin, View,
};

// Re-export parsers
#[cfg(any(feature = "standard", feature = "reinhardt-parsers"))]
pub use reinhardt_parsers::{
	FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
	Parser,
};

// Re-export renderers
#[cfg(any(feature = "standard", feature = "reinhardt-renderers"))]
pub use reinhardt_renderers::{BrowsableAPIRenderer, JSONRenderer, XMLRenderer};

// Re-export versioning
#[cfg(any(feature = "standard", feature = "reinhardt-versioning"))]
pub use reinhardt_versioning::{
	AcceptHeaderVersioning, BaseVersioning, HostNameVersioning, NamespaceVersioning,
	QueryParameterVersioning, RequestVersionExt, URLPathVersioning, VersioningError,
	VersioningMiddleware,
};

// Re-export metadata
#[cfg(any(feature = "standard", feature = "reinhardt-metadata"))]
pub use reinhardt_metadata::{
	ActionMetadata, BaseMetadata, ChoiceInfo, FieldInfo, FieldInfoBuilder, FieldType,
	MetadataOptions, MetadataResponse, SimpleMetadata,
};

// Re-export negotiation
#[cfg(any(feature = "standard", feature = "reinhardt-negotiation"))]
pub use reinhardt_negotiation::*;

// Re-export REST integration
#[cfg(feature = "rest")]
pub use reinhardt_rest::*;

// Re-export shortcuts (Django-style convenience functions)
#[cfg(feature = "shortcuts")]
pub use reinhardt_shortcuts::{
	get_list_or_404, get_object_or_404, redirect, render, render_json, render_template,
};

// Re-export URL utilities
pub use reinhardt_routers::{
	UrlPattern, UrlPatternWithParams, UrlReverser, include_routes as include, path, re_path,
	reverse,
};

// Re-export database related (database feature)
#[cfg(feature = "database")]
pub use reinhardt_contenttypes::{
	CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable,
	GenericRelationQuery, ModelType,
};
#[cfg(feature = "database")]
pub use reinhardt_migrations::{
	FieldState, MakeMigrationsCommand, MakeMigrationsOptions, Migration, MigrationAutodetector,
	MigrationError, MigrationExecutor, MigrationLoader, MigrationPlan, MigrationRecorder,
	MigrationWriter, ModelState, ProjectState,
};

// Re-export cache (cache feature)
#[cfg(feature = "cache")]
pub use reinhardt_cache::{
	Cache, CacheKeyBuilder, CacheMiddleware, CacheMiddlewareConfig, CacheService, InMemoryCache,
};

#[cfg(all(feature = "cache", feature = "redis-backend"))]
pub use reinhardt_cache::RedisCache;

// Re-export sessions (sessions feature)
#[cfg(feature = "sessions")]
pub use reinhardt_sessions::{
	CacheSessionBackend, InMemorySessionBackend, Session, SessionBackend, SessionError,
};

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_sessions::{HttpSessionConfig, SameSite, SessionMiddleware};

// Re-export contrib modules (contrib feature)
// Note: reinhardt_contrib exports individual modules (auth, sessions, etc.)
// rather than a single "contrib" module

// Re-export forms (forms feature)
#[cfg(feature = "forms")]
pub use reinhardt_forms::{
	BoundField, CharField, EmailField, FieldError, FileField, Form, FormError, FormResult,
	IntegerField, ModelForm,
};

// Re-export DI and parameters (FastAPI-style parameter extraction)
#[cfg(feature = "di")]
pub use reinhardt_di::{Depends, DiError, DiResult, InjectionContext, RequestContext};

#[cfg(feature = "minimal")]
pub use reinhardt_params::{Body, Cookie, Header, Json, Path, Query};

// Re-export templates
#[cfg(feature = "templates")]
pub use reinhardt_template::TemplateError;

// Re-export tasks
#[cfg(feature = "tasks")]
pub use reinhardt_tasks::{Scheduler, Task, TaskExecutor, TaskQueue};

// Re-export test utilities
#[cfg(feature = "test")]
pub use reinhardt_test::{APIClient, APIRequestFactory, APITestCase, TestResponse};

// Re-export storage
#[cfg(feature = "storage")]
pub use reinhardt_storage::{InMemoryStorage, LocalStorage, Storage};

// Re-export common external dependencies
pub use async_trait::async_trait;
pub use serde::{Deserialize, Serialize};
pub use tokio;

/// Prelude module for convenient imports
pub mod prelude {
	pub use crate::{
		// Versioning
		AcceptHeaderVersioning,
		Action,

		// Throttling
		AnonRateThrottle,
		AppConfig,
		// App
		Apps,
		AuthBackend,
		// Middleware
		BrowsableAPIRenderer,

		CorsMiddleware,
		CursorPagination,
		DatabaseConnection,

		DefaultRouter,
		Deserialize,
		Deserializer,
		DetailView,
		// Core
		Error,

		FieldInfo,
		FieldOrderingExt,
		FieldType,
		FilterBackend,

		FormParser,
		Handler,
		// Parsers
		JSONParser,
		// Renderers
		JSONRenderer,
		JsonSerializer,
		JwtAuth,
		LimitOffsetPagination,
		ListView,
		LoggingMiddleware,

		Middleware,
		MiddlewareChain,

		// ORM
		Model,
		ModelViewSet,
		MultiPartParser,
		// Filters
		MultiTermSearch,
		MultipleObjectMixin,
		// Pagination
		PageNumberPagination,
		Paginator,

		Parser,

		PasswordHasher,
		Permission,

		QueryParameterVersioning,
		ReadOnlyModelViewSet,
		Request,
		Response,
		Result,
		Route,

		// Routers
		Router,
		ScopedRateThrottle,
		Serialize,
		// Serializers
		Serializer,
		Settings,

		// Signals
		Signal,
		// Metadata
		SimpleMetadata,
		SimpleUser,
		SingleObjectMixin,

		SoftDeletable,
		StatusCode,
		Throttle,

		Timestamped,
		URLPathVersioning,
		UnifiedRouter,
		// Auth
		User,
		UserRateThrottle,
		VersioningMiddleware,

		// Views
		View,
		// ViewSets
		ViewSet,

		XMLRenderer,
		// External
		async_trait,
		clear_router,
		get_router,
		is_router_registered,
		m2m_changed,

		post_delete,
		post_save,
		pre_delete,
		pre_save,
		register_router,
	};

	// Cache (if enabled)
	#[cfg(feature = "cache")]
	pub use crate::{Cache, InMemoryCache};

	// Sessions (if enabled)
	#[cfg(feature = "sessions")]
	pub use crate::{AuthenticationMiddleware, InMemorySessionBackend, Session};

	// Forms (if enabled)
	#[cfg(feature = "forms")]
	pub use crate::{CharField, EmailField, FieldError, Form, FormError, ModelForm};

	// Shortcuts (if enabled)
	#[cfg(feature = "shortcuts")]
	pub use crate::{get_list_or_404, get_object_or_404, redirect, render};

	// DI/Params (if enabled)
	#[cfg(feature = "minimal")]
	pub use crate::{Body, Cookie, Header, Json, Path, Query};

	#[cfg(feature = "di")]
	pub use crate::{Depends, DiError, DiResult, InjectionContext, RequestContext};

	// Templates (if enabled)
	#[cfg(feature = "templates")]
	pub use crate::TemplateError;

	// Tasks (if enabled)
	#[cfg(feature = "tasks")]
	pub use crate::{Scheduler, Task, TaskQueue};

	// URL utilities
	pub use crate::{
		UrlPattern, UrlPatternWithParams, UrlReverser, include, path, re_path, reverse,
	};

	// HTTP utilities
	pub use crate::Extensions;

	// Storage (if enabled)
	#[cfg(feature = "storage")]
	pub use crate::{InMemoryStorage, LocalStorage, Storage};

	// Database pool (if enabled)
	#[cfg(feature = "database")]
	pub use crate::{ConnectionPool, PoolConfig, PoolError};

	// Core utilities (if enabled)
	#[cfg(feature = "core")]
	pub use crate::{
		CreditCardValidator, EmailValidator, IBANValidator, IPAddressValidator,
		PhoneNumberValidator, UrlValidator, ValidationResult, Validator, ValidatorError,
	};

	// Test utilities (if enabled)
	#[cfg(feature = "test")]
	pub use crate::{APIClient, TestResponse};

	pub use std::sync::Arc;
}
