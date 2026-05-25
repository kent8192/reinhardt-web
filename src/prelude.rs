//! Convenience re-exports for common usage patterns.

/// Wasm-side `prelude` shim (Issue #4189).
///
/// The native `prelude` module is `#[cfg(native)]`-gated, but the
/// `--with-pages` scaffold emits `use reinhardt::prelude::*;` in the
/// generated `src/config/urls.rs`, which must compile on
/// `wasm32-unknown-unknown` so wasm SPA consumers can `cargo check --lib`
/// without modifying the scaffolded sources.
///
/// This wasm-only stub re-exports the minimum surface the scaffold uses
/// (`UnifiedRouter` from the wasm-side `urls::prelude` shim, gated behind
/// the `client-router` feature; the module is empty when that feature is
/// disabled). Native builds keep using the full server-side prelude below.
#[cfg(all(not(native), target_family = "wasm"))]
mod wasm {
    #[cfg(feature = "client-router")]
    pub use crate::urls::prelude::UnifiedRouter;
}

#[cfg(all(not(native), target_family = "wasm"))]
pub use wasm::*;

#[cfg(native)]
mod server {
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
        clear_router,
        get_router,
        is_router_registered,
        register_router,
    };

    #[cfg(feature = "core")]
    pub use crate::ViewResult;

    #[cfg(feature = "client-router")]
    pub use crate::UnifiedRouter;

    #[cfg(feature = "core")]
    pub use crate::core::async_trait;
    #[cfg(feature = "core")]
    pub use crate::core::serde::{Deserialize, Serialize};

    #[cfg(feature = "core")]
    pub use crate::{
        Error, Handler, Middleware, MiddlewareChain, Request, Response, Result, Signal,
        m2m_changed, post_delete, post_save, pre_delete, pre_save,
    };

    pub use crate::{api_view, delete, get, patch, post, put};

    #[cfg(feature = "database")]
    pub use crate::{
        Aggregate,
        Annotation,
        CheckConstraint,
        Concat,
        CurrentDate,
        DatabaseConnection,
        DenseRank,
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
        Transaction,
        UniqueConstraint,
        Upper,
        Window,
        atomic,
        model,
    };

    #[cfg(feature = "database")]
    pub use reinhardt_db::orm::Model;

    #[cfg(feature = "auth")]
    pub use crate::{
        AuthBackend,
        AuthIdentity,
        Group,
        GroupManager,
        ObjectPermission,
        ObjectPermissionChecker,
        PasswordHasher,
        Permission,
        UserManager,
    };

    #[cfg(any(feature = "minimal", feature = "standard", feature = "di"))]
    pub use crate::{Body, Cookie, Header, Json, Path, Query};

    #[cfg(feature = "rest")]
    pub use crate::{
        AcceptHeaderVersioning,
        AnonRateThrottle,
        CursorPagination,
        FormParser,
        JSONParser,
        JsonSerializer,
        LimitOffsetPagination,
        MultiPartParser,
        MultiTermSearch,
        PageNumberPagination,
        Paginator,
        Parser,
        QueryParameterVersioning,
        ScopedRateThrottle,
        Serializer,
        SimpleMetadata,
        Throttle,
        URLPathVersioning,
        UserRateThrottle,
        VersioningMiddleware,
    };

    #[cfg(any(feature = "standard", feature = "middleware"))]
    pub use crate::LoggingMiddleware;

    #[cfg(feature = "middleware-security")]
    pub use crate::SecurityMiddleware;

    #[cfg(all(feature = "sessions", feature = "middleware", native))]
    pub use crate::AuthenticationMiddleware;
    #[cfg(feature = "sessions")]
    pub use crate::Session;

    #[cfg(feature = "cache")]
    pub use crate::{Cache, InMemoryCache};
}

#[cfg(native)]
pub use server::*;
