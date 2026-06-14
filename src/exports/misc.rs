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

// `reinhardt-tasks`, `reinhardt-test`, `reinhardt-utils`, and `reinhardt-auth`
// are declared as dependencies only under
// `[target.'cfg(not(all(target_family = "wasm", target_os = "unknown")))'.dependencies]`
// (see Cargo.toml), i.e. they are native-only crates. Their feature flags
// (`tasks`, `test`, `storage`, `cache`, `sessions`, ...) can still be enabled on
// WASM, so each re-export below must be gated by `native` in addition to its
// feature — otherwise the path resolves to an unlinked crate and the build fails
// on `wasm32-unknown-unknown` (regression guarded by the WASM consumer fixture,
// reinhardt-web#4161).
#[cfg(all(feature = "tasks", native))]
pub use reinhardt_tasks::{Scheduler, Task, TaskExecutor, TaskQueue};

#[cfg(all(feature = "test", native))]
pub use reinhardt_test::{APIClient, APIRequestFactory, APITestCase, TestResponse};

#[cfg(all(feature = "storage", native))]
pub use reinhardt_utils::storage::{InMemoryStorage, LocalStorage, Storage};

#[cfg(all(feature = "cache", native))]
pub use reinhardt_utils::cache::{Cache, CacheKeyBuilder, InMemoryCache};

#[cfg(all(feature = "cache", feature = "redis-backend", native))]
pub use reinhardt_utils::cache::RedisCache;

// Sessions (gated by `sessions` feature, NOT `auth` — sessions can be
// used independently of the auth module). Still native-only: `reinhardt-auth`
// is a native-only dependency (see the note above).
#[cfg(all(feature = "sessions", native))]
pub use reinhardt_auth::sessions::{
	CacheSessionBackend, InMemorySessionBackend, Session, SessionBackend, SessionError,
};

#[cfg(all(feature = "sessions", feature = "middleware", native))]
pub use reinhardt_auth::sessions::{HttpSessionConfig, SameSite, SessionMiddleware};
