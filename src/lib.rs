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
//! ```rust,no_run
//! use reinhardt::prelude::*;
//! use serde::{Serialize, Deserialize};
//! use std::sync::Arc;
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

// Re-export external crates for macro support
// Macro-generated code uses paths like `::reinhardt::reinhardt_apps::AppConfig`
// These wrapper modules provide the namespace structure that macros expect.
// Note: These are marked #[doc(hidden)] because they are internal dependencies
// used by macro-generated code. Users should not use these directly.
#[doc(hidden)]
pub mod reinhardt_apps {
	pub use reinhardt_apps::*;
}

#[doc(hidden)]
pub mod reinhardt_di {
	pub use reinhardt_di::*;
}

#[doc(hidden)]
pub mod reinhardt_core {
	pub use reinhardt_core::*;
}

#[doc(hidden)]
pub mod reinhardt_http {
	pub use reinhardt_http::*;
}

#[doc(hidden)]
pub mod reinhardt_params {
	pub use reinhardt_params::*;
}

#[doc(hidden)]
pub mod reinhardt_types {
	pub use reinhardt_types::*;
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

// Module re-exports following Django's structure
#[cfg(feature = "core")]
pub mod apps;
#[cfg(feature = "commands")]
pub mod commands;
#[cfg(feature = "conf")]
pub mod conf;
#[cfg(feature = "core")]
pub mod core;
#[cfg(feature = "di")]
pub mod di;
#[cfg(feature = "forms")]
pub mod forms;
pub mod http;
#[cfg(any(feature = "standard", feature = "middleware"))]
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

// Contrib modules (Django-style)
pub mod contrib {
	#[cfg(feature = "admin")]
	#[cfg_attr(docsrs, doc(cfg(feature = "admin")))]
	pub use reinhardt_admin::panel as admin;
}

// Re-export admin at top level for convenience (accessible as reinhardt::admin)
#[cfg(feature = "admin")]
#[cfg_attr(docsrs, doc(cfg(feature = "admin")))]
pub mod admin {
	// Re-export panel module for direct access to types
	pub use reinhardt_admin::panel;

	// Re-export commonly used admin types at module level
	pub use reinhardt_admin::panel::*;
}

// Re-export app types from reinhardt-apps
#[cfg(feature = "core")]
pub use reinhardt_apps::{
	AppConfig, AppError, AppResult, Apps, get_apps, init_apps, init_apps_checked,
};

// Re-export macros
#[cfg(feature = "core")]
pub use reinhardt_macros::{AppConfig, installed_apps};

// Re-export Model derive macro and model attribute macro (requires database feature)
#[cfg(feature = "database")]
pub use reinhardt_macros::{Model, model};

// Re-export collect_migrations macro (requires database feature)
#[cfg(feature = "database")]
pub use reinhardt_macros::collect_migrations;

// Re-export reinhardt_migrations crate (used by collect_migrations! macro)
#[cfg(feature = "database")]
pub use reinhardt_db::migrations;

// Alias for macro compatibility
#[cfg(feature = "database")]
#[doc(hidden)]
pub use migrations as reinhardt_migrations;

// Re-export HTTP method macros
pub use reinhardt_macros::{api_view, delete, get, patch, post, put};

// Re-export admin attribute macro (requires admin feature)
#[cfg(feature = "admin")]
pub use reinhardt_macros::admin;

// Re-export settings from dedicated crate
#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::{
	AdvancedSettings, CacheSettings, CorsSettings, DatabaseConfig, EmailSettings, LoggingSettings,
	MediaSettings, MiddlewareConfig, SessionSettings, Settings, SettingsError, StaticSettings,
	TemplateConfig,
};

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::builder::SettingsBuilder;

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::profile::Profile;

#[cfg(feature = "conf")]
pub use reinhardt_conf::settings::sources::{
	DefaultSource, EnvSource, LowPriorityEnvSource, TomlFileSource,
};

// Re-export core types
#[cfg(feature = "core")]
pub use reinhardt_core::{
	exception::{Error, Result},
	http::{Request, Response},
	types::{Handler, Middleware, MiddlewareChain},
};

// Re-export ViewResult from reinhardt-http
#[cfg(feature = "core")]
pub use reinhardt_http::ViewResult;

// Re-export ORM
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	DatabaseBackend, DatabaseConnection, Model, QuerySet, SoftDeletable, SoftDelete, Timestamped,
	Timestamps,
};

