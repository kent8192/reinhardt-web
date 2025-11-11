//! # Reinhardt Core
//!
//! Core components for the Reinhardt framework, providing fundamental types,
//! exception handling, signals, macros, security, and validation utilities.
//!
//! ## Available Validators
//!
//! The validators crate provides comprehensive validation utilities:
//! - **IPAddressValidator**: IPv4/IPv6 address validation
//! - **PhoneNumberValidator**: International phone number validation (E.164)
//! - **CreditCardValidator**: Credit card validation with Luhn algorithm
//! - **IBANValidator**: International bank account number validation
//! - **ColorValidator**: Hex, RGB, HSL color validation
//! - **FileTypeValidator**: MIME type and extension validation
//! - **CustomRegexValidator**: User-defined regex pattern validation
//!
//! ## Available Backend Implementations
//!
//! The backends crate provides multiple backend implementations:
//! - **Cache Backends**: Redis (✅), DynamoDB (✅), Memcached (✅)
//! - **Email Backends**: SMTP (✅), SendGrid (✅), AWS SES (✅), Mailgun (✅)
//! - **Queue Backends**: Redis (✅), RabbitMQ (✅), AWS SQS (✅)
//! - **Session Backends**: JWT (✅), Database (✅), Redis (✅), Cookie (✅), File (✅)
//! - **Storage Backends**: S3 (✅), Azure Blob (✅), GCS (✅), FileSystem (✅), Memory (✅)
//!
//! For detailed implementation and usage information, see the individual
//! crate documentation in `reinhardt-contrib`, `reinhardt-tasks`, `reinhardt-core/backends`.

#[cfg(feature = "types")]
pub use reinhardt_types as types;

#[cfg(feature = "exception")]
pub use reinhardt_exception as exception;

#[cfg(feature = "signals")]
pub use reinhardt_signals as signals;

#[cfg(feature = "macros")]
pub use reinhardt_macros as macros;

#[cfg(feature = "security")]
pub use reinhardt_security as security;

#[cfg(feature = "validators")]
pub use reinhardt_validators as validators;

#[cfg(feature = "serializers")]
pub use reinhardt_core_serializers as serializers;

#[cfg(feature = "auth")]
pub use reinhardt_core_auth as auth;

// New facade subcrate re-exports
#[cfg(feature = "negotiation")]
pub use reinhardt_negotiation as negotiation;

#[cfg(feature = "pagination")]
pub use reinhardt_pagination as pagination;

#[cfg(feature = "http")]
pub use reinhardt_http as http;

#[cfg(feature = "apps")]
pub use reinhardt_apps as apps;

#[cfg(feature = "di")]
pub use reinhardt_di as di;

#[cfg(feature = "parsers")]
pub use reinhardt_parsers as parsers;
