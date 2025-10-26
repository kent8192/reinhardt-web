//! Multiple serialization format support for sessions
//!
//! This module provides support for different serialization formats
//! including JSON, MessagePack, and more.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_sessions::serialization::{Serializer, JsonSerializer};
//! use serde_json::json;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let serializer = JsonSerializer;
//!
//! let data = json!({
//!     "user_id": 42,
//!     "username": "alice",
//! });
//!
//! // Serialize to bytes
//! let bytes = serializer.serialize(&data)?;
//!
//! // Deserialize back
//! let restored: serde_json::Value = serializer.deserialize(&bytes)?;
//! assert_eq!(restored["user_id"], 42);
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Serialization errors
#[derive(Debug, Error)]
pub enum SerializationError {
    /// JSON serialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// MessagePack serialization error
    #[cfg(feature = "messagepack")]
    #[error("MessagePack error: {0}")]
    MessagePackError(#[from] rmp_serde::encode::Error),

    /// MessagePack deserialization error
    #[cfg(feature = "messagepack")]
    #[error("MessagePack decode error: {0}")]
    MessagePackDecodeError(#[from] rmp_serde::decode::Error),

    /// Unsupported format
    #[error("Unsupported serialization format")]
    UnsupportedFormat,
}

/// Serializer trait for different formats
///
/// # Example
///
/// ```rust
/// use reinhardt_sessions::serialization::{Serializer, JsonSerializer};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct UserData {
///     id: i32,
///     name: String,
/// }
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let serializer = JsonSerializer;
/// let user = UserData { id: 1, name: "Alice".to_string() };
///
/// let bytes = serializer.serialize(&user)?;
/// let restored: UserData = serializer.deserialize(&bytes)?;
///
/// assert_eq!(user, restored);
/// # Ok(())
/// # }
/// ```
pub trait Serializer: Send + Sync {
    /// Serialize data to bytes
    fn serialize<T: Serialize>(&self, data: &T) -> Result<Vec<u8>, SerializationError>;

    /// Deserialize bytes to data
    fn deserialize<T: for<'de> Deserialize<'de>>(&self, bytes: &[u8])
        -> Result<T, SerializationError>;
}

/// JSON serializer
///
/// # Example
///
/// ```rust
/// use reinhardt_sessions::serialization::{Serializer, JsonSerializer};
/// use serde_json::json;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let serializer = JsonSerializer;
/// let data = json!({"key": "value"});
///
/// let bytes = serializer.serialize(&data)?;
/// let restored: serde_json::Value = serializer.deserialize(&bytes)?;
///
/// assert_eq!(restored["key"], "value");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn serialize<T: Serialize>(&self, data: &T) -> Result<Vec<u8>, SerializationError> {
        Ok(serde_json::to_vec(data)?)
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(
        &self,
        bytes: &[u8],
    ) -> Result<T, SerializationError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

/// MessagePack serializer (requires "messagepack" feature)
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_sessions::serialization::{Serializer, MessagePackSerializer};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Data {
///     value: String,
/// }
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let serializer = MessagePackSerializer;
/// let data = Data { value: "test".to_string() };
///
/// let bytes = serializer.serialize(&data)?;
/// let restored: Data = serializer.deserialize(&bytes)?;
///
/// assert_eq!(data, restored);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "messagepack")]
#[derive(Debug, Clone, Copy)]
pub struct MessagePackSerializer;

#[cfg(feature = "messagepack")]
impl Serializer for MessagePackSerializer {
    fn serialize<T: Serialize>(&self, data: &T) -> Result<Vec<u8>, SerializationError> {
        Ok(rmp_serde::to_vec(data)?)
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(
        &self,
        bytes: &[u8],
    ) -> Result<T, SerializationError> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}

/// Serialization format enum
///
/// # Example
///
/// ```rust
/// use reinhardt_sessions::serialization::SerializationFormat;
///
/// let format = SerializationFormat::Json;
/// assert_eq!(format.name(), "json");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// JSON format
    Json,
    /// MessagePack format
    #[cfg(feature = "messagepack")]
    MessagePack,
}

impl SerializationFormat {
    /// Get format name as string
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_sessions::serialization::SerializationFormat;
    ///
    /// assert_eq!(SerializationFormat::Json.name(), "json");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            SerializationFormat::Json => "json",
            #[cfg(feature = "messagepack")]
            SerializationFormat::MessagePack => "messagepack",
        }
    }

    /// Create serializer for this format
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_sessions::serialization::{SerializationFormat, Serializer};
    /// use serde_json::json;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let format = SerializationFormat::Json;
    /// let serializer = format.create_serializer();
    ///
    /// let data = json!({"test": true});
    /// let bytes = serializer.serialize(&data)?;
    /// let restored: serde_json::Value = serializer.deserialize(&bytes)?;
    ///
    /// assert_eq!(restored["test"], true);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_serializer(&self) -> Box<dyn Serializer> {
        match self {
            SerializationFormat::Json => Box::new(JsonSerializer),
            #[cfg(feature = "messagepack")]
            SerializationFormat::MessagePack => Box::new(MessagePackSerializer),
        }
    }
}

impl Default for SerializationFormat {
    /// Default serialization format is JSON
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_sessions::serialization::SerializationFormat;
    ///
    /// let format = SerializationFormat::default();
    /// assert_eq!(format, SerializationFormat::Json);
    /// ```
    fn default() -> Self {
        Self::Json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestData {
        id: i32,
        name: String,
        active: bool,
    }

    #[test]
    fn test_json_serializer() {
        let serializer = JsonSerializer;
        let data = TestData {
            id: 42,
            name: "test".to_string(),
            active: true,
        };

        let bytes = serializer.serialize(&data).unwrap();
        let restored: TestData = serializer.deserialize(&bytes).unwrap();

        assert_eq!(data, restored);
    }

    #[test]
    fn test_json_serializer_with_value() {
        let serializer = JsonSerializer;
        let data = serde_json::json!({
            "user_id": 123,
            "username": "alice",
            "roles": ["admin", "user"],
        });

        let bytes = serializer.serialize(&data).unwrap();
        let restored: serde_json::Value = serializer.deserialize(&bytes).unwrap();

        assert_eq!(restored["user_id"], 123);
        assert_eq!(restored["username"], "alice");
    }

    #[cfg(feature = "messagepack")]
    #[test]
    fn test_messagepack_serializer() {
        let serializer = MessagePackSerializer;
        let data = TestData {
            id: 99,
            name: "messagepack".to_string(),
            active: false,
        };

        let bytes = serializer.serialize(&data).unwrap();
        let restored: TestData = serializer.deserialize(&bytes).unwrap();

        assert_eq!(data, restored);
    }

    #[test]
    fn test_serialization_format_name() {
        assert_eq!(SerializationFormat::Json.name(), "json");

        #[cfg(feature = "messagepack")]
        assert_eq!(SerializationFormat::MessagePack.name(), "messagepack");
    }

    #[test]
    fn test_serialization_format_default() {
        let format = SerializationFormat::default();
        assert_eq!(format, SerializationFormat::Json);
    }

    #[test]
    fn test_serialization_format_create_serializer() {
        let format = SerializationFormat::Json;
        let serializer = format.create_serializer();

        let data = serde_json::json!({"test": "value"});
        let bytes = serializer.serialize(&data).unwrap();
        let restored: serde_json::Value = serializer.deserialize(&bytes).unwrap();

        assert_eq!(restored["test"], "value");
    }
}