// Re-export ORM query expressions (Django-style F/Q objects)
//
// # Availability
//
// Requires `database` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{F, Q};
//
// // Reference a field (like Django's F object)
// let price_expr = F::field("price");
//
// // Build complex queries (like Django's Q object)
// let filter = Q::and(vec![
//     Q::field("status").equals("active"),
//     Q::field("price").gt(100),
// ]);
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	// Query expressions (equivalent to Django's F and Q)
	Exists,
	F,
	OuterRef,
	Q,
	QOperator,
	Subquery,
};

// Re-export ORM annotations and aggregations
//
// # Availability
//
// Requires `database` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{Annotation, Aggregate};
//
// // Annotate query results with computed values
// let query = User::objects()
//     .annotate("full_name", Annotation::concat(vec![
//         F::field("first_name"),
//         F::value(" "),
//         F::field("last_name"),
//     ]));
//
// // Aggregate data
// let stats = Product::objects()
//     .aggregate("avg_price", Aggregate::avg("price"));
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	// Aggregations
	Aggregate,
	AggregateFunc,
	AggregateValue,
	// Annotations
	Annotation,
	AnnotationValue,
};

// Re-export ORM transactions
//
// # Availability
//
// Requires `database` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{atomic, IsolationLevel, Transaction};
//
// // Use atomic decorator for transactions
// let result = atomic(|| async {
//     let user = User::create(data).await?;
//     let profile = Profile::create(user.id).await?;
//     Ok((user, profile))
// }).await?;
//
// // Or with specific isolation level
// use reinhardt::atomic_with_isolation;
// let result = atomic_with_isolation(IsolationLevel::Serializable, || async {
//     // Your transaction code
// }).await?;
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	// Transaction management
	IsolationLevel,
	QueryValue,
	Savepoint,
	Transaction,
	TransactionExecutor,
	TransactionScope,
	atomic,
	atomic_with_isolation,
};

// Re-export ORM database functions
//
// # Availability
//
// Requires `database` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{Concat, Upper, Lower, Now};
//
// // String functions
// let query = User::objects()
//     .annotate("full_name", Concat::new(vec![
//         F::field("first_name"),
//         F::value(" "),
//         F::field("last_name"),
//     ]))
//     .annotate("email_upper", Upper::new(F::field("email")));
//
// // Date/time functions
// let recent_users = User::objects()
//     .filter(Q::field("created_at").gte(Now::new()));
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	// Math functions
	Abs,
	// Utility functions
	Cast,
	Ceil,
	// String functions
	Concat,
	// Date/time functions
	CurrentDate,
	CurrentTime,
	Extract,
	ExtractComponent,
	Floor,
	Greatest,
	Least,
	Length,
	Lower,
	Mod,
	Now,
	NullIf,
	Power,
	Round,
	SqlType,
	Sqrt,
	Substr,
	Trim,
	TrimType,
	Upper,
};

// Re-export ORM window functions
//
// # Availability
//
// Requires `database` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{Window, RowNumber, Rank};
//
// // Add row numbers to query results
// let query = Product::objects()
//     .annotate("row_num", RowNumber::new()
//         .over(Window::new().order_by("price")));
//
// // Ranking within partitions
// let query = Sale::objects()
//     .annotate("rank", Rank::new()
//         .over(Window::new()
//             .partition_by("category")
//             .order_by("-amount")));
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	// Ranking functions
	DenseRank,
	// Value functions
	FirstValue,
	// Window specification
	Frame,
	FrameBoundary,
	FrameType,
	Lag,
	LastValue,
	Lead,
	NTile,
	NthValue,
	Rank,
	RowNumber,
	Window,
	WindowFunction,
};

// Re-export ORM constraints and indexes
//
// # Availability
//
// Requires `database` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{UniqueConstraint, Index, BTreeIndex};
//
// // Define constraints programmatically
// let constraint = UniqueConstraint::new(vec!["email"]);
//
// // Create indexes
// let index = BTreeIndex::new("user_email_idx", vec!["email"]);
// ```
#[cfg(feature = "database")]
pub use reinhardt_db::orm::{
	// Indexes
	BTreeIndex,
	// Constraints
	CheckConstraint,
	Constraint,
	ForeignKeyConstraint,
	GinIndex,
	GistIndex,
	HashIndex,
	Index,
	OnDelete,
	OnUpdate,
	UniqueConstraint,
};

