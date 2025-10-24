//! Validation support for parameter extraction
//!
//! This module provides validation capabilities for extracted parameters,
//! integrating with the `reinhardt-validators` crate.
//!
//! # Overview
//!
//! Reinhardt provides a powerful validation system that allows you to declaratively
//! specify constraints on path, query, and form parameters. The validation system
//! supports:
//!
//! - **Length constraints**: `min_length()`, `max_length()`
//! - **Numeric ranges**: `min_value()`, `max_value()`
//! - **Pattern matching**: `regex()`
//! - **Format validation**: `email()`, `url()`
//! - **Constraint chaining**: Combine multiple constraints with builder pattern
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use reinhardt_params::{Path, Query, WithValidation};
//!
//! // Extract and validate a path parameter
//! let id = Path::<i32>::from_request(req, ctx).await?;
//! let validated_id = id.min_value(1).max_value(1000);
//! validated_id.validate_number(&validated_id.0)?;
//!
//! // Extract and validate a query parameter
//! let email = Query::<String>::from_request(req, ctx).await?;
//! let validated_email = email.min_length(5).max_length(100).email();
//! validated_email.validate_string(&validated_email.0)?;
//! ```
//!
//! # Type Aliases
//!
//! For convenience, the module provides type aliases:
//!
//! - `ValidatedPath<T>` - Validated path parameters
//! - `ValidatedQuery<T>` - Validated query parameters
//! - `ValidatedForm<T>` - Validated form parameters
//!
//! # Examples
//!
//! ## Numeric Range Validation
//!
//! ```rust,ignore
//! use reinhardt_params::{Path, WithValidation};
//!
//! let age = Path(25);
//! let validated = age.min_value(0).max_value(120);
//!
//! assert!(validated.validate_number(&25).is_ok());
//! assert!(validated.validate_number(&150).is_err());
//! ```
//!
//! ## Email Validation
//!
//! ```rust,ignore
//! use reinhardt_params::{Query, WithValidation};
//!
//! let email = Query("user@example.com".to_string());
//! let validated = email.email();
//!
//! assert!(validated.validate_string("user@example.com").is_ok());
//! assert!(validated.validate_string("invalid").is_err());
//! ```
//!
//! ## Combined Constraints
//!
//! ```rust,ignore
//! use reinhardt_params::{Path, WithValidation};
//!
//! let username = Path("alice".to_string());
//! let validated = username
//!     .min_length(3)
//!     .max_length(20)
//!     .regex(r"^[a-zA-Z0-9_]+$");
//!
//! validated.validate_string(&validated.0)?;
//! ```
//!
//! # Error Handling
//!
//! Validation errors are returned as `ValidationError` from the `reinhardt-validators`
//! crate, which provides detailed error messages including:
//!
//! - The constraint that failed (e.g., "too short", "too large")
//! - The actual value
//! - The expected constraint (e.g., minimum, maximum)
//!
//! Example error message:
//! ```text
//! Validation error for 'email': Length too short: 3 (minimum: 5)
//! ```

#[cfg(feature = "validation")]
use reinhardt_validators::{ValidationResult, Validator};
#[cfg(feature = "validation")]
use std::fmt::{self, Debug};
use std::ops::Deref;

/// A validated wrapper for extracted parameters
///
/// This type wraps an extracted parameter and ensures validation is performed.
/// It requires the `validation` feature to be enabled.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_params::{Path, Validated};
/// use reinhardt_validators::MinLengthValidator;
///
/// async fn handler(id: Validated<Path<String>, MinLengthValidator>) {
///     // id is guaranteed to meet the validation constraints
///     let value = id.into_inner().0;
/// }
/// ```
#[cfg(feature = "validation")]
pub struct Validated<T, V> {
    inner: T,
    _validator: std::marker::PhantomData<V>,
}

