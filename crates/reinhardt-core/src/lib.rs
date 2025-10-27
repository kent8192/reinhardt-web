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
//! - **Cache Backends**: Redis (✅), DynamoDB (✅), Memcached (planned)
//! - **Email Backends**: SMTP (✅), SendGrid (✅), AWS SES (✅), Mailgun (✅)
//!
//! ## Planned Features
//!
//! The following enhancements are planned for future releases:
//! - **Session Backends**: Database, Redis, JWT
//! - **Storage Backends**: S3, Azure Blob, GCS
//! - **Queue Backends**: Redis, RabbitMQ, AWS SQS
//! - **Memcached Cache Backend**: Complete implementation
//!
//! For detailed implementation plans and design discussions, see the individual
//! crate documentation in `reinhardt-middleware`, `reinhardt-security`,
//! `reinhardt-validators`, and `reinhardt-backends`.

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
