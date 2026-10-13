#![warn(missing_docs)]
//! # Reinhardt Utils
//!
//! Utility functions for Reinhardt framework, inspired by Django's utils module.
//!
//! ## Modules
//!
//! - `timezone`: Timezone-aware datetime handling
//! - `dateformat`: Date and time formatting utilities
//! - `html`: HTML escaping and manipulation
//! - `encoding`: Text encoding and URL encoding
//! - `text`: Text manipulation utilities
//! - `humanize`: Human-friendly formatting utilities
//! - `logging`: Logging utilities (feature: `logging`)
//! - `cache`: Caching utilities (feature: `cache`)
//! - `storage`: Storage utilities (feature: `storage`)
//! - `staticfiles`: Static file serving utilities (feature: `staticfiles`)
//!
//! ## Browser WASM
//!
//! On `wasm32-unknown-unknown`, only the shared utilities (`utils_core`,
//! `humanize`, and their re-exports) are compiled. `logging`, `cache`,
//! `storage`, and `staticfiles` depend on the native runtime (filesystem,
//! Tokio networking, and HTTP serving) and are not available there. Within
//! `utils_core`, `safe_path_join`/`PathTraversalError` (filesystem
//! canonicalization) and `timezone::make_aware_local` (DST gap/overlap
//! detection) are also native-only. All of these APIs have **Parity: P0**
//! (target-only behavior): naming them in a browser WASM build fails at
//! compile time instead of diverging at runtime.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_utils::{timezone, dateformat, html, encoding, text};
//!
//! // Timezone
//! let now = timezone::now();
//! let formatted = dateformat::format(&now, "Y-m-d H:i:s");
//!
//! // HTML
//! let escaped = html::escape("<script>alert('XSS')</script>");
//!
//! // Encoding
//! let slug = encoding::slugify("Hello World");
//!
//! // Text
//! let ordinal = text::ordinal(1); // "1st"
//! ```

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub mod cache;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub mod logging;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub mod staticfiles;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub mod storage;
pub mod utils_core;

// Core modules
pub mod humanize;

// Re-export utils-core modules
pub use crate::utils_core::{dateformat, encoding, html, text, timezone};

pub use crate::utils_core::input_validation::{
	IdentifierError, sanitize_log_input, validate_identifier, validate_redirect_url,
};
pub use crate::utils_core::lock_recovery::{
	LockRecoveryError, recover_mutex, recover_rwlock_read, recover_rwlock_write,
};
pub use crate::utils_core::path_safety::is_safe_filename_component;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub use crate::utils_core::path_safety::{PathTraversalError, safe_path_join};
pub use dateformat::format as format_date;
pub use encoding::{escapejs, slugify, truncate_chars, truncate_words, urldecode, urlencode};
pub use html::{SafeString, escape, escape_attr, strip_tags, unescape};
pub use text::{capfirst, floatcomma, intcomma, ordinal, pluralize, title};
pub use timezone::{
	get_timezone_name_local, get_timezone_name_utc, localtime, now, to_local, to_utc,
};