#[cfg(feature = "validation")]
impl<T, V> Validated<T, V> {
    /// Create a new validated value
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    pub fn new<U>(inner: T, validator: &V) -> Result<Self, crate::ParamError>
    where
        V: Validator<U>,
        T: AsRef<U>,
        U: ?Sized,
    {
        validator
            .validate(inner.as_ref())
            .map_err(|e| crate::ParamError::ValidationError {
                name: "parameter".to_string(),
                message: e.to_string(),
            })?;

        Ok(Self {
            inner,
            _validator: std::marker::PhantomData,
        })
    }

    /// Unwrap the validated value
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(feature = "validation")]
impl<T, V> Deref for Validated<T, V> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "validation")]
impl<T: Debug, V> Debug for Validated<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

// Feature-gated trait is defined at the end of the file for non-validation builds

/// Validation constraints for a parameter
#[cfg(feature = "validation")]
pub struct ValidationConstraints<T> {
    inner: T,
    min_length: Option<usize>,
    max_length: Option<usize>,
    min_value: Option<String>,
    max_value: Option<String>,
    regex: Option<String>,
    email: bool,
    url: bool,
}

#[cfg(feature = "validation")]
impl<T> ValidationConstraints<T> {
    /// Add another min_length constraint
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Add another max_length constraint
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Add another min_value constraint
    pub fn min_value<V: ToString>(mut self, min: V) -> Self {
        self.min_value = Some(min.to_string());
        self
    }

    /// Add another max_value constraint
    pub fn max_value<V: ToString>(mut self, max: V) -> Self {
        self.max_value = Some(max.to_string());
        self
    }

    /// Add regex constraint
    pub fn regex(mut self, pattern: impl Into<String>) -> Self {
        self.regex = Some(pattern.into());
        self
    }

    /// Add email validation
    pub fn email(mut self) -> Self {
        self.email = true;
        self
    }

    /// Add URL validation
    pub fn url(mut self) -> Self {
        self.url = true;
        self
    }

    /// Validate a string value against the constraints
    pub fn validate_string(&self, value: &str) -> ValidationResult<()> {
        // Length constraints
        if let Some(min) = self.min_length {
            reinhardt_validators::MinLengthValidator::new(min).validate(value)?;
        }
        if let Some(max) = self.max_length {
            reinhardt_validators::MaxLengthValidator::new(max).validate(value)?;
        }

        // Regex constraint
        if let Some(ref pattern) = self.regex {
            reinhardt_validators::RegexValidator::new(pattern)
                .map_err(|e| {
                    reinhardt_validators::ValidationError::Custom(format!(
                        "Invalid regex pattern: {}",
                        e
                    ))
                })?
                .validate(value)?;
        }

        // Email constraint
        if self.email {
            reinhardt_validators::EmailValidator::new().validate(value)?;
        }

        // URL constraint
        if self.url {
            reinhardt_validators::UrlValidator::new().validate(value)?;
        }

        Ok(())
    }

    /// Validate a numeric value against the constraints
    pub fn validate_number<N>(&self, value: &N) -> ValidationResult<()>
    where
        N: PartialOrd + std::fmt::Display + Clone + std::str::FromStr,
        <N as std::str::FromStr>::Err: std::fmt::Display,
    {
        if let Some(ref min_str) = self.min_value {
            if let Ok(min) = min_str.parse::<N>() {
                reinhardt_validators::MinValueValidator::new(min).validate(value)?;
            }
        }
        if let Some(ref max_str) = self.max_value {
            if let Ok(max) = max_str.parse::<N>() {
                reinhardt_validators::MaxValueValidator::new(max).validate(value)?;
            }
        }
        Ok(())
    }

    /// Get the inner value
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(feature = "validation")]
impl<T> Deref for ValidationConstraints<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// ============================================================================
// WithValidation Trait (feature-gated)
// ============================================================================

