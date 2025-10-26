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
//! - `logging`: Logging utilities (feature: `logging`)
//! - `cache`: Caching utilities (feature: `cache`)
//! - `storage`: Storage utilities (feature: `storage`)
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

// Re-export utils-core modules
pub use utils_core::{dateformat, encoding, html, text, timezone};

// Re-export internal crates
#[cfg(feature = "logging")]
pub use reinhardt_logging as logging;

#[cfg(feature = "cache")]
pub use reinhardt_cache as cache;

#[cfg(feature = "storage")]
pub use reinhardt_storage as storage;

pub use dateformat::format as format_date;
pub use encoding::{escapejs, slugify, truncate_chars, truncate_words, urldecode, urlencode};
pub use html::{SafeString, escape, escape_attr, strip_tags, unescape};
pub use text::{capfirst, floatcomma, intcomma, ordinal, pluralize, title};
pub use timezone::{
    get_timezone_name_local, get_timezone_name_utc, localtime, now, to_local, to_utc,
};
