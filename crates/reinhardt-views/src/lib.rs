//! # Reinhardt Views
//!
//! Generic views for Reinhardt framework, inspired by Django's class-based views.
//!
//! ## Features
//!
//! - **ListView**: Display a list of objects with pagination support
//! - **DetailView**: Display a single object
//! - **CreateView**: Handle object creation
//! - **UpdateView**: Handle object updates
//! - **DeleteView**: Handle object deletion
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_views::{ListView, DetailView, View};
//! use reinhardt_serializers::JsonSerializer;
//! use reinhardt_orm::{Model, QuerySet};
//! use reinhardt_apps::{Request, Response};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct User {
//!     id: Option<i64>,
//!     username: String,
//!     email: String,
//! }
//!
//! impl Model for User {
//!     type PrimaryKey = i64;
//!     fn table_name() -> &'static str { "users" }
//!     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
//!     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
//! }
//!
//! // Create a ListView to display paginated users
//! let users = vec![
//!     User { id: Some(1), username: "alice".to_string(), email: "alice@example.com".to_string() },
//!     User { id: Some(2), username: "bob".to_string(), email: "bob@example.com".to_string() },
//! ];
//!
//! let list_view = ListView::<User, JsonSerializer<User>>::new()
//!     .with_objects(users.clone())
//!     .with_paginate_by(10)
//!     .with_ordering(vec!["-id".to_string()]);
//!
//! // Create a DetailView to display a single user
//! let detail_view = DetailView::<User, JsonSerializer<User>>::new()
//!     .with_object(users[0].clone())
//!     .with_context_object_name("user");
//!
//! // Use the views in request handlers
//! async fn handle_list(request: Request) -> Result<Response, reinhardt_exception::Error> {
//!     list_view.dispatch(request).await
//! }
//!
//! async fn handle_detail(request: Request) -> Result<Response, reinhardt_exception::Error> {
//!     detail_view.dispatch(request).await
//! }
//! ```

// Re-export from views-core
pub use reinhardt_views_core::browsable_api;
pub use reinhardt_views_core::generic;

// Re-export viewsets if the feature is enabled
#[cfg(feature = "viewsets")]
pub use reinhardt_viewsets;

use async_trait::async_trait;
use reinhardt_apps::{Request, Response};
use reinhardt_exception::{Error, Result};
use reinhardt_orm::{
    query::{Filter, FilterOperator, FilterValue},
    Model, QuerySet,
};
use reinhardt_serializers::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::marker::PhantomData;
// Note: 以前はcrate内に`ViewError`を定義していたが、統一のため`reinhardt_exception::Error`を利用する

/// Base trait for all generic views
#[async_trait]
pub trait View: Send + Sync {
    async fn dispatch(&self, request: Request) -> Result<Response>;

    /// Returns the list of HTTP methods allowed by this view
    fn allowed_methods(&self) -> Vec<&'static str> {
        vec!["GET", "HEAD", "OPTIONS"]
    }
}

/// Context data for template rendering
pub type Context = HashMap<String, serde_json::Value>;

/// Trait for views that work with multiple objects
#[async_trait]
pub trait MultipleObjectMixin<T>: Send + Sync
where
    T: Model + Serialize + Send + Sync + Clone,
{
    /// Get objects for this view
    async fn get_objects(&self) -> Result<Vec<T>>;

    /// Get the ordering for the queryset
    fn get_ordering(&self) -> Option<Vec<String>> {
        None
    }

    /// Whether to allow empty result sets
    fn allow_empty(&self) -> bool {
        true
    }

    /// Get the number of items per page
    fn get_paginate_by(&self) -> Option<usize> {
        None
    }

    /// Get the context object name
    fn get_context_object_name(&self) -> Option<&str> {
        None
    }

    /// Build context data for the view
    fn get_context_data(&self, object_list: Vec<T>) -> Result<Context> {
        let mut context = Context::new();
        context.insert("object_list".to_string(), json!(object_list));

        if let Some(name) = self.get_context_object_name() {
            context.insert(name.to_string(), json!(object_list));
        }

        Ok(context)
    }
}

