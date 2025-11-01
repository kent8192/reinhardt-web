//! Type traits for field lookups
//!
//! These traits are used to constrain which methods are available
//! for different field types.

use super::lookup::LookupValue;
use serde::{Deserialize, Serialize};

/// Marker trait for types that can be compared (=, !=, <, >, <=, >=)
pub trait Comparable:
	Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + Into<LookupValue>
{
}

/// Marker trait for string types (String, &str)
pub trait StringType: Comparable {}

/// Marker trait for numeric types (i32, i64, f32, f64)
pub trait NumericType: Comparable {}

/// Marker trait for date/time types
pub trait DateTimeType: Comparable {}

// Implement Comparable for common types
impl Comparable for String {}
impl Comparable for i32 {}
impl Comparable for i64 {}
impl Comparable for f32 {}
impl Comparable for f64 {}
impl Comparable for bool {}

// Implement StringType
impl StringType for String {}

// Implement NumericType
impl NumericType for i32 {}
impl NumericType for i64 {}
impl NumericType for f32 {}
impl NumericType for f64 {}

// DateTime type (simplified for now)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
	pub timestamp: i64,
}

impl Comparable for DateTime {}
impl DateTimeType for DateTime {}

// Date type (simplified for now)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
	pub year: i32,
	pub month: u8,
	pub day: u8,
}

impl Comparable for Date {}
