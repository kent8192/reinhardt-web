//! Server-side (native) public re-exports for `reinhardt`.
//!
//! This module is gated `#[cfg(native)]` at the declaration site in
//! [`super`], so per-item `native` gates from the pre-refactor `src/lib.rs`
//! are simplified to feature-only here (the outer module gate handles the
//! target check). Feature gates are otherwise preserved verbatim to keep the
//! public API surface unchanged.

// --- External crate wrappers for macro path resolution --------------------

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod reinhardt_apps {
	pub use reinhardt_apps::*;
}

#[cfg(feature = "di")]
#[doc(hidden)]
pub mod reinhardt_di {
	pub use reinhardt_di::*;
}

#[cfg(feature = "auth")]
#[doc(hidden)]
pub mod reinhardt_auth {
	pub use reinhardt_auth::*;
}

#[cfg(feature = "commands")]
#[doc(hidden)]
pub mod reinhardt_commands {
	pub use reinhardt_commands::*;
}

#[doc(hidden)]
pub mod reinhardt_core {
	pub use reinhardt_core::*;
	// For macro compatibility: Re-export EndpointMetadata at module level.
	pub use reinhardt_core::endpoint::EndpointMetadata;
}

#[doc(hidden)]
pub mod reinhardt_http {
	pub use reinhardt_http::*;
}

#[cfg(feature = "di")]
#[doc(hidden)]
pub mod reinhardt_params {
	pub use reinhardt_di::params::*;
}

#[doc(hidden)]
pub mod async_trait {
	pub use async_trait::*;
}

#[cfg(feature = "database")]
#[doc(hidden)]
pub mod linkme {
	pub use linkme::*;
}

#[cfg(feature = "database")]
#[doc(hidden)]
pub mod ctor {
	pub use ctor::*;
}

// Re-export paste for macro-generated code (Issue #3526: namespaced URL resolvers).
#[doc(hidden)]
pub use paste::paste;

#[cfg(feature = "database")]
#[doc(hidden)]
pub mod reinhardt_orm {
	pub use reinhardt_db::orm::*;
}

// --- App types ------------------------------------------------------------

#[cfg(feature = "core")]
pub use reinhardt_apps::{AppConfig, AppError, AppResult, Apps};

// --- reinhardt_migrations alias for macro compatibility -------------------

// Re-export reinhardt_migrations crate (used by collect_migrations! macro).
#[cfg(feature = "database")]
pub use reinhardt_db::migrations;

#[cfg(feature = "database")]
#[doc(hidden)]
pub use reinhardt_db::migrations as reinhardt_migrations;

// --- Settings (conf feature) ---------------------------------------------

#[cfg(feature = "conf")]
#[allow(deprecated)]
// Re-exports deprecated Settings and AdvancedSettings for backward compatibility
pub use reinhardt_conf::settings::{
	AdvancedSettings, CacheSettings, CorsSettings, DatabaseConfig, EmailSettings, LoggingSettings,
	MediaSettings, MiddlewareConfig, SessionSettings, Settings, SettingsError, StaticSettings,
	TemplateConfig,
};

#[cfg(feature = "conf")]
pub use reinhardt_conf::SecuritySettings;

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::core_settings::{CoreSettings, HasCoreSettings};

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::fragment::{HasSettings, SettingsFragment};

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::builder::SettingsBuilder;

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::profile::Profile;

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::sources::{
	DefaultSource, EnvSource, LowPriorityEnvSource, TomlFileSource,
};

// --- Core types -----------------------------------------------------------

#[cfg(feature = "core")]
pub use reinhardt_core::{
	endpoint::EndpointMetadata,
	exception::{Error, Result},
};

#[cfg(feature = "core")]
pub use reinhardt_http::{Handler, Middleware, MiddlewareChain, Request, Response, ViewResult};

// Re-export inventory crate (used by HTTP method macros for endpoint registration).
#[cfg(feature = "core")]
#[doc(hidden)]
pub use inventory;

// --- ORM ------------------------------------------------------------------

#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	DatabaseBackend, DatabaseConnection, Model, QuerySet, SoftDeletable, SoftDelete, Timestamped,
	Timestamps,
};