// Re-export database pool
#[cfg(feature = "database")]
pub use reinhardt_db::pool::{ConnectionPool, PoolConfig, PoolError};

// Re-export serializers
#[cfg(feature = "rest")]
pub use reinhardt_rest::serializers::{Deserializer, JsonSerializer, Serializer};

// Re-export viewsets
pub use reinhardt_views::viewsets::{
	Action, ActionType, CreateMixin, DestroyMixin, GenericViewSet, ListMixin, ModelViewSet,
	ReadOnlyModelViewSet, RetrieveMixin, UpdateMixin, ViewSet,
};

// Re-export routers
pub use reinhardt_urls::routers::{
	DefaultRouter, PathMatcher, PathPattern, Route, Router, UnifiedRouter, clear_router,
	get_router, is_router_registered, register_router, register_router_arc,
};

// Re-export auth
#[cfg(feature = "auth")]
pub use reinhardt_auth::{
	AllowAny, AnonymousUser, AuthBackend, BaseUser, DefaultUser, FullUser, IsAdminUser,
	IsAuthenticated, PasswordHasher, Permission, PermissionsMixin, SimpleUser, User,
};

#[cfg(feature = "auth")]
#[cfg_attr(docsrs, doc(cfg(feature = "argon2-hasher")))]
pub use reinhardt_auth::Argon2Hasher;

#[cfg(feature = "auth-jwt")]
pub use reinhardt_auth::{Claims, JwtAuth};

// Re-export auth management
//
// # Availability
//
// Requires `auth` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{UserManager, GroupManager, ObjectPermission};
//
// // User management
// let user_manager = UserManager::new();
// let user = user_manager.create_user(CreateUserData {
//     username: "alice".to_string(),
//     email: "alice@example.com".to_string(),
//     password: "secret".to_string(),
// }).await?;
//
// // Group management
// let group_manager = GroupManager::new();
// let group = group_manager.create_group(CreateGroupData {
//     name: "editors".to_string(),
// }).await?;
//
// // Object-level permissions
// let perm = ObjectPermission::new("edit", user, article);
// ```
#[cfg(feature = "auth")]
pub use reinhardt_auth::{
	// Group management
	CreateGroupData,
	// User management
	CreateUserData,
	Group,
	GroupManagementError,
	GroupManagementResult,
	GroupManager,
	// Object-level permissions
	ObjectPermission,
	ObjectPermissionChecker,
	ObjectPermissionManager,
	UpdateUserData,
	UserManagementError,
	UserManagementResult,
	UserManager,
};

// Re-export middleware
#[cfg(feature = "sessions")]
pub use reinhardt_middleware::AuthenticationMiddleware;

#[cfg(any(feature = "standard", feature = "middleware"))]
pub use reinhardt_middleware::LoggingMiddleware;

#[cfg(feature = "middleware-cors")]
pub use reinhardt_middleware::CorsMiddleware;

// Re-export HTTP types (additional commonly used types)
#[cfg(feature = "core")]
pub use reinhardt_core::http::Extensions;

// Re-export HTTP types from hyper (already used in reinhardt_http)
pub use hyper::{Method, StatusCode};

// Re-export pagination
#[cfg(feature = "rest")]
pub use reinhardt_rest::pagination::{
	CursorPagination, LimitOffsetPagination, PageNumberPagination, PaginatedResponse, Paginator,
};

// Re-export filters
#[cfg(feature = "rest")]
pub use reinhardt_rest::filters::{
	FieldOrderingExt, FilterBackend, FilterError, FilterResult, MultiTermSearch,
};

// Re-export throttling
#[cfg(feature = "rest")]
pub use reinhardt_rest::throttling::{
	AnonRateThrottle, ScopedRateThrottle, Throttle, UserRateThrottle,
};

