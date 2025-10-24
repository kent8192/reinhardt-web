//! Serializer trait and implementations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait Serializer {
    type Input;
    type Output;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError>;
    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError>;
}

/// Error type for validation failures
///
/// Provides detailed information about what validation rule failed,
/// which field(s) were involved, and what values caused the failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorError {
    /// A field value failed uniqueness validation
    UniqueViolation {
        field_name: String,
        value: String,
        message: String,
    },
    /// A combination of field values failed unique_together validation
    UniqueTogetherViolation {
        field_names: Vec<String>,
        values: HashMap<String, String>,
        message: String,
    },
    /// A required field is missing
    RequiredField { field_name: String, message: String },
    /// Field validation failed (e.g., regex, range check)
    FieldValidation {
        field_name: String,
        value: String,
        constraint: String,
        message: String,
    },
    /// Database connection or query error during validation
    DatabaseError {
        message: String,
        source: Option<String>,
    },
    /// Generic validation error
    Custom { message: String },
}

impl ValidatorError {
    /// Get the primary error message
    pub fn message(&self) -> &str {
        match self {
            ValidatorError::UniqueViolation { message, .. } => message,
            ValidatorError::UniqueTogetherViolation { message, .. } => message,
            ValidatorError::RequiredField { message, .. } => message,
            ValidatorError::FieldValidation { message, .. } => message,
            ValidatorError::DatabaseError { message, .. } => message,
            ValidatorError::Custom { message } => message,
        }
    }

    /// Get the field name(s) involved in the error
    pub fn field_names(&self) -> Vec<&str> {
        match self {
            ValidatorError::UniqueViolation { field_name, .. } => vec![field_name.as_str()],
            ValidatorError::UniqueTogetherViolation { field_names, .. } => {
                field_names.iter().map(|s| s.as_str()).collect()
            }
            ValidatorError::RequiredField { field_name, .. } => vec![field_name.as_str()],
            ValidatorError::FieldValidation { field_name, .. } => vec![field_name.as_str()],
            ValidatorError::DatabaseError { .. } => vec![],
            ValidatorError::Custom { .. } => vec![],
        }
    }

    /// Check if this error is related to database operations
    pub fn is_database_error(&self) -> bool {
        matches!(self, ValidatorError::DatabaseError { .. })
    }

    /// Check if this error is a uniqueness violation
    pub fn is_uniqueness_violation(&self) -> bool {
        matches!(
            self,
            ValidatorError::UniqueViolation { .. } | ValidatorError::UniqueTogetherViolation { .. }
        )
    }
}

impl std::fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidatorError::UniqueViolation {
                field_name,
                value,
                message,
            } => {
                write!(
                    f,
                    "Unique violation on field '{}' with value '{}': {}",
                    field_name, value, message
                )
            }
            ValidatorError::UniqueTogetherViolation {
                field_names,
                values,
                message,
            } => {
                let fields = field_names.join(", ");
                let vals: Vec<String> = field_names
                    .iter()
                    .filter_map(|name| values.get(name).map(|v| format!("{}={}", name, v)))
                    .collect();
                write!(
                    f,
                    "Unique together violation on fields [{}] with values ({}): {}",
                    fields,
                    vals.join(", "),
                    message
                )
            }
            ValidatorError::RequiredField {
                field_name,
                message,
            } => {
                write!(f, "Required field '{}': {}", field_name, message)
            }
            ValidatorError::FieldValidation {
                field_name,
                value,
                constraint,
                message,
            } => {
                write!(
                    f,
                    "Field '{}' with value '{}' failed constraint '{}': {}",
                    field_name, value, constraint, message
                )
            }
            ValidatorError::DatabaseError { message, source } => {
                if let Some(src) = source {
                    write!(f, "Database error: {} (source: {})", message, src)
                } else {
                    write!(f, "Database error: {}", message)
                }
            }
            ValidatorError::Custom { message } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for ValidatorError {}

/// Error type for serialization operations
#[derive(Debug, Clone)]
pub enum SerializerError {
    /// Validation error with detailed information
    Validation(ValidatorError),
    /// Serialization/deserialization error
    Serde { message: String },
    /// Generic error with message
    Other { message: String },
}

impl SerializerError {
    /// Create a new generic error
    pub fn new(message: String) -> Self {
        Self::Other { message }
    }

    /// Create a validation error
    pub fn validation(error: ValidatorError) -> Self {
        Self::Validation(error)
    }

    /// Create a unique violation error
    pub fn unique_violation(field_name: String, value: String, message: String) -> Self {
        Self::Validation(ValidatorError::UniqueViolation {
            field_name,
            value,
            message,
        })
    }

    /// Create a unique_together violation error
    pub fn unique_together_violation(
        field_names: Vec<String>,
        values: HashMap<String, String>,
        message: String,
    ) -> Self {
        Self::Validation(ValidatorError::UniqueTogetherViolation {
            field_names,
            values,
            message,
        })
    }

    /// Create a required field error
    pub fn required_field(field_name: String, message: String) -> Self {
        Self::Validation(ValidatorError::RequiredField {
            field_name,
            message,
        })
    }

    /// Create a database error
    pub fn database_error(message: String, source: Option<String>) -> Self {
        Self::Validation(ValidatorError::DatabaseError { message, source })
    }

    /// Get the error message
    pub fn message(&self) -> String {
        match self {
            SerializerError::Validation(err) => err.message().to_string(),
            SerializerError::Serde { message } => message.clone(),
            SerializerError::Other { message } => message.clone(),
        }
    }

    /// Check if this is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(self, SerializerError::Validation(_))
    }

    /// Get the underlying ValidatorError if this is a validation error
    pub fn as_validator_error(&self) -> Option<&ValidatorError> {
        match self {
            SerializerError::Validation(err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for SerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializerError::Validation(err) => write!(f, "{}", err),
            SerializerError::Serde { message } => write!(f, "Serde error: {}", message),
            SerializerError::Other { message } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for SerializerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SerializerError::Validation(err) => Some(err),
            _ => None,
        }
    }
}

impl From<SerializerError> for reinhardt_apps::Error {
    fn from(e: SerializerError) -> Self {
        reinhardt_apps::Error::Internal(e.message())
    }
}

pub struct JsonSerializer<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> JsonSerializer<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Default for JsonSerializer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Serializer for JsonSerializer<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    type Input = T;
    type Output = String;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError> {
        serde_json::to_string(input).map_err(|e| SerializerError::Serde {
            message: format!("Serialization error: {}", e),
        })
    }

    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError> {
        serde_json::from_str(output).map_err(|e| SerializerError::Serde {
            message: format!("Deserialization error: {}", e),
        })
    }
}

pub trait Deserializer {
    type Input;
    type Output;

    fn deserialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError>;
}