/// Trait for views that work with a single object
#[async_trait]
pub trait SingleObjectMixin<T>: Send + Sync
where
    T: Model + Serialize + Send + Sync + Clone,
{
    /// Get the slug field name
    fn get_slug_field(&self) -> &str {
        "slug"
    }

    /// Get the primary key URL parameter name
    fn pk_url_kwarg(&self) -> &str {
        "pk"
    }

    /// Get the slug URL parameter name
    fn slug_url_kwarg(&self) -> &str {
        "slug"
    }

    /// Get a single object
    async fn get_object(&self, request: &Request) -> Result<T>;

    /// Get the context object name
    fn get_context_object_name(&self) -> Option<&str> {
        None
    }

    /// Build context data for the view
    fn get_context_data(&self, object: T) -> Result<Context> {
        let mut context = Context::new();
        context.insert("object".to_string(), json!(object));

        if let Some(name) = self.get_context_object_name() {
            context.insert(name.to_string(), json!(object));
        }

        Ok(context)
    }
}

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
    /// use reinhardt_views::ListView;
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
    /// use reinhardt_views::ListView;
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
    /// use reinhardt_views::ListView;
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
    /// use reinhardt_views::ListView;
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
    /// use reinhardt_views::ListView;
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
    /// use reinhardt_views::ListView;
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

                    if descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
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

/// DetailView for displaying a single object
pub struct DetailView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<Input = T, Output = String> + Send + Sync,
{
    object: Option<T>,
    queryset: Option<QuerySet<T>>,
    slug_field: String,
    pk_url_kwarg_name: String,
    slug_url_kwarg_name: String,
    context_object_name: Option<String>,
    _serializer: PhantomData<S>,
}

