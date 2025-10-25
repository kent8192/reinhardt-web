//! Integration tests for reinhardt-messages
//!
//! These tests verify the functionality of reinhardt-messages both standalone and
//! in integration with other crates.
//!
//! ## Test Organization
//!
//! ### ✅ Implemented Tests (runnable without #[ignore])
//!
//! - `test_storage_basic.rs` - Basic MemoryStorage tests (15 tests)
//! - `test_api.rs` - Basic message API tests (3 tests)
//!
//! ### ⏳ Future Integration Tests (marked as #[ignore])
//!
//! - `test_api.rs` - Full HTTP/middleware integration tests (5 tests)
//! - `test_middleware.rs` - Middleware integration tests (requires reinhardt-middleware)
//! - `test_cookie_storage.rs` - Cookie storage backend tests (requires cookie support)
//! - `test_session_storage.rs` - Session storage backend tests (requires reinhardt-sessions)
//! - `test_fallback_storage.rs` - Fallback storage tests (requires both cookie and session)
//! - `test_views.rs` - View-related tests (requires reinhardt-views, reinhardt-templates)
//! - `test_settings.rs` - Configuration tests (requires settings system)
//! - `test_assertions.rs` - Test helper tests (requires reinhardt-test)
//!
//! ## Running Tests
//!
//! Run only implemented tests:
//! ```bash
//! cargo test -p reinhardt-integration-tests --test messages_integration
//! ```
//!
//! Run all tests including ignored ones:
//! ```bash
//! cargo test -p reinhardt-integration-tests --test messages_integration -- --ignored
//! ```
//!
//! List all available tests:
//! ```bash
//! cargo test -p reinhardt-integration-tests --test messages_integration -- --list
//! ```
