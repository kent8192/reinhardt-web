//! Miscellaneous re-exports (tasks, test, storage, cache).

// Third-party trait re-exports for user convenience
#[cfg(native)]
pub use async_trait::async_trait;
// `serde` is not a direct dependency of this crate; it is re-exported through
// `crate::core::serde`. The `crate::core` module is gated by
// `all(feature = "core", native)` (see lib.rs), so this re-export must use the
// same gate — otherwise it fails to compile on WASM, where `crate::core` is
// absent even when the `core` feature is enabled.
#[cfg(all(feature = "core", native))]
pub use crate::core::serde::{Deserialize, Serialize};

#[cfg(feature = "tasks")]
pub use reinhardt_tasks::{Scheduler, Task, TaskExecutor, TaskQueue};

#[cfg(feature = "test")]
pub use reinhardt_test::{APIClient, APIRequestFactory, APITestCase, TestResponse};

#[cfg(feature = "storage")]
pub use reinhardt_utils::storage::{InMemoryStorage, LocalStorage, Storage};

#[cfg(feature = "cache")]
pub use reinhardt_utils::cache::{Cache, CacheKeyBuilder, InMemoryCache};

#[cfg(all(feature = "cache", feature = "redis-backend"))]
pub use reinhardt_utils::cache::RedisCache;

// Sessions (gated by `sessions` feature, NOT `auth` — sessions can be
// used independently of the auth module)
#[cfg(feature = "sessions")]
pub use reinhardt_auth::sessions::{
	CacheSessionBackend, InMemorySessionBackend, Session, SessionBackend, SessionError,
};

#[cfg(all(feature = "sessions", feature = "middleware"))]
pub use reinhardt_auth::sessions::{HttpSessionConfig, SameSite, SessionMiddleware};