impl<T, S> DetailView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<Input = T, Output = String> + Send + Sync,
{
    /// Creates a new `DetailView` with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::DetailView;
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
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new();
    /// assert_eq!(view.get_slug_field(), "slug");
    /// assert_eq!(view.pk_url_kwarg(), "pk");
    /// ```
    pub fn new() -> Self {
        Self {
            object: None,
            queryset: None,
            slug_field: "slug".to_string(),
            pk_url_kwarg_name: "pk".to_string(),
            slug_url_kwarg_name: "slug".to_string(),
            context_object_name: None,
            _serializer: PhantomData,
        }
    }
    /// Sets the object to display in the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::DetailView;
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
    /// let article = Article { id: Some(1), title: "Hello World".to_string() };
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new()
    ///     .with_object(article.clone());
    /// # tokio_test::block_on(async {
    /// let result = view.get_object(&reinhardt_apps::Request::default()).await;
    /// assert!(result.is_ok());
    /// # });
    /// ```
    pub fn with_object(mut self, object: T) -> Self {
        self.object = Some(object);
        self
    }
    /// Sets the slug field name for object lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::DetailView;
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
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new()
    ///     .with_slug_field("title");
    /// assert_eq!(view.get_slug_field(), "title");
    /// ```
    pub fn with_slug_field(mut self, slug_field: impl Into<String>) -> Self {
        self.slug_field = slug_field.into();
        self
    }
    /// Sets the primary key URL parameter name.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::DetailView;
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
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new()
    ///     .with_pk_url_kwarg("article_id");
    /// assert_eq!(view.pk_url_kwarg(), "article_id");
    /// ```
    pub fn with_pk_url_kwarg(mut self, kwarg: impl Into<String>) -> Self {
        self.pk_url_kwarg_name = kwarg.into();
        self
    }
    /// Sets the slug URL parameter name.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::DetailView;
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
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new()
    ///     .with_slug_url_kwarg("article_slug");
    /// assert_eq!(view.slug_url_kwarg(), "article_slug");
    /// ```
    pub fn with_slug_url_kwarg(mut self, kwarg: impl Into<String>) -> Self {
        self.slug_url_kwarg_name = kwarg.into();
        self
    }
    /// Sets a custom name for the object in the context.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_views::DetailView;
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
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new()
    ///     .with_context_object_name("article");
    /// assert_eq!(view.get_context_object_name(), Some("article"));
    /// ```
    pub fn with_context_object_name(mut self, name: impl Into<String>) -> Self {
        self.context_object_name = Some(name.into());
        self
    }
    /// Sets the QuerySet for database lookup.
    ///
    /// When a QuerySet is configured, the view will use it to fetch objects from the database
    /// based on the primary key or slug URL parameter.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use reinhardt_views::DetailView;
    /// use reinhardt_serializers::JsonSerializer;
    /// use reinhardt_orm::{Model, QuerySet};
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct Article {
    ///     id: Option<i64>,
    ///     title: String,
    ///     slug: String,
    /// }
    ///
    /// impl Model for Article {
    ///     type PrimaryKey = i64;
    ///     fn table_name() -> &'static str { "articles" }
    ///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    ///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// }
    ///
    /// let queryset = QuerySet::<Article>::new();
    /// let view = DetailView::<Article, JsonSerializer<Article>>::new()
    ///     .with_queryset(queryset);
    /// ```
    pub fn with_queryset(mut self, queryset: QuerySet<T>) -> Self {
        self.queryset = Some(queryset);
        self
    }
}

#[async_trait]
impl<T, S> SingleObjectMixin<T> for DetailView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<Input = T, Output = String> + Send + Sync,
{
    async fn get_object(&self, request: &Request) -> Result<T> {
        // If object is already set, return it
        if let Some(ref object) = self.object {
            return Ok(object.clone());
        }

        // Get queryset or return error if not configured
        let queryset = self.queryset.as_ref().ok_or_else(|| {
            Error::NotFound(
                "No queryset configured. Set object directly with with_object() \
                 or configure queryset with with_queryset()."
                    .to_string(),
            )
        })?;

        // Try to extract pk or slug from URL parameters
        let pk_kwarg = self.pk_url_kwarg();
        let slug_kwarg = self.slug_url_kwarg();

        // Try to get pk from URL parameters
        if let Some(pk_value) = request.path_params.get(pk_kwarg) {
            let pk_field = T::primary_key_field();

            // Try to parse as i64 first (most common primary key type)
            let filter_value = if let Ok(int_pk) = pk_value.parse::<i64>() {
                FilterValue::Integer(int_pk)
            } else {
                // Fallback to string if not a valid integer
                FilterValue::String(pk_value.clone())
            };

            let results = queryset
                .clone()
                .filter(Filter {
                    field: pk_field.to_string(),
                    operator: FilterOperator::Eq,
                    value: filter_value,
                })
                .all();

            return results.into_iter().next().ok_or_else(|| {
                Error::NotFound(format!("Object with pk='{}' not found", pk_value))
            });
        }

        // Try to get slug from URL parameters
        if let Some(slug_value) = request.path_params.get(slug_kwarg) {
            let results = queryset
                .clone()
                .filter(Filter {
                    field: self.get_slug_field().to_string(),
                    operator: FilterOperator::Eq,
                    value: FilterValue::String(slug_value.clone()),
                })
                .all();

            return results.into_iter().next().ok_or_else(|| {
                Error::NotFound(format!("Object with slug='{}' not found", slug_value))
            });
        }

        Err(Error::NotFound("No pk or slug provided in URL".to_string()))
    }

    fn get_slug_field(&self) -> &str {
        &self.slug_field
    }

    fn pk_url_kwarg(&self) -> &str {
        &self.pk_url_kwarg_name
    }

    fn slug_url_kwarg(&self) -> &str {
        &self.slug_url_kwarg_name
    }

    fn get_context_object_name(&self) -> Option<&str> {
        self.context_object_name.as_deref()
    }
}

#[async_trait]
impl<T, S> View for DetailView<T, S>
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

        // Get object - extracts pk/slug from URL parameters
        let object = self.get_object(&request).await?;

        // Serialize object
        let serializer = S::default();
        let serialized = serializer.serialize(&object)?;

        // Build response - for HEAD, return same headers but empty body
        if is_head {
            Ok(Response::ok().with_header("Content-Type", "application/json"))
        } else {
            Response::ok().with_json(&serialized)
        }
    }
}
