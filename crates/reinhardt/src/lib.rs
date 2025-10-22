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
    DatabaseBackend, DatabaseConnection, Model, SoftDeletable, SoftDelete, Timestamped, Timestamps,
};

// Re-export serializers
pub use reinhardt_serializers::{Deserializer, JsonSerializer, Serializer};

// Re-export viewsets
pub use reinhardt_viewsets::{
    Action, ActionType, CreateMixin, DestroyMixin, GenericViewSet, ListMixin, ModelViewSet,
    ReadOnlyModelViewSet, RetrieveMixin, UpdateMixin, ViewSet,
};

// Re-export routers
pub use reinhardt_routers::{DefaultRouter, PathMatcher, PathPattern, Route, Router};

// Re-export auth
pub use reinhardt_auth::{
    AllowAny, AnonymousUser, Argon2Hasher, AuthBackend, Claims, IsAdminUser, IsAuthenticated,
    JwtAuth, PasswordHasher, Permission, SimpleUser, User,
};

// Re-export middleware
pub use reinhardt_middleware::{AuthenticationMiddleware, CorsMiddleware, LoggingMiddleware};

// Re-export pagination
pub use reinhardt_pagination::{
    CursorPagination, LimitOffsetPagination, PageNumberPagination, PaginatedResponse, Paginator,
};

// Re-export filters
pub use reinhardt_filters::{
    FieldOrderingExt, FilterBackend, FilterError, FilterResult, MultiTermSearch,
};

// Re-export throttling
pub use reinhardt_throttling::{AnonRateThrottle, ScopedRateThrottle, Throttle, UserRateThrottle};

// Re-export signals
pub use reinhardt_signals::{
    m2m_changed, post_delete, post_save, pre_delete, pre_save, M2MAction, M2MChangeEvent, Signal,
};

// Re-export views
pub use reinhardt_views::{
    Context, DetailView, ListView, MultipleObjectMixin, SingleObjectMixin, View,
};

// Re-export parsers
pub use reinhardt_parsers::{
    FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
    Parser,
};

// Re-export renderers
pub use reinhardt_renderers::{BrowsableAPIRenderer, JSONRenderer, XMLRenderer};

// Re-export versioning
pub use reinhardt_versioning::{
    AcceptHeaderVersioning, BaseVersioning, HostNameVersioning, NamespaceVersioning,
    QueryParameterVersioning, RequestVersionExt, URLPathVersioning, VersioningError,
    VersioningMiddleware,
};

// Re-export metadata
pub use reinhardt_metadata::{
    ActionMetadata, BaseMetadata, ChoiceInfo, FieldInfo, FieldInfoBuilder, FieldType,
    MetadataOptions, MetadataResponse, SimpleMetadata,
};

// Re-export negotiation
pub use reinhardt_negotiation::*;

// Re-export REST integration
pub use reinhardt_rest::*;

// Re-export database related (database feature)
#[cfg(feature = "database")]
pub use reinhardt_contenttypes::{
    ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable, GenericRelationQuery,
    ModelType, CONTENT_TYPE_REGISTRY,
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
    Cache, CacheError, CacheKeyBuilder, CacheMiddleware, CacheMiddlewareConfig, CacheResult,
    CacheService, InMemoryCache, RedisCache,
};

// Re-export sessions (sessions feature)
#[cfg(feature = "sessions")]
pub use reinhardt_sessions::{
    CacheSessionBackend, InMemorySessionBackend, Session, SessionBackend, SessionConfig,
    SessionData, SessionError, SessionMiddleware, SessionResult, SessionService,
};

// Re-export contrib modules (contrib feature)
#[cfg(feature = "contrib")]
pub use reinhardt_contrib::contrib;

// Re-export common external dependencies
pub use async_trait::async_trait;
pub use serde::{Deserialize, Serialize};
pub use tokio;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        // External
        async_trait,
        m2m_changed,

        post_delete,
        post_save,
        pre_delete,
        pre_save,
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
        AuthenticationMiddleware,
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
        Throttle,

        Timestamped,
        URLPathVersioning,
        // Auth
        User,
        UserRateThrottle,
        VersioningMiddleware,

        // Views
        View,
        // ViewSets
        ViewSet,

        XMLRenderer,
    };

    // Cache (if enabled)
    #[cfg(feature = "cache")]
    pub use crate::{Cache, InMemoryCache};

    // Sessions (if enabled)
    #[cfg(feature = "sessions")]
    pub use crate::{InMemorySessionBackend, Session};

    pub use std::sync::Arc;
}
