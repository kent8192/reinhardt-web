//! ListView for displaying lists of objects.

use async_trait::async_trait;
use reinhardt_apps::{Request, Response};
use reinhardt_exception::{Error, Result};
use reinhardt_orm::Model;
use reinhardt_serializers::Serializer;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::core::View;
use crate::mixins::MultipleObjectMixin;

/// ListView for displaying multiple objects
pub struct ListView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<Input = T, Output = String> + Send + Sync,
{
    objects: Vec<T>,
    ordering: Option<Vec<String>>,
    paginate_by: Option<usize>,
    allow_empty_flag: bool,
    context_object_name: Option<String>,
    _serializer: PhantomData<S>,
}

impl<T, S> ListView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<Input = T, Output = String> + Send + Sync,
{
    /// Creates a new `ListView` with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::{ListView, MultipleObjectMixin};
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let view = ListView::<Article, JsonSerializer<Article>>::new();
    /// assert!(view.get_context_object_name().is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            ordering: None,
            paginate_by: None,
            allow_empty_flag: true,
            context_object_name: None,
            _serializer: PhantomData,
        }
    }
    /// Sets the list of objects to display in the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::{ListView, MultipleObjectMixin};
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let articles = vec![
    ///     Article { id: Some(1), title: "First".to_string() },
    ///     Article { id: Some(2), title: "Second".to_string() },
    /// ];
    ///
    /// let view = ListView::<Article, JsonSerializer<Article>>::new()
    ///     .with_objects(articles.clone());
    /// # tokio_test::block_on(async {
    /// let objects = view.get_objects().await.unwrap();
    /// assert_eq!(objects.len(), 2);
    /// assert_eq!(objects[0].title, "First");
    /// # });
    /// ```
    pub fn with_objects(mut self, objects: Vec<T>) -> Self {
        self.objects = objects;
        self
    }
    /// Sets the ordering for the object list.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::{ListView, MultipleObjectMixin};
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let view = ListView::<Article, JsonSerializer<Article>>::new()
    ///     .with_ordering(vec!["-created_at".to_string(), "title".to_string()]);
    ///
    /// assert_eq!(view.get_ordering(), Some(vec!["-created_at".to_string(), "title".to_string()]));
    /// ```
    pub fn with_ordering(mut self, ordering: Vec<String>) -> Self {
        self.ordering = Some(ordering);
        self
    }
    /// Sets the number of items per page.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::{ListView, MultipleObjectMixin};
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let view = ListView::<Article, JsonSerializer<Article>>::new()
    ///     .with_paginate_by(25);
    ///
    /// assert_eq!(view.get_paginate_by(), Some(25));
    /// ```
    pub fn with_paginate_by(mut self, paginate_by: usize) -> Self {
        self.paginate_by = Some(paginate_by);
        self
    }
    /// Sets whether to allow empty result sets.
    ///
    /// When set to `false`, the view will return an error if no objects are found.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::{ListView, MultipleObjectMixin};
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let view = ListView::<Article, JsonSerializer<Article>>::new()
    ///     .with_allow_empty(false);
    ///
    /// assert_eq!(view.allow_empty(), false);
    /// ```
    pub fn with_allow_empty(mut self, allow_empty: bool) -> Self {
        self.allow_empty_flag = allow_empty;
        self
    }
    /// Sets a custom name for the object list in the context.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::{ListView, MultipleObjectMixin};
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::Model;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let view = ListView::<Article, JsonSerializer<Article>>::new()
    ///     .with_context_object_name("articles");
    ///
    /// assert_eq!(view.get_context_object_name(), Some("articles"));
    /// ```
    pub fn with_context_object_name(mut self, name: impl Into<String>) -> Self {
        self.context_object_name = Some(name.into());
        self
    }
}

#[async_trait]
impl<T, S> MultipleObjectMixin<T> for ListView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<Input = T, Output = String> + Send + Sync,
{
    async fn get_objects(&self) -> Result<Vec<T>> {
        Ok(self.objects.clone())
    }

    fn get_ordering(&self) -> Option<Vec<String>> {
        self.ordering.clone()
    }

    fn allow_empty(&self) -> bool {
        self.allow_empty_flag
    }

    fn get_paginate_by(&self) -> Option<usize> {
        self.paginate_by
    }

    fn get_context_object_name(&self) -> Option<&str> {
        self.context_object_name.as_deref()
    }
}

