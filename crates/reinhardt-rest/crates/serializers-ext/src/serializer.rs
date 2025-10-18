//! Serializer trait and implementations

use serde::{Deserialize, Serialize};

pub trait Serializer {
    type Input;
    type Output;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError>;
    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError>;
}

#[derive(Debug, Clone)]
pub struct SerializerError {
    pub message: String,
}

impl SerializerError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for SerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SerializerError {}

impl From<SerializerError> for reinhardt_apps::Error {
    fn from(e: SerializerError) -> Self {
        reinhardt_apps::Error::Internal(e.message)
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
        serde_json::to_string(input)
            .map_err(|e| SerializerError::new(format!("Serialization error: {}", e)))
    }

    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError> {
        serde_json::from_str(output)
            .map_err(|e| SerializerError::new(format!("Deserialization error: {}", e)))
    }
}

pub trait Deserializer {
    type Input;
    type Output;

    fn deserialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError>;
}
