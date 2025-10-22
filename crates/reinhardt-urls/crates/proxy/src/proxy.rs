//! Core association proxy implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::{ProxyError, ProxyResult};

/// Association proxy for transparent access to related object attributes
///
/// ## Example
///
/// ```rust,ignore
// Access keyword names through user_keywords relationship
/// let proxy = AssociationProxy::new("user_keywords", "keyword");
/// let names = proxy.get_collection(&user).await?;
/// ```
pub struct AssociationProxy<T, U> {
    /// Name of the relationship attribute
    pub relationship: String,

    /// Name of the attribute on the related object
    pub attribute: String,

    /// Optional creator function for new associations
    pub creator: Option<fn(U) -> T>,

    /// Phantom data for type parameters
    _phantom: PhantomData<(T, U)>,
}

impl<T, U> AssociationProxy<T, U> {
    /// Create a new association proxy
    ///
    /// # Arguments
    ///
    /// * `relationship` - Name of the relationship to traverse
    /// * `attribute` - Name of the attribute to access on related objects
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::AssociationProxy;
    ///
    /// let proxy: AssociationProxy<(), ()> = AssociationProxy::new("user_keywords", "keyword");
    /// assert_eq!(proxy.relationship, "user_keywords");
    /// assert_eq!(proxy.attribute, "keyword");
    /// ```
    pub fn new(relationship: &str, attribute: &str) -> Self {
        Self {
            relationship: relationship.to_string(),
            attribute: attribute.to_string(),
            creator: None,
            _phantom: PhantomData,
        }
    }
    /// Set a creator function for new associations
    ///
    /// The creator function is called when adding new items to the association.
    /// It should create an association object from the target value.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::AssociationProxy;
    ///
    /// fn create_association(value: String) -> i32 { 42 }
    ///
    /// let proxy = AssociationProxy::new("items", "value")
    ///     .with_creator(create_association);
    /// assert!(proxy.creator.is_some());
    /// ```
    pub fn with_creator(mut self, creator: fn(U) -> T) -> Self {
        self.creator = Some(creator);
        self
    }
}

/// Trait for accessing proxy targets
#[async_trait]
pub trait ProxyAccessor<T> {
    /// Get the target value(s) from the source object
    async fn get(&self, source: &T) -> ProxyResult<ProxyTarget>;

    /// Set the target value(s) on the source object
    async fn set(&self, source: &mut T, value: ProxyTarget) -> ProxyResult<()>;
}

/// Represents the target of a proxy operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProxyTarget {
    /// Single scalar value
    Scalar(ScalarValue),

    /// Collection of values
    Collection(Vec<ScalarValue>),

    /// No value (None)
    None,
}

/// Scalar value types supported by proxies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ScalarValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

impl From<String> for ScalarValue {
    fn from(s: String) -> Self {
        ScalarValue::String(s)
    }
}

impl From<&str> for ScalarValue {
    fn from(s: &str) -> Self {
        ScalarValue::String(s.to_string())
    }
}

impl From<i64> for ScalarValue {
    fn from(i: i64) -> Self {
        ScalarValue::Integer(i)
    }
}

impl From<f64> for ScalarValue {
    fn from(f: f64) -> Self {
        ScalarValue::Float(f)
    }
}

impl From<bool> for ScalarValue {
    fn from(b: bool) -> Self {
        ScalarValue::Boolean(b)
    }
}

impl ScalarValue {
    /// Try to convert to String
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ScalarValue;
    ///
    /// let value = ScalarValue::String("hello".to_string());
    /// assert_eq!(value.as_string().unwrap(), "hello");
    ///
    /// let int_value = ScalarValue::Integer(42);
    /// assert!(int_value.as_string().is_err());
    /// ```
    pub fn as_string(&self) -> ProxyResult<String> {
        match self {
            ScalarValue::String(s) => Ok(s.clone()),
            _ => Err(ProxyError::TypeMismatch {
                expected: "String".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }
    /// Try to convert to i64
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ScalarValue;
    ///
    /// let value = ScalarValue::Integer(42);
    /// assert_eq!(value.as_integer().unwrap(), 42);
    ///
    /// let str_value = ScalarValue::String("test".to_string());
    /// assert!(str_value.as_integer().is_err());
    /// ```
    pub fn as_integer(&self) -> ProxyResult<i64> {
        match self {
            ScalarValue::Integer(i) => Ok(*i),
            _ => Err(ProxyError::TypeMismatch {
                expected: "Integer".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }
    /// Try to convert to f64
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ScalarValue;
    ///
    /// let value = ScalarValue::Float(3.14);
    /// assert_eq!(value.as_float().unwrap(), 3.14);
    ///
    /// let bool_value = ScalarValue::Boolean(true);
    /// assert!(bool_value.as_float().is_err());
    /// ```
    pub fn as_float(&self) -> ProxyResult<f64> {
        match self {
            ScalarValue::Float(f) => Ok(*f),
            _ => Err(ProxyError::TypeMismatch {
                expected: "Float".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }
    /// Try to convert to bool
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ScalarValue;
    ///
    /// let value = ScalarValue::Boolean(true);
    /// assert_eq!(value.as_boolean().unwrap(), true);
    ///
    /// let int_value = ScalarValue::Integer(1);
    /// assert!(int_value.as_boolean().is_err());
    /// ```
    pub fn as_boolean(&self) -> ProxyResult<bool> {
        match self {
            ScalarValue::Boolean(b) => Ok(*b),
            _ => Err(ProxyError::TypeMismatch {
                expected: "Boolean".to_string(),
                actual: format!("{:?}", self),
            }),
        }
    }
    /// Check if value is null
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ScalarValue;
    ///
    /// let null_value = ScalarValue::Null;
    /// assert!(null_value.is_null());
    ///
    /// let string_value = ScalarValue::String("test".to_string());
    /// assert!(!string_value.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self, ScalarValue::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_scalar_conversions_unit() {
        let s = ScalarValue::String("test".to_string());
        assert_eq!(s.as_string().unwrap(), "test");

        let i = ScalarValue::Integer(42);
        assert_eq!(i.as_integer().unwrap(), 42);

        let f = ScalarValue::Float(3.14);
        assert_eq!(f.as_float().unwrap(), 3.14);

        let b = ScalarValue::Boolean(true);
        assert_eq!(b.as_boolean().unwrap(), true);
    }

    #[test]
    fn test_proxy_scalar_type_mismatch_unit() {
        let s = ScalarValue::String("test".to_string());
        assert!(s.as_integer().is_err());
    }
}