/// Trait for adding validation constraints to parameters
///
/// This trait is enabled with the `validation` feature flag.
#[cfg(feature = "validation")]
pub trait WithValidation: Sized {
    /// Add minimum length constraint
    fn min_length(self, min: usize) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: Some(min),
            max_length: None,
            min_value: None,
            max_value: None,
            regex: None,
            email: false,
            url: false,
        }
    }

    /// Add maximum length constraint
    fn max_length(self, max: usize) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: Some(max),
            min_value: None,
            max_value: None,
            regex: None,
            email: false,
            url: false,
        }
    }

    /// Add minimum value constraint
    fn min_value<V: ToString>(self, min: V) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: Some(min.to_string()),
            max_value: None,
            regex: None,
            email: false,
            url: false,
        }
    }

    /// Add maximum value constraint
    fn max_value<V: ToString>(self, max: V) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: Some(max.to_string()),
            regex: None,
            email: false,
            url: false,
        }
    }

    /// Add regex pattern constraint
    fn regex(self, pattern: impl Into<String>) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            regex: Some(pattern.into()),
            email: false,
            url: false,
        }
    }

    /// Add email validation
    fn email(self) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            regex: None,
            email: true,
            url: false,
        }
    }

    /// Add URL validation
    fn url(self) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            regex: None,
            email: false,
            url: true,
        }
    }
}

// WithValidation implementations are provided in their respective modules:
// - Path<T>: path.rs
// - Query<T>: query.rs
// - Form<T>: form.rs

// ============================================================================
// Type Aliases for Validated Parameters
// ============================================================================

/// Type alias for validated path parameters
///
/// This is a convenience type that wraps a Path<T> with validation constraints.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_params::{ValidatedPath, WithValidation};
///
// In your handler:
/// async fn handler(
///     // Extract path parameter "id" and validate it
///     id: ValidatedPath<i32>,
/// ) {
///     // Use the validated value
///     let value = id.0;
/// }
///
// Usage pattern:
// 1. Extract Path<T> from request
// 2. Apply validation constraints
// 3. Validate
/// let path = Path::<i32>::from_request(req, ctx).await?;
/// let validated = path.min_value(1).max_value(100);
/// validated.validate_number(&validated.0)?;
/// ```
#[cfg(feature = "validation")]
pub type ValidatedPath<T> = ValidationConstraints<crate::Path<T>>;

/// Type alias for validated query parameters
///
/// This is a convenience type that wraps a Query<T> with validation constraints.
#[cfg(feature = "validation")]
pub type ValidatedQuery<T> = ValidationConstraints<crate::Query<T>>;

/// Type alias for validated form parameters
///
/// This is a convenience type that wraps a Form<T> with validation constraints.
#[cfg(feature = "validation")]
pub type ValidatedForm<T> = ValidationConstraints<crate::Form<T>>;

// ============================================================================
// Non-feature-gated versions for testing
// ============================================================================

#[cfg(not(feature = "validation"))]
pub struct ValidationConstraints<T> {
    pub inner: T,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub regex: Option<String>,
    pub email: bool,
    pub url: bool,
}

#[cfg(not(feature = "validation"))]
impl<T> ValidationConstraints<T> {
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    pub fn min_value<V: ToString>(mut self, min: V) -> Self {
        self.min_value = Some(min.to_string());
        self
    }

    pub fn max_value<V: ToString>(mut self, max: V) -> Self {
        self.max_value = Some(max.to_string());
        self
    }

    pub fn regex(mut self, pattern: impl Into<String>) -> Self {
        self.regex = Some(pattern.into());
        self
    }

    pub fn email(mut self) -> Self {
        self.email = true;
        self
    }

    pub fn url(mut self) -> Self {
        self.url = true;
        self
    }

