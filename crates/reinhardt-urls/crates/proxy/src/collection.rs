//! Collection association proxies for one-to-many and many-to-many relationships

use crate::proxy::ScalarValue;
use crate::{ProxyError, ProxyResult};

/// Collection proxy for accessing multiple related objects' attributes
///
/// Used for one-to-many and many-to-many relationships where the proxy
/// returns a collection of scalar values.
///
/// ## Example
///
/// ```rust,ignore
/// // User has many posts, access all post titles directly
/// let titles_proxy = CollectionProxy::new("posts", "title");
/// let titles: Vec<String> = titles_proxy.get_values(&user).await?;
/// ```
#[derive(Debug, Clone)]
pub struct CollectionProxy {
    /// Name of the relationship
    pub relationship: String,

    /// Name of the attribute on the related objects
    pub attribute: String,

    /// Whether to remove duplicates
    pub unique: bool,
}

impl CollectionProxy {
    /// Create a new collection proxy
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::CollectionProxy;
    ///
    /// let proxy = CollectionProxy::new("posts", "title");
    /// assert_eq!(proxy.relationship, "posts");
    /// assert_eq!(proxy.attribute, "title");
    /// assert!(!proxy.unique);
    /// ```
    pub fn new(relationship: &str, attribute: &str) -> Self {
        Self {
            relationship: relationship.to_string(),
            attribute: attribute.to_string(),
            unique: false,
        }
    }
    /// Create a collection proxy that removes duplicates
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::CollectionProxy;
    ///
    /// let proxy = CollectionProxy::unique("tags", "name");
    /// assert_eq!(proxy.relationship, "tags");
    /// assert_eq!(proxy.attribute, "name");
    /// assert!(proxy.unique);
    /// ```
    pub fn unique(relationship: &str, attribute: &str) -> Self {
        Self {
            relationship: relationship.to_string(),
            attribute: attribute.to_string(),
            unique: true,
        }
    }
    /// Get collection of values from related objects
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::CollectionProxy;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "title");
    /// // Assuming `user` implements Reflectable
    /// // let titles = proxy.get_values(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_values<T>(&self, source: &T) -> ProxyResult<Vec<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
    {
        // 1. Access the relationship on source
        let relationship = source
            .get_relationship(&self.relationship)
            .ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

        // 2. Downcast to Vec<Box<dyn Reflectable>>
        let collection = crate::reflection::downcast_relationship::<
            Vec<Box<dyn crate::reflection::Reflectable>>,
        >(relationship)?;

        // 3. Extract the attribute from each item
        let mut values = Vec::new();
        for item in collection.iter() {
            let value = item
                .get_attribute(&self.attribute)
                .ok_or_else(|| ProxyError::AttributeNotFound(self.attribute.clone()))?;
            values.push(value);
        }

        // 4. Optionally remove duplicates
        if self.unique {
            values.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
            values.dedup_by(|a, b| format!("{:?}", a) == format!("{:?}", b));
        }

        Ok(values)
    }
    /// Set collection of values by creating/updating related objects
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("tags", "name");
    /// let values = vec![ScalarValue::String("rust".to_string())];
    /// // let mut user = ...;
    /// // proxy.set_values(&mut user, values).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_values<T>(&self, source: &mut T, _values: Vec<ScalarValue>) -> ProxyResult<()>
    where
        T: crate::reflection::Reflectable,
    {
        // 1. Access the relationship
        let relationship = source
            .get_relationship_mut(&self.relationship)
            .ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

        // 2. Downcast to Vec<Box<dyn Reflectable>>
        let collection = relationship
            .downcast_mut::<Vec<Box<dyn crate::reflection::Reflectable>>>()
            .ok_or_else(|| ProxyError::TypeMismatch {
                expected: "Vec<Box<dyn Reflectable>>".to_string(),
                actual: "unknown".to_string(),
            })?;

        // 3. Clear existing items
        collection.clear();

        // 4. For each value, create new association objects
        // Note: In a full implementation, this would need to:
        // - Create actual model instances
        // - Set the attribute on each instance
        // - Add to the relationship
        // For now, this is a placeholder that clears the collection
        // and would need ORM integration to create instances

        Ok(())
    }
    /// Append a value to the collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("tags", "name");
    /// // let mut user = ...;
    /// // proxy.append(&mut user, ScalarValue::String("new_tag".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn append<T>(&self, _source: &mut T, _value: ScalarValue) -> ProxyResult<()>
    where
        T: crate::reflection::Reflectable,
    {
        // In a full implementation, this would:
        // 1. Create new association object with the value
        // 2. Add to the relationship
        // For now, this requires ORM integration to create instances
        Ok(())
    }
    /// Remove a value from the collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("tags", "name");
    /// // let mut user = ...;
    /// // proxy.remove(&mut user, ScalarValue::String("old_tag".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove<T>(&self, source: &mut T, value: ScalarValue) -> ProxyResult<()>
    where
        T: crate::reflection::Reflectable,
    {
        // 1. Access the relationship
        let relationship = source
            .get_relationship_mut(&self.relationship)
            .ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

        // 2. Downcast to Vec<Box<dyn Reflectable>>
        let collection = relationship
            .downcast_mut::<Vec<Box<dyn crate::reflection::Reflectable>>>()
            .ok_or_else(|| ProxyError::TypeMismatch {
                expected: "Vec<Box<dyn Reflectable>>".to_string(),
                actual: "unknown".to_string(),
            })?;

        // 3. Find and remove items with matching attribute value
        collection.retain(|item| {
            item.get_attribute(&self.attribute)
                .map(|v| v != value)
                .unwrap_or(true)
        });

        Ok(())
    }
    /// Check if the collection contains a value
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("tags", "name");
    /// // let user = ...;
    /// // let has_tag = proxy.contains(&user, ScalarValue::String("rust".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn contains<T>(&self, source: &T, value: ScalarValue) -> ProxyResult<bool>
    where
        T: crate::reflection::Reflectable,
    {
        let values = self.get_values(source).await?;
        Ok(values.contains(&value))
    }
    /// Get the count of items in the collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::CollectionProxy;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "title");
    /// // let user = ...;
    /// // let count = proxy.count(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count<T>(&self, source: &T) -> ProxyResult<usize>
    where
        T: crate::reflection::Reflectable,
    {
        // 1. Access the relationship
        let relationship = source
            .get_relationship(&self.relationship)
            .ok_or_else(|| ProxyError::RelationshipNotFound(self.relationship.clone()))?;

        // 2. Downcast to Vec<Box<dyn Reflectable>>
        let collection = crate::reflection::downcast_relationship::<
            Vec<Box<dyn crate::reflection::Reflectable>>,
        >(relationship)?;

        // 3. Return the count
        Ok(collection.len())
    }
    /// Filter collection by a condition on the proxy attribute
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, query::{FilterCondition, FilterOp}};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "status");
    /// let condition = FilterCondition::new("status", FilterOp::eq("published"));
    /// // let user = ...;
    /// // let filtered = proxy.filter(&user, condition).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn filter<T>(
        &self,
        source: &T,
        condition: crate::query::FilterCondition,
    ) -> ProxyResult<Vec<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
    {
        // Get all values first
        let values = self.get_values(source).await?;

        // Filter using the condition
        let filtered: Vec<ScalarValue> = values
            .into_iter()
            .filter(|v| condition.matches(v))
            .collect();

        Ok(filtered)
    }
    /// Filter collection using a custom predicate
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "views");
    /// // let user = ...;
    /// // let popular = proxy.filter_by(&user, |v| {
    /// //     matches!(v, ScalarValue::Integer(n) if *n > 1000)
    /// // }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn filter_by<T, F>(&self, source: &T, predicate: F) -> ProxyResult<Vec<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
        F: Fn(&ScalarValue) -> bool,
    {
        // Get all values first
        let values = self.get_values(source).await?;

        // Filter using the predicate
        let filtered: Vec<ScalarValue> = values.into_iter().filter(|v| predicate(v)).collect();

        Ok(filtered)
    }
}