// ORM query expressions (Django-style F/Q objects).
//
// # Examples
//
// ```rust,no_run
// # use reinhardt::{F, Q};
// let price_expr = F::field("price");
// let filter = Q::and(vec![
//     Q::field("status").equals("active"),
//     Q::field("price").gt(100),
// ]);
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	Exists, F, FieldRef, Filter, FilterOperator, FilterValue, OuterRef, Q, QOperator, Subquery,
};

// ORM annotations and aggregations.
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	Aggregate, AggregateFunc, AggregateValue, Annotation, AnnotationValue,
};

// ORM transactions.
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	IsolationLevel, QueryValue, Savepoint, Transaction, TransactionExecutor, TransactionScope,
	atomic, atomic_with_isolation,
};

// ORM database functions (string / date-time / math / utility).
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	Abs, Cast, Ceil, Concat, CurrentDate, CurrentTime, Extract, ExtractComponent, Floor, Greatest,
	Least, Length, Lower, Mod, Now, NullIf, Power, Round, SqlType, Sqrt, Substr, Trim, TrimType,
	Upper,
};

// ORM window functions.
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	DenseRank, FirstValue, Frame, FrameBoundary, FrameType, Lag, LastValue, Lead, NTile, NthValue,
	Rank, RowNumber, Window, WindowFunction,
};

// ORM constraints and indexes.
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	BTreeIndex, CheckConstraint, Constraint, ForeignKeyConstraint, GinIndex, GistIndex, HashIndex,
	Index, OnDelete, OnUpdate, UniqueConstraint,
};

// reinhardt-query prelude types (via reinhardt-db orm).
// Value is re-exported as QueryBuilderValue to avoid name conflicts.
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{IntoValue, Order, QueryBuilderValue};

// Database pool.
#[cfg(feature = "database")]
pub use reinhardt_db::pool::{ConnectionPool, PoolConfig, PoolError};

// Database content types and migrations.
#[cfg(feature = "database")]
pub use reinhardt_db::contenttypes::{
	CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable,
	GenericRelationQuery, ModelType,
};

#[cfg(feature = "database")]
pub use reinhardt_db::migrations::{
	FieldState, Migration, MigrationAutodetector, MigrationError, MigrationPlan, MigrationRecorder,
	ModelState, ProjectState,
};

// --- REST ----------------------------------------------------------------

#[cfg(feature = "rest")]
pub use reinhardt_rest::serializers::{Deserializer, JsonSerializer, Serializer};

#[cfg(feature = "rest")]
pub use reinhardt_rest::pagination::{
	CursorPagination, LimitOffsetPagination, PageNumberPagination, PaginatedResponse, Paginator,
};

#[cfg(feature = "rest")]
pub use reinhardt_rest::filters::{
	FieldOrderingExt, FilterBackend, FilterError, FilterResult, MultiTermSearch,
};

#[cfg(feature = "rest")]
pub use reinhardt_rest::throttling::{
	AnonRateThrottle, ScopedRateThrottle, Throttle, UserRateThrottle,
};

#[cfg(feature = "rest")]
pub use reinhardt_rest::parsers::{
	FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
	Parser,
};

#[cfg(feature = "rest")]
pub use reinhardt_rest::versioning::{
	AcceptHeaderVersioning, BaseVersioning, HostNameVersioning, NamespaceVersioning,
	QueryParameterVersioning, RequestVersionExt, URLPathVersioning, VersioningError,
	VersioningMiddleware,
};

#[cfg(feature = "rest")]
pub use reinhardt_rest::metadata::{
	ActionMetadata, BaseMetadata, ChoiceInfo, FieldInfo, FieldInfoBuilder, FieldType,
	MetadataOptions, MetadataResponse, SimpleMetadata,
};

#[cfg(feature = "rest")]
pub use reinhardt_rest::negotiation::*;

// REST integration modules (re-exported as nested namespaces).
#[cfg(feature = "rest")]
pub use reinhardt_rest::{
	filters, metadata, negotiation, pagination, parsers, serializers, throttling, versioning,
};

// Browsable API (from reinhardt-browsable-api via reinhardt-rest).
#[cfg(feature = "rest")]
pub use reinhardt_rest::browsable_api;

// --- Viewsets and routers ------------------------------------------------