#[async_trait]
impl<T, S> View for ListView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    S: Serializer<Input = T, Output = String> + Send + Sync + Default + 'static,
{
    async fn dispatch(&self, request: Request) -> Result<Response> {
        // Handle OPTIONS method
        if request.method == "OPTIONS" {
            let methods = self.allowed_methods().join(", ");
            return Ok(Response::ok()
                .with_header("Allow", &methods)
                .with_header("Content-Type", "application/json"));
        }

        // Support GET and HEAD methods
        let is_head = request.method == "HEAD";
        if !matches!(request.method.as_str(), "GET" | "HEAD") {
            return Err(Error::Validation(format!(
                "Method {} not allowed",
                request.method
            )));
        }

        // Get objects
        let mut object_list = self.get_objects().await?;

        // Check if empty is allowed
        if !self.allow_empty() && object_list.is_empty() {
            return Err(Error::NotFound(
                "Empty list and allow_empty is false".to_string(),
            ));
        }

        // Apply ordering if configured
        if let Some(ordering_fields) = self.get_ordering() {
            // Sort by each field in reverse order (last field is primary sort)
            for field in ordering_fields.iter().rev() {
                let (field_name, descending) = if field.starts_with('-') {
                    (&field[1..], true)
                } else {
                    (field.as_str(), false)
                };

                // Use serde_json::Value for dynamic field comparison
                object_list.sort_by(|a, b| {
                    let a_val = serde_json::to_value(a).unwrap_or(serde_json::Value::Null);
                    let b_val = serde_json::to_value(b).unwrap_or(serde_json::Value::Null);

                    // Extract field value from JSON
                    let a_field = a_val.get(field_name).unwrap_or(&serde_json::Value::Null);
                    let b_field = b_val.get(field_name).unwrap_or(&serde_json::Value::Null);

                    // Compare based on value type
                    let cmp = match (a_field, b_field) {
                        (serde_json::Value::String(a), serde_json::Value::String(b)) => a.cmp(b),
                        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                            // Compare as f64 for numeric values
                            let a_num = a.as_f64().unwrap_or(0.0);
                            let b_num = b.as_f64().unwrap_or(0.0);
                            a_num
                                .partial_cmp(&b_num)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => a.cmp(b),
                        (serde_json::Value::Null, serde_json::Value::Null) => {
                            std::cmp::Ordering::Equal
                        }
                        (serde_json::Value::Null, _) => std::cmp::Ordering::Less,
                        (_, serde_json::Value::Null) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    };

                    if descending { cmp.reverse() } else { cmp }
                });
            }
        }

        // Apply pagination if configured
        let total_count = object_list.len();
        let (paginated_objects, pagination_metadata) =
            if let Some(page_size) = self.get_paginate_by() {
                // Parse page number from query params (default to 1)
                let page: usize = request
                    .query_params
                    .get("page")
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(1);

                // Validate page number
                let page = if page < 1 { 1 } else { page };

                // Calculate pagination
                let start = (page - 1) * page_size;
                let end = start + page_size;

                // Apply pagination
                let paginated = if start < object_list.len() {
                    object_list[start..end.min(object_list.len())].to_vec()
                } else {
                    Vec::new()
                };

                // Build pagination metadata
                let total_pages = (total_count + page_size - 1) / page_size; // Ceiling division
                let has_next = page < total_pages;
                let has_previous = page > 1;

                let metadata = serde_json::json!({
                    "count": total_count,
                    "page": page,
                    "page_size": page_size,
                    "total_pages": total_pages,
                    "next": if has_next { Some(page + 1) } else { None },
                    "previous": if has_previous { Some(page - 1) } else { None },
                });

                (paginated, Some(metadata))
            } else {
                (object_list, None)
            };

        // Serialize objects
        let serializer = S::default();
        let serialized_objects: Result<Vec<_>> = paginated_objects
            .iter()
            .map(|obj| serializer.serialize(obj).map_err(|e| e.into()))
            .collect();

        let serialized_objects = serialized_objects?;

        // Build response - for HEAD, return same headers but empty body
        if is_head {
            Ok(Response::ok().with_header("Content-Type", "application/json"))
        } else {
            // If pagination is enabled, wrap results in DRF-style format
            if let Some(metadata) = pagination_metadata {
                let response_data = serde_json::json!({
                    "count": metadata["count"],
                    "page": metadata["page"],
                    "page_size": metadata["page_size"],
                    "total_pages": metadata["total_pages"],
                    "next": metadata["next"],
                    "previous": metadata["previous"],
                    "results": serialized_objects
                });
                Response::ok().with_json(&response_data)
            } else {
                Response::ok().with_json(&serialized_objects)
            }
        }
    }
}
