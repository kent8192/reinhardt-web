//! Lookup type and value definitions

use crate::Model;
use serde::{Deserialize, Serialize};

/// Lookup type - defines how to compare the field value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LookupType {
    // Equality
    Exact,  // =
    IExact, // ILIKE (case-insensitive)
    Ne,     // !=

    // Pattern matching
    Contains,    // LIKE '%x%'
    IContains,   // ILIKE '%x%'
    StartsWith,  // LIKE 'x%'
    IStartsWith, // ILIKE 'x%'
    EndsWith,    // LIKE '%x'
    IEndsWith,   // ILIKE '%x'
    Regex,       // ~ (PostgreSQL)
    IRegex,      // ~* (PostgreSQL)

    // Comparison
    Gt,    // >
    Gte,   // >=
    Lt,    // <
    Lte,   // <=
    Range, // BETWEEN

    // Set operations
    In,    // IN
    NotIn, // NOT IN

    // NULL checks
    IsNull,    // IS NULL
    IsNotNull, // IS NOT NULL
}

/// Lookup value - the value to compare against
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LookupValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<LookupValue>),
    Range(Box<LookupValue>, Box<LookupValue>),
    Null,
}

impl From<String> for LookupValue {
    fn from(s: String) -> Self {
        LookupValue::String(s)
    }
}

impl From<&str> for LookupValue {
    fn from(s: &str) -> Self {
        LookupValue::String(s.to_string())
    }
}

impl From<i32> for LookupValue {
    fn from(i: i32) -> Self {
        LookupValue::Int(i as i64)
    }
}

impl From<i64> for LookupValue {
    fn from(i: i64) -> Self {
        LookupValue::Int(i)
    }
}

impl From<f32> for LookupValue {
    fn from(f: f32) -> Self {
        LookupValue::Float(f as f64)
    }
}

impl From<f64> for LookupValue {
    fn from(f: f64) -> Self {
        LookupValue::Float(f)
    }
}

impl From<bool> for LookupValue {
    fn from(b: bool) -> Self {
        LookupValue::Bool(b)
    }
}

impl From<()> for LookupValue {
    fn from(_: ()) -> Self {
        LookupValue::Null
    }
}

// DateTime and Date conversions
impl From<crate::query_fields::traits::DateTime> for LookupValue {
    fn from(dt: crate::query_fields::traits::DateTime) -> Self {
        LookupValue::Int(dt.timestamp)
    }
}

impl From<crate::query_fields::traits::Date> for LookupValue {
    fn from(date: crate::query_fields::traits::Date) -> Self {
        // Encode as days since epoch or similar
        let days = date.year * 10000 + (date.month as i32) * 100 + (date.day as i32);
        LookupValue::Int(days as i64)
    }
}

impl<T: Into<LookupValue>> From<(T, T)> for LookupValue {
    fn from((start, end): (T, T)) -> Self {
        LookupValue::Range(Box::new(start.into()), Box::new(end.into()))
    }
}

/// A complete lookup specification ready to be compiled to SQL
#[derive(Debug, Clone)]
pub struct Lookup<M: Model> {
    pub(crate) field_path: Vec<&'static str>,
    pub(crate) lookup_type: LookupType,
    pub(crate) value: LookupValue,
    pub(crate) _phantom: std::marker::PhantomData<M>,
}

impl<M: Model> Lookup<M> {
    /// Create a new lookup for field filtering in QuerySets
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use reinhardt_orm::query_fields::{Lookup, LookupType, LookupValue};
    ///
    /// let lookup = Lookup::new(
    ///     vec!["name"],
    ///     LookupType::Exact,
    ///     LookupValue::String("Alice".to_string())
    /// );
    // Represents: WHERE name = 'Alice'
    /// ```
    pub fn new(field_path: Vec<&'static str>, lookup_type: LookupType, value: LookupValue) -> Self {
        Self {
            field_path,
            lookup_type,
            value,
            _phantom: std::marker::PhantomData,
        }
    }
    /// Get the field path
    ///
    pub fn field_path(&self) -> &[&'static str] {
        &self.field_path
    }
    /// Get the lookup type
    ///
    pub fn lookup_type(&self) -> &LookupType {
        &self.lookup_type
    }
    /// Get the lookup value
    ///
    pub fn value(&self) -> &LookupValue {
        &self.value
    }
}