pub use reinhardt_views::viewsets::{
	Action, ActionType, CreateMixin, DestroyMixin, GenericViewSet, ListMixin, ModelViewSet,
	ReadOnlyModelViewSet, RetrieveMixin, UpdateMixin, ViewSet,
};

pub use reinhardt_urls::routers::{
	DefaultRouter, PathMatcher, PathPattern, Route, Router, RouterFactory, ServerRouter,
	UrlPatternsRegistration, clear_router, get_router, is_router_registered, register_router,
	register_router_arc,
};

// URL resolver traits (server-side).
pub use reinhardt_urls::routers::resolver::{UrlResolver, WebSocketUrlResolver};

// URL utilities.
pub use reinhardt_urls::routers::{
	UrlPattern, UrlPatternWithParams, UrlReverser, include_routes as include, path, re_path,
	reverse,
};

// --- Auth ----------------------------------------------------------------

#[cfg(feature = "auth")]
#[allow(deprecated)] // CurrentUser is deprecated in favor of AuthUser
pub use reinhardt_auth::{
	AllowAny, AnonymousUser, AuthBackend, AuthInfo, AuthUser, BaseUser, CurrentUser, FullUser,
	IsAdminUser, IsAuthenticated, PasswordHasher, Permission, PermissionsMixin, SimpleUser,
	validate_auth_extractors,
};

// argon2-hasher gated types (DefaultUser, DefaultUserManager, Argon2Hasher).
#[cfg(all(feature = "auth", feature = "argon2-hasher"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "auth", feature = "argon2-hasher"))))]
pub use reinhardt_auth::{Argon2Hasher, DefaultUser, DefaultUserManager};

#[cfg(feature = "auth-jwt")]
pub use reinhardt_auth::{Claims, JwtAuth, JwtError};

// Auth management.
#[cfg(feature = "auth")]
pub use reinhardt_auth::{
	CreateGroupData, CreateUserData, Group, GroupManagementError, GroupManagementResult,
	GroupManager, ObjectPermission, ObjectPermissionChecker, ObjectPermissionManager,
	UpdateUserData, UserManagementError, UserManagementResult, UserManager,
};

// --- Middleware ----------------------------------------------------------

// AuthenticationMiddleware requires sessions + middleware.
#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::AuthenticationMiddleware;

#[cfg(feature = "middleware-auth-jwt")]
pub use reinhardt_middleware::JwtAuthMiddleware;

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::{CookieSessionAuthMiddleware, CookieSessionConfig};

#[cfg(all(feature = "session-redis", feature = "middleware"))]
pub use reinhardt_middleware::RedisSessionBackend;

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::OriginGuardMiddleware;

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_middleware::{PersistentRemoteUserMiddleware, RemoteUserMiddleware};

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::{LoginRequiredConfig, LoginRequiredMiddleware};

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::LoggingMiddleware;

#[cfg(feature = "middleware-cors")]
pub use reinhardt_middleware::CorsMiddleware;

#[cfg(feature = "middleware-security")]
pub use reinhardt_middleware::SecurityMiddleware;

#[cfg(feature = "middleware-security")]
#[allow(deprecated)] // SecurityConfig is deprecated but still re-exported for compatibility
pub use reinhardt_middleware::SecurityConfig;

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::{CspConfig, CspMiddleware, CspNonce};

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::{XFrameOptions, XFrameOptionsMiddleware};

#[cfg(feature = "middleware")]
pub use reinhardt_middleware::CacheMiddleware;

// --- HTTP / hyper --------------------------------------------------------

#[cfg(feature = "core")]
pub use reinhardt_http::Extensions;

// Re-export HTTP types from hyper (already used in reinhardt_http).
pub use hyper::{Method, StatusCode};

// --- Signals + validators + views ----------------------------------------

#[cfg(feature = "core")]
pub use reinhardt_core::signals::{
	M2MAction, M2MChangeEvent, Signal, m2m_changed, post_delete, post_save, pre_delete, pre_save,
};

#[cfg(feature = "core")]
pub use reinhardt_core::validators::{
	CreditCardValidator, EmailValidator, IBANValidator, IPAddressValidator, PhoneNumberValidator,
	UrlValidator, Validate, ValidationError as ValidatorError, ValidationErrors, ValidationResult,
	Validator,
};