// Re-export signals
#[cfg(feature = "core")]
pub use reinhardt_core::signals::{
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
#[cfg(feature = "rest")]
pub use reinhardt_rest::parsers::{
	FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
	Parser,
};

// Re-export renderers
#[cfg(feature = "reinhardt-template")]
pub use reinhardt_template::renderers::{BrowsableApiRenderer, JSONRenderer, TemplateHTMLRenderer};

// Re-export versioning
#[cfg(feature = "rest")]
pub use reinhardt_rest::versioning::{
	AcceptHeaderVersioning, BaseVersioning, HostNameVersioning, NamespaceVersioning,
	QueryParameterVersioning, RequestVersionExt, URLPathVersioning, VersioningError,
	VersioningMiddleware,
};

// Re-export metadata
#[cfg(feature = "rest")]
pub use reinhardt_rest::metadata::{
	ActionMetadata, BaseMetadata, ChoiceInfo, FieldInfo, FieldInfoBuilder, FieldType,
	MetadataOptions, MetadataResponse, SimpleMetadata,
};

// Re-export negotiation
#[cfg(feature = "rest")]
pub use reinhardt_rest::negotiation::*;

// Re-export REST integration modules
#[cfg(feature = "rest")]
pub use reinhardt_rest::{
	filters, metadata, negotiation, pagination, parsers, renderers, serializers, throttling,
	versioning,
};

// Re-export browsable API (from reinhardt-browsable-api via reinhardt-rest)
#[cfg(feature = "rest")]
pub use reinhardt_browsable_api as browsable_api;

// Re-export OpenAPI types
//
// # Availability
//
// Requires `openapi` feature.
//
// # Examples
//
// ```rust,ignore
// use reinhardt::{OpenApi, ApiDoc};
//
// // Define API documentation
// #[derive(OpenApi)]
// #[openapi(paths(get_users, create_user))]
// struct ApiDoc;
//
// // Generate OpenAPI schema
// let openapi = ApiDoc::openapi();
// let json = serde_json::to_string_pretty(&openapi)?;
// ```
#[cfg(feature = "openapi")]
pub use reinhardt_rest::openapi::*;

// Re-export shortcuts (Django-style convenience functions)
#[cfg(feature = "shortcuts")]
pub use reinhardt_shortcuts::{
	get_list_or_404, get_object_or_404, redirect, render, render_json, render_template,
};

// Re-export URL utilities
pub use reinhardt_urls::routers::{
	UrlPattern, UrlPatternWithParams, UrlReverser, include_routes as include, path, re_path,
	reverse,
};

// Re-export admin panel (admin feature)
#[cfg(feature = "admin")]
pub use reinhardt_admin::panel::{
	// Actions
	ActionRegistry,
	ActionResult,
	// Dashboard
	Activity,
	AdminAction,
	// Auth
	AdminAuthBackend,
	AdminAuthMiddleware,
	// Templates
	AdminContext,
	// Database
	AdminDatabase,
	// Error types
	AdminError,
	// Forms
	AdminForm,
	AdminPermissionChecker,
	AdminResult,
	// Core types
	AdminSite,
	AdminTemplateRenderer,
	// Audit
	AuditAction,
	AuditLog,
	AuditLogBuilder,
	AuditLogQuery,
	AuditLogQueryBuilder,
	AuditLogger,
	// Filters
	BooleanFilter,
	// Advanced features
	BulkEdit,
	BulkEditConfig,
	BulkEditField,
	BulkEditForm,
	BulkEditResult,
	ChartData,
	ChartDataset,
	ChartType,
	ChartWidget,
	ChoiceFilter,
	// Views
	CreateView as AdminCreateView,
	// Export/Import
	CsvExporter,
	CsvImporter,
	// Custom views
	CustomView,
	CustomViewRegistry,
	DashboardContext,
	DashboardUserInfo,
	DashboardWidget,
	DatabaseAuditLogger,
	DateRangeFilter,
	DeleteConfirmationContext,
	DeleteSelectedAction,
	DeleteView as AdminDeleteView,
	DetailView as AdminDetailView,
	DragDropConfig,
	DragDropConfigBuilder,
	// Widgets
	EditorType,
	ExportBuilder,
	ExportConfig,
	ExportFormat,
	ExportResult,
	FieldType as AdminFieldType,
	FilterManager,
	FilterSpec,
	FormBuilder,
	FormField,
	FormViewContext,
	ImageFormat,
	ImageUploadConfig,
	ImportBuilder,
	ImportConfig,
	ImportError,
	ImportFormat,
	ImportResult,
	// Inline editing
	InlineForm,
	InlineFormset,
	InlineModelAdmin,
	InlineType,
	JsonExporter,
	JsonImporter,
	ListFilter,
	ListView as AdminListView,
	ListViewContext,
	MemoryAuditLogger,
	ModelAdmin,
	ModelAdminConfig,
	NumberRangeFilter,
	PaginationContext,
	PermissionAction,
	QuickLink,
	QuickLinksWidget,
	RecentActivityWidget,
	ReorderHandler,
	ReorderResult,
	ReorderableModel,
	RichTextEditorConfig,
	StatWidget,
	TableWidget,
	TsvExporter,
	TsvImporter,
	UpdateView as AdminUpdateView,
	UserContext,
	ViewConfig,
	ViewConfigBuilder,
	Widget,
	WidgetConfig,
	WidgetContext,
	WidgetFactory,
	WidgetPosition,
	WidgetRegistry,
	WidgetType,
};

// Re-export database related (database feature)
#[cfg(feature = "database")]
pub use reinhardt_db::contenttypes::{
	CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable,
	GenericRelationQuery, ModelType,
};
#[cfg(feature = "database")]
pub use reinhardt_db::migrations::{
	FieldState, Migration, MigrationAutodetector, MigrationError, MigrationExecutor, MigrationPlan,
	MigrationRecorder, ModelState, ProjectState,
};

// Re-export cache (cache feature)
#[cfg(feature = "cache")]
pub use reinhardt_utils::cache::{Cache, CacheKeyBuilder, InMemoryCache};

// Cache middleware is in reinhardt-middleware
#[cfg(feature = "middleware")]
pub use reinhardt_middleware::CacheMiddleware;

#[cfg(all(feature = "cache", feature = "redis-backend"))]
pub use reinhardt_utils::cache::RedisCache;

// Re-export sessions (sessions feature)
#[cfg(feature = "sessions")]
pub use reinhardt_auth::sessions::{
	CacheSessionBackend, InMemorySessionBackend, Session, SessionBackend, SessionError,
};

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_auth::sessions::{HttpSessionConfig, SameSite, SessionMiddleware};

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
pub use reinhardt_core::di::{Depends, DiError, DiResult, InjectionContext, RequestContext};

// Re-export DI params - available in minimal, standard, and di features
#[cfg(any(feature = "minimal", feature = "standard", feature = "di"))]
pub use reinhardt_core::di::params::{Body, Cookie, Header, Json, Path, Query};

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
pub use reinhardt_utils::storage::{InMemoryStorage, LocalStorage, Storage};

pub mod prelude {
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
		SingleObjectMixin,
		StatusCode,
		UnifiedRouter,
		View,
		ViewResult,
		ViewSet,
		// Routers
		clear_router,
		get_router,
		is_router_registered,
		register_router,
	};

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
		// Model derive macro and attribute macro
		Model,
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
		User,
		// User and group management
		UserManager,
	};

	// OpenAPI feature - schema generation and documentation
	// Note: When 'openapi' feature is enabled, types are available at top level
	// Example: use reinhardt::prelude::*; or use reinhardt::{OpenApi, ApiDoc, Schema};

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

	// Template renderer feature
	#[cfg(feature = "reinhardt-template")]
	pub use crate::JSONRenderer;

	// Settings feature
	#[cfg(feature = "conf")]
	pub use crate::Settings;

	// Middleware
	#[cfg(any(feature = "standard", feature = "middleware"))]
	pub use crate::LoggingMiddleware;

	// Sessions feature
	#[cfg(feature = "sessions")]
	pub use crate::{AuthenticationMiddleware, Session};

	// Cache feature
	#[cfg(feature = "cache")]
	pub use crate::{Cache, InMemoryCache};

	// Admin feature
	#[cfg(feature = "admin")]
	pub use crate::{
		// Actions
		ActionRegistry,
		AdminAction,
		AdminError,
		// Forms
		AdminFieldType as FieldType,
		AdminForm,
		AdminResult,
		AdminSite,
		// Filters
		BooleanFilter,
		ChoiceFilter,
		DateRangeFilter,
		DeleteSelectedAction,
		FormBuilder,
		FormField,
		ListFilter,
		ModelAdmin,
		ModelAdminConfig,
	};
}

// Re-export database modules for Model derive macro generated code
// These must be available at `::reinhardt::db::*` for the macro to work correctly
#[cfg(feature = "database")]
pub mod db {
	// Re-export commonly used types at module level for easier access
	pub use reinhardt_db::DatabaseConnection;

	// Explicitly re-export modules used by Model derive macro
	pub mod migrations {
		pub use reinhardt_db::migrations::*;
	}

	pub mod orm {
		pub use reinhardt_db::orm::*;
	}

	// Re-export associations module for relationship definitions
	pub mod associations {
		pub use reinhardt_db::associations::*;
	}

	// Re-export prelude for convenience
	pub mod prelude {
		pub use reinhardt_db::prelude::*;
	}
}
