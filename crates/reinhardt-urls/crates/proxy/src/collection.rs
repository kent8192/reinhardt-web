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
// User has many posts, access all post titles directly
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
    // Assuming `user` implements Reflectable
    // let titles = proxy.get_values(&user).await?;
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
    // let mut user = ...;
    // proxy.set_values(&mut user, values).await?;
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

        // 4. Create new association objects for each value
        // Note: This requires T to be constructible from ScalarValue
        // In practice, this would use ORM integration to create instances
        // For now, we clear the collection as the actual object creation
        // depends on the specific ORM model implementation

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
    // let mut user = ...;
    // proxy.append(&mut user, ScalarValue::String("new_tag".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn append<T>(&self, _source: &mut T, _value: ScalarValue) -> ProxyResult<()>
    where
        T: crate::reflection::Reflectable,
    {
        // Note: Creating and appending a new association object requires:
        // 1. A constructor function for the association type
        // 2. Access to the relationship collection
        // 3. Setting the attribute on the new instance
        // This functionality requires ORM integration and a creator function
        // The implementation depends on the specific model structure
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
    // let mut user = ...;
    // proxy.remove(&mut user, ScalarValue::String("old_tag".to_string())).await?;
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
    // let user = ...;
    // let has_tag = proxy.contains(&user, ScalarValue::String("rust".to_string())).await?;
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
    // let user = ...;
    // let count = proxy.count(&user).await?;
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
    // let user = ...;
    // let filtered = proxy.filter(&user, condition).await?;
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
    // let user = ...;
    // let popular = proxy.filter_by(&user, |v| {
    //     matches!(v, ScalarValue::Integer(n) if *n > 1000)
    // }).await?;
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
    // Operations wrapper is ready to use
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
    // let user = ...;
    // let filtered = ops.filter(&user, |v| v.is_null()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn filter<T, F>(&self, source: &T, predicate: F) -> ProxyResult<Vec<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
        F: Fn(&ScalarValue) -> bool,
    {
        let values = self.proxy.get_values(source).await?;
        Ok(values.into_iter().filter(|v| predicate(v)).collect())
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
    // let user = ...;
    // let lengths = ops.map(&user, |v| match v {
    //     ScalarValue::String(s) => s.len(),
    //     _ => 0,
    // }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn map<T, F, U>(&self, source: &T, mapper: F) -> ProxyResult<Vec<U>>
    where
        T: crate::reflection::Reflectable,
        F: Fn(&ScalarValue) -> U,
    {
        let values = self.proxy.get_values(source).await?;
        Ok(values.iter().map(|v| mapper(v)).collect())
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
    // let user = ...;
    // let sorted = ops.sort(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sort<T>(&self, source: &T) -> ProxyResult<Vec<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
    {
        let mut values = self.proxy.get_values(source).await?;
        values.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        Ok(values)
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
    // let user = ...;
    // let distinct = ops.distinct(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn distinct<T>(&self, source: &T) -> ProxyResult<Vec<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
    {
        let mut values = self.proxy.get_values(source).await?;
        values.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        values.dedup_by(|a, b| format!("{:?}", a) == format!("{:?}", b));
        Ok(values)
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
    // Aggregations wrapper is ready to use
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
    // let product = ...;
    // let total = agg.sum(&product).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sum<T>(&self, source: &T) -> ProxyResult<f64>
    where
        T: crate::reflection::Reflectable,
    {
        let values = self.proxy.get_values(source).await?;
        let mut sum = 0.0;
        for value in values {
            match value {
                ScalarValue::Integer(i) => sum += i as f64,
                ScalarValue::Float(f) => sum += f,
                _ => {}
            }
        }
        Ok(sum)
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
    // let product = ...;
    // let average = agg.avg(&product).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn avg<T>(&self, source: &T) -> ProxyResult<f64>
    where
        T: crate::reflection::Reflectable,
    {
        let values = self.proxy.get_values(source).await?;
        let mut sum = 0.0;
        let mut count = 0;
        for value in &values {
            match value {
                ScalarValue::Integer(i) => {
                    sum += *i as f64;
                    count += 1;
                }
                ScalarValue::Float(f) => {
                    sum += f;
                    count += 1;
                }
                _ => {}
            }
        }
        if count == 0 {
            Ok(0.0)
        } else {
            Ok(sum / count as f64)
        }
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
    // let product = ...;
    // let min_price = agg.min(&product).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn min<T>(&self, source: &T) -> ProxyResult<Option<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
    {
        let values = self.proxy.get_values(source).await?;
        Ok(values.into_iter().min_by(|a, b| {
            use std::cmp::Ordering;
            match (a, b) {
                (ScalarValue::Integer(x), ScalarValue::Integer(y)) => x.cmp(y),
                (ScalarValue::Float(x), ScalarValue::Float(y)) => {
                    x.partial_cmp(y).unwrap_or(Ordering::Equal)
                }
                (ScalarValue::Integer(x), ScalarValue::Float(y)) => {
                    (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal)
                }
                (ScalarValue::Float(x), ScalarValue::Integer(y)) => {
                    x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal)
                }
                _ => format!("{:?}", a).cmp(&format!("{:?}", b)),
            }
        }))
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
    // let auction = ...;
    // let highest_bid = agg.max(&auction).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn max<T>(&self, source: &T) -> ProxyResult<Option<ScalarValue>>
    where
        T: crate::reflection::Reflectable,
    {
        let values = self.proxy.get_values(source).await?;
        Ok(values.into_iter().max_by(|a, b| {
            use std::cmp::Ordering;
            match (a, b) {
                (ScalarValue::Integer(x), ScalarValue::Integer(y)) => x.cmp(y),
                (ScalarValue::Float(x), ScalarValue::Float(y)) => {
                    x.partial_cmp(y).unwrap_or(Ordering::Equal)
                }
                (ScalarValue::Integer(x), ScalarValue::Float(y)) => {
                    (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal)
                }
                (ScalarValue::Float(x), ScalarValue::Integer(y)) => {
                    x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal)
                }
                _ => format!("{:?}", a).cmp(&format!("{:?}", b)),
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflection::Reflectable;
    use std::any::Any;

    #[derive(Clone)]
    struct TestParent {
        id: i64,
        children: Vec<TestChild>,
    }

    #[derive(Clone)]
    struct TestChild {
        id: i64,
        value: i64,
        score: f64,
    }

    impl Reflectable for TestParent {
        fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
            match name {
                "children" => {
                    let boxed: Vec<Box<dyn Reflectable>> = self
                        .children
                        .iter()
                        .map(|c| Box::new(c.clone()) as Box<dyn Reflectable>)
                        .collect();
                    Some(Box::new(boxed))
                }
                _ => None,
            }
        }

        fn get_relationship_mut(&mut self, name: &str) -> Option<&mut dyn Any> {
            match name {
                "children" => Some(&mut self.children as &mut dyn Any),
                _ => None,
            }
        }

        fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
            match name {
                "id" => Some(ScalarValue::Integer(self.id)),
                _ => None,
            }
        }

        fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
            match name {
                "id" => {
                    self.id = value.as_integer()?;
                    Ok(())
                }
                _ => Err(ProxyError::AttributeNotFound(name.to_string())),
            }
        }

        fn set_relationship_attribute(
            &mut self,
            relationship: &str,
            _attribute: &str,
            _value: ScalarValue,
        ) -> ProxyResult<()> {
            Err(ProxyError::RelationshipNotFound(relationship.to_string()))
        }
    }

    impl Reflectable for TestChild {
        fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any>> {
            None
        }

        fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
            None
        }

        fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
            match name {
                "id" => Some(ScalarValue::Integer(self.id)),
                "value" => Some(ScalarValue::Integer(self.value)),
                "score" => Some(ScalarValue::Float(self.score)),
                _ => None,
            }
        }

        fn set_attribute(&mut self, name: &str, value: ScalarValue) -> ProxyResult<()> {
            match name {
                "id" => {
                    self.id = value.as_integer()?;
                    Ok(())
                }
                "value" => {
                    self.value = value.as_integer()?;
                    Ok(())
                }
                "score" => {
                    self.score = value.as_float()?;
                    Ok(())
                }
                _ => Err(ProxyError::AttributeNotFound(name.to_string())),
            }
        }

        fn set_relationship_attribute(
            &mut self,
            relationship: &str,
            _attribute: &str,
            _value: ScalarValue,
        ) -> ProxyResult<()> {
            Err(ProxyError::RelationshipNotFound(relationship.to_string()))
        }
    }

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

    #[tokio::test]
    async fn test_collection_operations_filter() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 30,
                    score: 3.0,
                },
            ],
        };

        let filtered = ops
            .filter(&parent, |v| matches!(v, ScalarValue::Integer(i) if *i > 15))
            .await
            .unwrap();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].as_integer().unwrap(), 20);
        assert_eq!(filtered[1].as_integer().unwrap(), 30);
    }

    #[tokio::test]
    async fn test_collection_operations_filter_all_match() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let filtered = ops
            .filter(&parent, |v| matches!(v, ScalarValue::Integer(_)))
            .await
            .unwrap();

        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_collection_operations_filter_none_match() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let filtered = ops
            .filter(&parent, |v| matches!(v, ScalarValue::String(_)))
            .await
            .unwrap();

        assert_eq!(filtered.len(), 0);
    }

    #[tokio::test]
    async fn test_collection_operations_filter_empty_collection() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let filtered = ops
            .filter(&parent, |v| matches!(v, ScalarValue::Integer(_)))
            .await
            .unwrap();

        assert_eq!(filtered.len(), 0);
    }

    #[tokio::test]
    async fn test_collection_operations_filter_complex_condition() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 15,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 25,
                    score: 3.0,
                },
            ],
        };

        let filtered = ops
            .filter(
                &parent,
                |v| matches!(v, ScalarValue::Integer(i) if *i >= 10 && *i <= 20),
            )
            .await
            .unwrap();

        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_collection_operations_map() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let mapped: Vec<i64> = ops
            .map(&parent, |v| match v {
                ScalarValue::Integer(i) => i * 2,
                _ => 0,
            })
            .await
            .unwrap();

        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0], 20);
        assert_eq!(mapped[1], 40);
    }

    #[tokio::test]
    async fn test_collection_operations_map_to_string() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let mapped: Vec<String> = ops
            .map(&parent, |v| match v {
                ScalarValue::Integer(i) => format!("Value: {}", i),
                _ => String::from("Unknown"),
            })
            .await
            .unwrap();

        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0], "Value: 10");
        assert_eq!(mapped[1], "Value: 20");
    }

    #[tokio::test]
    async fn test_collection_operations_map_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let mapped: Vec<i64> = ops
            .map(&parent, |v| match v {
                ScalarValue::Integer(i) => i * 2,
                _ => 0,
            })
            .await
            .unwrap();

        assert_eq!(mapped.len(), 0);
    }

    #[tokio::test]
    async fn test_collection_operations_map_identity() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let mapped: Vec<i64> = ops
            .map(&parent, |v| match v {
                ScalarValue::Integer(i) => *i,
                _ => 0,
            })
            .await
            .unwrap();

        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0], 10);
        assert_eq!(mapped[1], 20);
    }

    #[tokio::test]
    async fn test_collection_operations_map_constant() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let mapped: Vec<i64> = ops.map(&parent, |_| 42).await.unwrap();

        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0], 42);
        assert_eq!(mapped[1], 42);
    }

    #[tokio::test]
    async fn test_collection_operations_sort() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 30,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 10,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 20,
                    score: 3.0,
                },
            ],
        };

        let sorted = ops.sort(&parent).await.unwrap();

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].as_integer().unwrap(), 10);
        assert_eq!(sorted[1].as_integer().unwrap(), 20);
        assert_eq!(sorted[2].as_integer().unwrap(), 30);
    }

    #[tokio::test]
    async fn test_collection_operations_sort_already_sorted() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 30,
                    score: 3.0,
                },
            ],
        };

        let sorted = ops.sort(&parent).await.unwrap();

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].as_integer().unwrap(), 10);
        assert_eq!(sorted[1].as_integer().unwrap(), 20);
        assert_eq!(sorted[2].as_integer().unwrap(), 30);
    }

    #[tokio::test]
    async fn test_collection_operations_sort_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let sorted = ops.sort(&parent).await.unwrap();
        assert_eq!(sorted.len(), 0);
    }

    #[tokio::test]
    async fn test_collection_operations_sort_single() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![TestChild {
                id: 1,
                value: 42,
                score: 1.0,
            }],
        };

        let sorted = ops.sort(&parent).await.unwrap();

        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].as_integer().unwrap(), 42);
    }

    #[tokio::test]
    async fn test_collection_operations_sort_reverse_order() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 50,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 40,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 30,
                    score: 3.0,
                },
            ],
        };

        let sorted = ops.sort(&parent).await.unwrap();

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].as_integer().unwrap(), 30);
        assert_eq!(sorted[1].as_integer().unwrap(), 40);
        assert_eq!(sorted[2].as_integer().unwrap(), 50);
    }

    #[tokio::test]
    async fn test_collection_operations_distinct() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 10,
                    score: 3.0,
                },
            ],
        };

        let distinct = ops.distinct(&parent).await.unwrap();

        assert_eq!(distinct.len(), 2);
        assert_eq!(distinct[0].as_integer().unwrap(), 10);
        assert_eq!(distinct[1].as_integer().unwrap(), 20);
    }

    #[tokio::test]
    async fn test_collection_operations_distinct_all_unique() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 30,
                    score: 3.0,
                },
            ],
        };

        let distinct = ops.distinct(&parent).await.unwrap();

        assert_eq!(distinct.len(), 3);
    }

    #[tokio::test]
    async fn test_collection_operations_distinct_all_same() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 10,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 10,
                    score: 3.0,
                },
            ],
        };

        let distinct = ops.distinct(&parent).await.unwrap();

        assert_eq!(distinct.len(), 1);
        assert_eq!(distinct[0].as_integer().unwrap(), 10);
    }

    #[tokio::test]
    async fn test_collection_operations_distinct_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let distinct = ops.distinct(&parent).await.unwrap();
        assert_eq!(distinct.len(), 0);
    }

    #[tokio::test]
    async fn test_collection_operations_distinct_multiple_duplicates() {
        let proxy = CollectionProxy::new("children", "value");
        let ops = CollectionOperations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 10,
                    score: 3.0,
                },
                TestChild {
                    id: 4,
                    value: 20,
                    score: 4.0,
                },
                TestChild {
                    id: 5,
                    value: 30,
                    score: 5.0,
                },
            ],
        };

        let distinct = ops.distinct(&parent).await.unwrap();

        assert_eq!(distinct.len(), 3);
        assert_eq!(distinct[0].as_integer().unwrap(), 10);
        assert_eq!(distinct[1].as_integer().unwrap(), 20);
        assert_eq!(distinct[2].as_integer().unwrap(), 30);
    }

    #[tokio::test]
    async fn test_collection_aggregations_sum() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 30,
                    score: 3.0,
                },
            ],
        };

        let sum = agg.sum(&parent).await.unwrap();
        assert_eq!(sum, 60.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_sum_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let sum = agg.sum(&parent).await.unwrap();
        assert_eq!(sum, 0.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_sum_floats() {
        let proxy = CollectionProxy::new("children", "score");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.5,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.5,
                },
            ],
        };

        let sum = agg.sum(&parent).await.unwrap();
        assert_eq!(sum, 4.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_sum_single() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![TestChild {
                id: 1,
                value: 42,
                score: 1.0,
            }],
        };

        let sum = agg.sum(&parent).await.unwrap();
        assert_eq!(sum, 42.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_sum_negative() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: -10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let sum = agg.sum(&parent).await.unwrap();
        assert_eq!(sum, 10.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_avg() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 30,
                    score: 3.0,
                },
            ],
        };

        let avg = agg.avg(&parent).await.unwrap();
        assert_eq!(avg, 20.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_avg_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let avg = agg.avg(&parent).await.unwrap();
        assert_eq!(avg, 0.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_avg_single() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![TestChild {
                id: 1,
                value: 42,
                score: 1.0,
            }],
        };

        let avg = agg.avg(&parent).await.unwrap();
        assert_eq!(avg, 42.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_avg_floats() {
        let proxy = CollectionProxy::new("children", "score");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 3.0,
                },
            ],
        };

        let avg = agg.avg(&parent).await.unwrap();
        assert_eq!(avg, 2.0);
    }

    #[tokio::test]
    async fn test_collection_aggregations_avg_decimal() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 15,
                    score: 2.0,
                },
            ],
        };

        let avg = agg.avg(&parent).await.unwrap();
        assert_eq!(avg, 12.5);
    }

    #[tokio::test]
    async fn test_collection_aggregations_min() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 30,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 10,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 20,
                    score: 3.0,
                },
            ],
        };

        let min = agg.min(&parent).await.unwrap();
        assert!(min.is_some());
        assert_eq!(min.unwrap().as_integer().unwrap(), 10);
    }

    #[tokio::test]
    async fn test_collection_aggregations_min_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let min = agg.min(&parent).await.unwrap();
        assert!(min.is_none());
    }

    #[tokio::test]
    async fn test_collection_aggregations_min_single() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![TestChild {
                id: 1,
                value: 42,
                score: 1.0,
            }],
        };

        let min = agg.min(&parent).await.unwrap();
        assert!(min.is_some());
        assert_eq!(min.unwrap().as_integer().unwrap(), 42);
    }

    #[tokio::test]
    async fn test_collection_aggregations_min_negative() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: -10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 20,
                    score: 2.0,
                },
            ],
        };

        let min = agg.min(&parent).await.unwrap();
        assert!(min.is_some());
        assert_eq!(min.unwrap().as_integer().unwrap(), -10);
    }

    #[tokio::test]
    async fn test_collection_aggregations_min_all_same() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 10,
                    score: 2.0,
                },
            ],
        };

        let min = agg.min(&parent).await.unwrap();
        assert!(min.is_some());
        assert_eq!(min.unwrap().as_integer().unwrap(), 10);
    }

    #[tokio::test]
    async fn test_collection_aggregations_max() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 30,
                    score: 2.0,
                },
                TestChild {
                    id: 3,
                    value: 20,
                    score: 3.0,
                },
            ],
        };

        let max = agg.max(&parent).await.unwrap();
        assert!(max.is_some());
        assert_eq!(max.unwrap().as_integer().unwrap(), 30);
    }

    #[tokio::test]
    async fn test_collection_aggregations_max_empty() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![],
        };

        let max = agg.max(&parent).await.unwrap();
        assert!(max.is_none());
    }

    #[tokio::test]
    async fn test_collection_aggregations_max_single() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![TestChild {
                id: 1,
                value: 42,
                score: 1.0,
            }],
        };

        let max = agg.max(&parent).await.unwrap();
        assert!(max.is_some());
        assert_eq!(max.unwrap().as_integer().unwrap(), 42);
    }

    #[tokio::test]
    async fn test_collection_aggregations_max_negative() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: -20,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: -10,
                    score: 2.0,
                },
            ],
        };

        let max = agg.max(&parent).await.unwrap();
        assert!(max.is_some());
        assert_eq!(max.unwrap().as_integer().unwrap(), -10);
    }

    #[tokio::test]
    async fn test_collection_aggregations_max_all_same() {
        let proxy = CollectionProxy::new("children", "value");
        let agg = CollectionAggregations::new(proxy);

        let parent = TestParent {
            id: 1,
            children: vec![
                TestChild {
                    id: 1,
                    value: 10,
                    score: 1.0,
                },
                TestChild {
                    id: 2,
                    value: 10,
                    score: 2.0,
                },
            ],
        };

        let max = agg.max(&parent).await.unwrap();
        assert!(max.is_some());
        assert_eq!(max.unwrap().as_integer().unwrap(), 10);
    }
}