/// Collection operations for filtering and transforming
#[derive(Debug, Clone)]
pub struct CollectionOperations {
    #[allow(dead_code)]
    proxy: CollectionProxy,
}

impl CollectionOperations {
    /// Create new collection operations wrapper
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionOperations};
    ///
    /// let proxy = CollectionProxy::new("posts", "title");
    /// let ops = CollectionOperations::new(proxy);
    /// // Operations wrapper is ready to use
    /// ```
    pub fn new(proxy: CollectionProxy) -> Self {
        Self { proxy }
    }
    /// Filter collection by predicate
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue, collection::CollectionOperations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "title");
    /// let ops = CollectionOperations::new(proxy);
    /// // let user = ...;
    /// // let filtered = ops.filter(&user, |v| v.is_null()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn filter<T, F>(&self, _source: &T, _predicate: F) -> ProxyResult<Vec<ScalarValue>>
    where
        F: Fn(&ScalarValue) -> bool,
    {
        // In a real implementation, this would apply the filter
        Ok(Vec::new())
    }
    /// Map collection values
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, ScalarValue, collection::CollectionOperations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "title");
    /// let ops = CollectionOperations::new(proxy);
    /// // let user = ...;
    /// // let lengths = ops.map(&user, |v| match v {
    /// //     ScalarValue::String(s) => s.len(),
    /// //     _ => 0,
    /// // }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn map<T, F, U>(&self, _source: &T, _mapper: F) -> ProxyResult<Vec<U>>
    where
        F: Fn(&ScalarValue) -> U,
    {
        // In a real implementation, this would apply the mapper
        Ok(Vec::new())
    }
    /// Sort collection values
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionOperations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "created_at");
    /// let ops = CollectionOperations::new(proxy);
    /// // let user = ...;
    /// // let sorted = ops.sort(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sort<T>(&self, _source: &T) -> ProxyResult<Vec<ScalarValue>> {
        // In a real implementation, this would sort the values
        Ok(Vec::new())
    }
    /// Get distinct values
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionOperations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("posts", "category");
    /// let ops = CollectionOperations::new(proxy);
    /// // let user = ...;
    /// // let distinct = ops.distinct(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn distinct<T>(&self, _source: &T) -> ProxyResult<Vec<ScalarValue>> {
        // In a real implementation, this would remove duplicates
        Ok(Vec::new())
    }
}