    pub fn validate_string(&self, value: &str) -> Result<(), String> {
        if let Some(min) = self.min_length {
            if value.len() < min {
                return Err(format!(
                    "String length {} is less than minimum {}",
                    value.len(),
                    min
                ));
            }
        }
        if let Some(max) = self.max_length {
            if value.len() > max {
                return Err(format!(
                    "String length {} exceeds maximum {}",
                    value.len(),
                    max
                ));
            }
        }
        if let Some(ref pattern) = self.regex {
            use regex::Regex;
            let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
            if !regex.is_match(value) {
                return Err(format!("String does not match pattern: {}", pattern));
            }
        }
        if self.email {
            if !value.contains('@') || !value.contains('.') {
                return Err("Invalid email format".to_string());
            }
            let parts: Vec<&str> = value.split('@').collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                return Err("Invalid email format".to_string());
            }
        }
        if self.url {
            if !value.starts_with("http://") && !value.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
        }
        Ok(())
    }

    pub fn validate_number<N>(&self, value: &N) -> Result<(), String>
    where
        N: PartialOrd + std::fmt::Display + Clone + std::str::FromStr,
        <N as std::str::FromStr>::Err: std::fmt::Display,
    {
        if let Some(ref min_str) = self.min_value {
            if let Ok(min) = min_str.parse::<N>() {
                if value < &min {
                    return Err(format!("Value {} is less than minimum {}", value, min));
                }
            }
        }
        if let Some(ref max_str) = self.max_value {
            if let Ok(max) = max_str.parse::<N>() {
                if value > &max {
                    return Err(format!("Value {} exceeds maximum {}", value, max));
                }
            }
        }
        Ok(())
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(not(feature = "validation"))]
impl<T> Deref for ValidationConstraints<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(not(feature = "validation"))]
pub type ValidatedPath<T> = ValidationConstraints<crate::Path<T>>;

#[cfg(not(feature = "validation"))]
pub type ValidatedQuery<T> = ValidationConstraints<crate::Query<T>>;

#[cfg(not(feature = "validation"))]
pub type ValidatedForm<T> = ValidationConstraints<crate::Form<T>>;

// Implement WithValidation trait for Path and Query
#[cfg(not(feature = "validation"))]
impl<T> WithValidation for crate::Path<T> {}

#[cfg(not(feature = "validation"))]
impl<T> WithValidation for crate::Query<T> {}

// Implement non-feature-gated WithValidation trait
#[cfg(not(feature = "validation"))]
pub trait WithValidation: Sized {
    fn min_length(self, min: usize) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: Some(min),
            max_length: None,
            min_value: None,
            max_value: None,
            regex: None,
            email: false,
            url: false,
        }
    }

    fn max_length(self, max: usize) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: Some(max),
            min_value: None,
            max_value: None,
            regex: None,
            email: false,
            url: false,
        }
    }

    fn min_value<V: ToString>(self, min: V) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: Some(min.to_string()),
            max_value: None,
            regex: None,
            email: false,
            url: false,
        }
    }

    fn max_value<V: ToString>(self, max: V) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: Some(max.to_string()),
            regex: None,
            email: false,
            url: false,
        }
    }

    fn regex(self, pattern: impl Into<String>) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            regex: Some(pattern.into()),
            email: false,
            url: false,
        }
    }

    fn email(self) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            regex: None,
            email: true,
            url: false,
        }
    }

    fn url(self) -> ValidationConstraints<Self> {
        ValidationConstraints {
            inner: self,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            regex: None,
            email: false,
            url: true,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "validation")]
mod tests {
    use super::*;
    use crate::Path;

    #[test]
    fn test_validation_constraints_builder() {
        let path = Path(42i32);
        let constrained = path.min_value(0).max_value(100);

        assert!(constrained.validate_number(&42).is_ok());
        assert!(constrained.validate_number(&-1).is_err());
        assert!(constrained.validate_number(&101).is_err());
    }

    #[test]
    fn test_string_validation_constraints() {
        let path = Path("test".to_string());
        let constrained = path.min_length(2).max_length(10);

        assert!(constrained.validate_string("test").is_ok());
        assert!(constrained.validate_string("a").is_err());
        assert!(constrained.validate_string("this is too long").is_err());
    }
}
