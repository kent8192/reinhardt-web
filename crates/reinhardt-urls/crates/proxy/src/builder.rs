//! Builder pattern for creating association proxies

use crate::proxy::AssociationProxy;
use std::marker::PhantomData;

/// Builder for creating association proxies with fluent API
///
/// ## Example
///
/// ```rust,ignore
/// let proxy = ProxyBuilder::new()
///     .relationship("user_keywords")
///     .attribute("keyword")
///     .creator(|keyword| UserKeyword::new(keyword))
///     .build();
/// ```
pub struct ProxyBuilder<T, U> {
    relationship: Option<String>,
    attribute: Option<String>,
    creator: Option<fn(U) -> T>,
    _phantom: PhantomData<(T, U)>,
}

impl<T, U> Default for ProxyBuilder<T, U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, U> ProxyBuilder<T, U> {
    /// Create a new proxy builder
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ProxyBuilder;
    ///
    /// let builder: ProxyBuilder<(), ()> = ProxyBuilder::new();
    /// // Builder is ready to configure
    /// ```
    pub fn new() -> Self {
        Self {
            relationship: None,
            attribute: None,
            creator: None,
            _phantom: PhantomData,
        }
    }
    /// Set the relationship name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ProxyBuilder;
    ///
    /// let builder: ProxyBuilder<(), ()> = ProxyBuilder::new()
    ///     .relationship("posts");
    /// // Builder now has relationship set
    /// ```
    pub fn relationship(mut self, name: &str) -> Self {
        self.relationship = Some(name.to_string());
        self
    }
    /// Set the attribute name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ProxyBuilder;
    ///
    /// let builder: ProxyBuilder<(), ()> = ProxyBuilder::new()
    ///     .relationship("posts")
    ///     .attribute("title");
    /// // Builder now has both relationship and attribute set
    /// ```
    pub fn attribute(mut self, name: &str) -> Self {
        self.attribute = Some(name.to_string());
        self
    }
    /// Set the creator function
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ProxyBuilder;
    ///
    /// fn create_item(value: i32) -> String { format!("item_{}", value) }
    ///
    /// let builder = ProxyBuilder::new()
    ///     .relationship("items")
    ///     .attribute("value")
    ///     .creator(create_item);
    /// // Builder now has creator function set
    /// ```
    pub fn creator(mut self, creator: fn(U) -> T) -> Self {
        self.creator = Some(creator);
        self
    }
    /// Build the association proxy
    ///
    /// # Panics
    ///
    /// Panics if relationship or attribute is not set
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ProxyBuilder;
    ///
    /// let proxy: reinhardt_proxy::AssociationProxy<(), ()> = ProxyBuilder::new()
    ///     .relationship("posts")
    ///     .attribute("title")
    ///     .build();
    /// assert_eq!(proxy.relationship, "posts");
    /// assert_eq!(proxy.attribute, "title");
    /// ```
    pub fn build(self) -> AssociationProxy<T, U> {
        let relationship = self.relationship.expect("Relationship must be set");
        let attribute = self.attribute.expect("Attribute must be set");

        let mut proxy = AssociationProxy::new(&relationship, &attribute);
        if let Some(creator) = self.creator {
            proxy = proxy.with_creator(creator);
        }
        proxy
    }
    /// Build the association proxy, returning None if configuration is incomplete
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_proxy::ProxyBuilder;
    ///
    /// // Complete configuration
    /// let proxy: Option<reinhardt_proxy::AssociationProxy<(), ()>> = ProxyBuilder::new()
    ///     .relationship("posts")
    ///     .attribute("title")
    ///     .try_build();
    /// assert!(proxy.is_some());
    ///
    /// // Incomplete configuration
    /// let incomplete: Option<reinhardt_proxy::AssociationProxy<(), ()>> = ProxyBuilder::new()
    ///     .relationship("posts")
    ///     .try_build();
    /// assert!(incomplete.is_none());
    /// ```
    pub fn try_build(self) -> Option<AssociationProxy<T, U>> {
        let relationship = self.relationship?;
        let attribute = self.attribute?;

        let mut proxy = AssociationProxy::new(&relationship, &attribute);
        if let Some(creator) = self.creator {
            proxy = proxy.with_creator(creator);
        }
        Some(proxy)
    }
}

/// Helper function to create a simple association proxy
///
/// ## Example
///
/// ```rust,ignore
/// let proxy = association_proxy("user_keywords", "keyword");
/// ```
pub fn association_proxy<T, U>(relationship: &str, attribute: &str) -> AssociationProxy<T, U> {
    AssociationProxy::new(relationship, attribute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_builder_basic_unit() {
        let proxy: AssociationProxy<(), ()> = ProxyBuilder::new()
            .relationship("rel")
            .attribute("attr")
            .build();

        assert_eq!(proxy.relationship, "rel");
        assert_eq!(proxy.attribute, "attr");
    }
}