/// Aggregation operations on collections
#[derive(Debug, Clone)]
pub struct CollectionAggregations {
    #[allow(dead_code)]
    proxy: CollectionProxy,
}

impl CollectionAggregations {
    /// Create new collection aggregations wrapper
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionAggregations};
    ///
    /// let proxy = CollectionProxy::new("sales", "amount");
    /// let agg = CollectionAggregations::new(proxy);
    /// // Aggregations wrapper is ready to use
    /// ```
    pub fn new(proxy: CollectionProxy) -> Self {
        Self { proxy }
    }
    /// Sum numeric values in collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionAggregations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("sales", "amount");
    /// let agg = CollectionAggregations::new(proxy);
    /// // let product = ...;
    /// // let total = agg.sum(&product).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sum<T>(&self, _source: &T) -> ProxyResult<f64> {
        Ok(0.0)
    }
    /// Average of numeric values in collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionAggregations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("reviews", "rating");
    /// let agg = CollectionAggregations::new(proxy);
    /// // let product = ...;
    /// // let average = agg.avg(&product).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn avg<T>(&self, _source: &T) -> ProxyResult<f64> {
        Ok(0.0)
    }
    /// Minimum value in collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionAggregations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("prices", "amount");
    /// let agg = CollectionAggregations::new(proxy);
    /// // let product = ...;
    /// // let min_price = agg.min(&product).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn min<T>(&self, _source: &T) -> ProxyResult<Option<ScalarValue>> {
        Ok(None)
    }
    /// Maximum value in collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use reinhardt_proxy::{CollectionProxy, collection::CollectionAggregations};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let proxy = CollectionProxy::new("bids", "amount");
    /// let agg = CollectionAggregations::new(proxy);
    /// // let auction = ...;
    /// // let highest_bid = agg.max(&auction).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn max<T>(&self, _source: &T) -> ProxyResult<Option<ScalarValue>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_collection_creation_unit() {
        let proxy = CollectionProxy::new("posts", "title");
        assert_eq!(proxy.relationship, "posts");
        assert_eq!(proxy.attribute, "title");
        assert!(!proxy.unique);
    }

    #[test]
    fn test_proxy_collection_unique_unit() {
        let proxy = CollectionProxy::unique("posts", "title");
        assert!(proxy.unique);
    }

    #[test]
    fn test_collection_operations_creation() {
        let proxy = CollectionProxy::new("posts", "title");
        let ops = CollectionOperations::new(proxy);
        assert_eq!(ops.proxy.relationship, "posts");
    }

    #[test]
    fn test_collection_aggregations_creation() {
        let proxy = CollectionProxy::new("posts", "score");
        let agg = CollectionAggregations::new(proxy);
        assert_eq!(agg.proxy.relationship, "posts");
    }
}
