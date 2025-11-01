//! # Reinhardt Contrib
//!
//! This crate provides optional Django-inspired contrib modules for Reinhardt.
//!
//! ## Available Modules
//!
//! - `auth` - Authentication and authorization
//! - `contenttypes` - Content type framework
//! - `sessions` - Session management
//! - `messages` - User messaging framework
//! - `static` - Static file serving
//! - `mail` - Email sending
//! - `graphql` - GraphQL support
//! - `websockets` - WebSocket support
//! - `i18n` - Internationalization
//! - `commands` - Management commands
//!
//! ## Usage
//!
//! Enable specific modules via features:
//!
//! ```toml
//! [dependencies]
//! reinhardt-contrib = { version = "0.1.0", features = ["auth", "sessions"] }
//! ```
//!
//! Or enable all modules:
//!
//! ```toml
//! [dependencies]
//! reinhardt-contrib = { version = "0.1.0", features = ["full"] }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Re-export all internal crates when their features are enabled

#[cfg(feature = "auth")]
pub use reinhardt_auth as auth;

#[cfg(feature = "contenttypes")]
pub use reinhardt_contenttypes as contenttypes;

#[cfg(feature = "sessions")]
pub use reinhardt_sessions as sessions;

#[cfg(feature = "messages")]
pub use reinhardt_messages as messages;

#[cfg(feature = "static")]
pub use reinhardt_static as r#static;

#[cfg(feature = "mail")]
pub use reinhardt_mail as mail;

#[cfg(feature = "graphql")]
pub use reinhardt_graphql as graphql;

#[cfg(feature = "websockets")]
pub use reinhardt_websockets as websockets;

#[cfg(feature = "i18n")]
pub use reinhardt_i18n as i18n;

/// Humanize utilities (from reinhardt-utils)
pub mod humanize {
	use chrono::{DateTime, Duration, Utc};

	pub use reinhardt_utils::text::{intcomma, ordinal, pluralize};

	/// Convert a large number to a word representation
	pub fn intword(n: i64) -> String {
		if n.abs() >= 1_000_000_000_000 {
			let val = n as f64 / 1_000_000_000_000.0;
			if val.fract() == 0.0 {
				format!("{} trillion", val as i64)
			} else {
				format!("{:.1} trillion", val)
			}
		} else if n.abs() >= 1_000_000_000 {
			let val = n as f64 / 1_000_000_000.0;
			if val.fract() == 0.0 {
				format!("{} billion", val as i64)
			} else {
				format!("{:.1} billion", val)
			}
		} else if n.abs() >= 1_000_000 {
			let val = n as f64 / 1_000_000.0;
			if val.fract() == 0.0 {
				format!("{} million", val as i64)
			} else {
				format!("{:.1} million", val)
			}
		} else if n.abs() >= 1_000 {
			let val = n as f64 / 1_000.0;
			if val.fract() == 0.0 {
				format!("{} thousand", val as i64)
			} else {
				format!("{:.1} thousand", val)
			}
		} else {
			n.to_string()
		}
	}

	/// Format file size in human-readable format
	pub fn filesizeformat(bytes: u64) -> String {
		const KB: u64 = 1024;
		const MB: u64 = KB * 1024;
		const GB: u64 = MB * 1024;

		if bytes == 0 {
			"0 bytes".to_string()
		} else if bytes == 1 {
			"1 byte".to_string()
		} else if bytes < KB {
			format!("{} bytes", bytes)
		} else if bytes < MB {
			format!("{:.1} KB", bytes as f64 / KB as f64)
		} else if bytes < GB {
			format!("{:.1} MB", bytes as f64 / MB as f64)
		} else {
			format!("{:.1} GB", bytes as f64 / GB as f64)
		}
	}

	/// Format a date as "today", "yesterday", "tomorrow", or the actual date
	pub fn naturalday(dt: &DateTime<Utc>) -> String {
		let now = Utc::now();
		let today = now.date_naive();
		let dt_date = dt.date_naive();

		if dt_date == today {
			"today".to_string()
		} else if dt_date == today - Duration::days(1) {
			"yesterday".to_string()
		} else if dt_date == today + Duration::days(1) {
			"tomorrow".to_string()
		} else {
			dt.format("%b %d, %Y").to_string()
		}
	}

	/// Format a datetime as natural language (e.g., "3 seconds ago", "2 hours from now")
	pub fn naturaltime(dt: &DateTime<Utc>) -> String {
		let now = Utc::now();
		let diff = now.signed_duration_since(*dt);

		if diff.num_seconds() < 60 && diff.num_seconds() >= 0 {
			if diff.num_seconds() < 10 {
				"a few seconds ago".to_string()
			} else {
				format!("{} seconds ago", diff.num_seconds())
			}
		} else if diff.num_seconds() < 0 {
			let future_diff = dt.signed_duration_since(now);
			if future_diff.num_seconds() < 60 {
				"a few seconds from now".to_string()
			} else if future_diff.num_minutes() < 60 {
				format!("{} minutes from now", future_diff.num_minutes())
			} else if future_diff.num_hours() < 24 {
				format!("{} hours from now", future_diff.num_hours())
			} else {
				format!("{} days from now", future_diff.num_days())
			}
		} else if diff.num_minutes() < 60 {
			format!("{} minutes ago", diff.num_minutes())
		} else if diff.num_hours() < 24 {
			format!("{} hours ago", diff.num_hours())
		} else {
			format!("{} days ago", diff.num_days())
		}
	}

	/// Calculate time since a given datetime
	pub fn timesince(dt: &DateTime<Utc>) -> String {
		let now = Utc::now();
		let diff = now.signed_duration_since(*dt);

		if diff.num_days() > 0 {
			if diff.num_days() == 1 {
				"1 day".to_string()
			} else {
				format!("{} days", diff.num_days())
			}
		} else if diff.num_hours() > 0 {
			if diff.num_hours() == 1 {
				"1 hour".to_string()
			} else {
				format!("{} hours", diff.num_hours())
			}
		} else if diff.num_minutes() > 0 {
			if diff.num_minutes() == 1 {
				"1 minute".to_string()
			} else {
				format!("{} minutes", diff.num_minutes())
			}
		} else {
			"less than a minute".to_string()
		}
	}
}