pub use reinhardt_views::{
	Context, DetailView, ListView, MultipleObjectMixin, SingleObjectMixin, View,
};

// --- OpenAPI -------------------------------------------------------------

#[cfg(feature = "openapi")]
pub use reinhardt_rest::openapi::*;

#[cfg(feature = "openapi-router")]
pub use reinhardt_openapi::OpenApiRouter;

// --- Shortcuts -----------------------------------------------------------

#[cfg(feature = "shortcuts")]
pub use reinhardt_shortcuts::{redirect, render_html, render_json, render_text};

#[cfg(all(feature = "shortcuts", feature = "database"))]
pub use reinhardt_shortcuts::{get_list_or_404, get_object_or_404};

// --- Cache + sessions ----------------------------------------------------

#[cfg(feature = "cache")]
pub use reinhardt_utils::cache::{Cache, CacheKeyBuilder, InMemoryCache};

#[cfg(all(feature = "cache", feature = "redis-backend"))]
pub use reinhardt_utils::cache::RedisCache;

#[cfg(feature = "sessions")]
pub use reinhardt_auth::sessions::{
	CacheSessionBackend, InMemorySessionBackend, Session, SessionBackend, SessionError,
};

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_auth::sessions::{HttpSessionConfig, SameSite, SessionMiddleware};

// --- Forms + DI + tasks + test + storage ---------------------------------

#[cfg(feature = "forms")]
pub use reinhardt_forms::{
	BoundField, CharField, EmailField, FieldError, FileField, Form, FormError, FormResult,
	IntegerField, ModelForm,
};

#[cfg(feature = "di")]
#[allow(deprecated)]
pub use reinhardt_di::injected::{Injected, OptionalInjected};

#[cfg(feature = "di")]
pub use reinhardt_di::scope::{RequestScope, Scope, SingletonScope};

#[cfg(feature = "di")]
pub use reinhardt_di::{
	Depends, DependsBuilder, DiError, DiResult, Injectable, InjectionContext,
	InjectionContextBuilder, InjectionMetadata, RequestContext,
};

// DI params - available in minimal, standard, and di features.
#[cfg(any(feature = "minimal", feature = "standard", feature = "di"))]
pub use reinhardt_di::params::{Body, Cookie, Header, Json, Path, Query};

#[cfg(feature = "tasks")]
pub use reinhardt_tasks::{Scheduler, Task, TaskExecutor, TaskQueue};

#[cfg(feature = "test")]
pub use reinhardt_test::{APIClient, APIRequestFactory, APITestCase, TestResponse};

#[cfg(feature = "storage")]
pub use reinhardt_utils::storage::{InMemoryStorage, LocalStorage, Storage};

// --- WebSockets ----------------------------------------------------------

#[cfg(feature = "websockets-pages")]
pub use reinhardt_websockets::integration::pages::PagesAuthenticator;

#[cfg(feature = "websockets")]
pub use reinhardt_websockets::room::{BroadcastResult, Room, RoomError, RoomManager, RoomResult};

#[cfg(feature = "websockets")]
pub use reinhardt_websockets::{
	ConsumerContext, Message, WebSocketConnection, WebSocketConsumer, WebSocketError,
	WebSocketResult,
};

#[cfg(feature = "websockets")]
pub use reinhardt_websockets::{
	RouteError, RouteResult, WebSocketRoute, WebSocketRouter, clear_websocket_router,
	get_websocket_router, register_websocket_router, reverse_websocket_url,
};

// --- db module (inline) --------------------------------------------------

/// Database re-exports for Model derive macro generated code.
///
/// These must be available at `::reinhardt::db::*` for the macro to work correctly.
#[cfg(feature = "database")]
pub mod db {
	pub use reinhardt_db::DatabaseConnection;
	pub use reinhardt_db::DatabaseError as Error;

	/// Database migration types and utilities.
	pub mod migrations {
		pub use reinhardt_db::migrations::*;
	}

	/// ORM query building and model operations.
	pub mod orm {
		pub use reinhardt_db::orm::*;
	}

	/// Model relationship (association) definitions.
	pub mod associations {
		pub use reinhardt_db::associations::*;
	}

	/// Convenience re-exports for database operations.
	pub mod prelude {
		pub use reinhardt_db::prelude::*;
	}
}
