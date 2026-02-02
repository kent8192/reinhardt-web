//! Scalar association proxies for one-to-one and many-to-one relationships

use serde::{Deserialize, Serialize};

use super::reflection::downcast_relationship;
use crate::proxy::ProxyResult;
use crate::proxy::ScalarValue;

/// Scalar proxy for accessing a single related object's attribute
///
/// Used for one-to-one and many-to-one relationships where the proxy
/// returns a single scalar value.
///
/// ## Example
///
/// ```rust,no_run
/// # use reinhardt_urls::proxy::ScalarProxy;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # #[derive(Clone)]
/// # struct User;
/// # let user = User;
/// // User has one profile, access profile.bio directly
/// let bio_proxy = ScalarProxy::new("profile", "bio");
/// // let bio: Option<String> = bio_proxy.get_value(&user).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ScalarProxy {
	/// Name of the relationship
	pub relationship: String,

	/// Name of the attribute on the related object
	pub attribute: String,
}

impl ScalarProxy {
	/// Create a new scalar proxy
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::ScalarProxy;
	///
	/// let proxy = ScalarProxy::new("profile", "bio");
	/// assert_eq!(proxy.relationship, "profile");
	/// assert_eq!(proxy.attribute, "bio");
	/// ```
	pub fn new(relationship: &str, attribute: &str) -> Self {
		Self {
			relationship: relationship.to_string(),
			attribute: attribute.to_string(),
		}
	}
	/// Get the scalar value from the related object
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::ScalarProxy;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = ScalarProxy::new("profile", "bio");
	// Assuming `user` implements Reflectable
	// let bio = proxy.get_value(&user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_value<T>(&self, source: &T) -> ProxyResult<Option<ScalarValue>>
	where
		T: super::reflection::Reflectable,
	{
		// 1. Access the relationship on source
		let relationship = match source.get_relationship(&self.relationship) {
			Some(rel) => rel,
			None => return Ok(None), // Relationship not found, return None
		};

		// 2. Downcast to Box<dyn Reflectable>
		let related =
			downcast_relationship::<Box<dyn super::reflection::Reflectable>>(relationship)?;

		// 3. Get the attribute from the related object
		let value = related.get_attribute(&self.attribute);

		Ok(value)
	}
	/// Set the scalar value on the related object
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarProxy, ScalarValue};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let proxy = ScalarProxy::new("profile", "bio");
	/// let value = ScalarValue::String("New bio".to_string());
	// Assuming `user` implements Reflectable
	// proxy.set_value(&mut user, value).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn set_value<T>(&self, source: &mut T, value: ScalarValue) -> ProxyResult<()>
	where
		T: super::reflection::Reflectable,
	{
		// Use the new set_relationship_attribute method to avoid type casting issues
		source.set_relationship_attribute(&self.relationship, &self.attribute, value)?;
		Ok(())
	}
}

/// Comparison operators for scalar proxies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalarComparison {
	/// Equal to
	Eq(ScalarValue),

	/// Not equal to
	Ne(ScalarValue),

	/// Greater than
	Gt(ScalarValue),

	/// Greater than or equal to
	Gte(ScalarValue),

	/// Less than
	Lt(ScalarValue),

	/// Less than or equal to
	Lte(ScalarValue),

	/// In collection
	In(Vec<ScalarValue>),

	/// Not in collection
	NotIn(Vec<ScalarValue>),

	/// Is null
	IsNull,

	/// Is not null
	IsNotNull,

	/// Like pattern (for strings)
	Like(String),

	/// Not like pattern (for strings)
	NotLike(String),
}

impl ScalarComparison {
	/// Create an equality comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let comparison = ScalarComparison::eq("test");
	/// assert!(matches!(comparison, ScalarComparison::Eq(_)));
	/// ```
	pub fn eq(value: impl Into<ScalarValue>) -> Self {
		ScalarComparison::Eq(value.into())
	}
	/// Create a not equal comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let comparison = ScalarComparison::ne(42);
	/// assert!(matches!(comparison, ScalarComparison::Ne(_)));
	/// ```
	pub fn ne(value: impl Into<ScalarValue>) -> Self {
		ScalarComparison::Ne(value.into())
	}
	/// Create a greater than comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let comparison = ScalarComparison::gt(100);
	/// assert!(matches!(comparison, ScalarComparison::Gt(ScalarValue::Int(100))));
	/// ```
	pub fn gt(value: impl Into<ScalarValue>) -> Self {
		ScalarComparison::Gt(value.into())
	}
	/// Create a greater than or equal comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let comparison = ScalarComparison::gte(50);
	/// assert!(matches!(comparison, ScalarComparison::Gte(ScalarValue::Int(50))));
	/// ```
	pub fn gte(value: impl Into<ScalarValue>) -> Self {
		ScalarComparison::Gte(value.into())
	}
	/// Create a less than comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let comparison = ScalarComparison::lt(25);
	/// assert!(matches!(comparison, ScalarComparison::Lt(ScalarValue::Int(25))));
	/// ```
	pub fn lt(value: impl Into<ScalarValue>) -> Self {
		ScalarComparison::Lt(value.into())
	}
	/// Create a less than or equal comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let comparison = ScalarComparison::lte(75);
	/// assert!(matches!(comparison, ScalarComparison::Lte(ScalarValue::Int(75))));
	/// ```
	pub fn lte(value: impl Into<ScalarValue>) -> Self {
		ScalarComparison::Lte(value.into())
	}
	/// Create an IN comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let values = vec![ScalarValue::Int(1), ScalarValue::Int(2)];
	/// let comparison = ScalarComparison::in_values(values);
	/// assert!(matches!(comparison, ScalarComparison::In(_)));
	/// ```
	pub fn in_values(values: Vec<ScalarValue>) -> Self {
		ScalarComparison::In(values)
	}
	/// Create a NOT IN comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{ScalarComparison, ScalarValue};
	///
	/// let values = vec![ScalarValue::String("banned".to_string())];
	/// let comparison = ScalarComparison::not_in_values(values);
	/// assert!(matches!(comparison, ScalarComparison::NotIn(_)));
	/// ```
	pub fn not_in_values(values: Vec<ScalarValue>) -> Self {
		ScalarComparison::NotIn(values)
	}
	/// Create an IS NULL comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::ScalarComparison;
	///
	/// let comparison = ScalarComparison::is_null();
	/// assert!(matches!(comparison, ScalarComparison::IsNull));
	/// ```
	pub fn is_null() -> Self {
		ScalarComparison::IsNull
	}
	/// Create an IS NOT NULL comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::ScalarComparison;
	///
	/// let comparison = ScalarComparison::is_not_null();
	/// assert!(matches!(comparison, ScalarComparison::IsNotNull));
	/// ```
	pub fn is_not_null() -> Self {
		ScalarComparison::IsNotNull
	}
	/// Create a LIKE comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::ScalarComparison;
	///
	/// let comparison = ScalarComparison::like("%test%");
	/// assert!(matches!(comparison, ScalarComparison::Like(_)));
	/// ```
	pub fn like(pattern: &str) -> Self {
		ScalarComparison::Like(pattern.to_string())
	}
	/// Create a NOT LIKE comparison
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::ScalarComparison;
	///
	/// let comparison = ScalarComparison::not_like("%spam%");
	/// assert!(matches!(comparison, ScalarComparison::NotLike(_)));
	/// ```
	pub fn not_like(pattern: &str) -> Self {
		ScalarComparison::NotLike(pattern.to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_scalar_proxy_creation() {
		let proxy = ScalarProxy::new("profile", "bio");
		assert_eq!(proxy.relationship, "profile");
		assert_eq!(proxy.attribute, "bio");
	}

	#[test]
	fn test_proxy_scalar_comparison_unit() {
		let eq = ScalarComparison::eq("test");
		assert!(matches!(eq, ScalarComparison::Eq(_)));

		let gt = ScalarComparison::gt(42);
		assert!(matches!(gt, ScalarComparison::Gt(_)));

		let is_null = ScalarComparison::is_null();
		assert!(matches!(is_null, ScalarComparison::IsNull));

		let like = ScalarComparison::like("%test%");
		assert!(matches!(like, ScalarComparison::Like(_)));
	}
}
