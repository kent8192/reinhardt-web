/// ViewSetHandler - wraps a ViewSet as a Handler
use crate::{Action, ViewSet};
use async_trait::async_trait;
use hyper::Method;
use reinhardt_auth::{Permission, PermissionContext};
use reinhardt_core::apps::{Handler, Request, Response, Result};
use reinhardt_db::orm::{Model, query_types::DbBackend};
use reinhardt_filters::FilterBackend;
use reinhardt_serializers::{ModelSerializer, Serializer};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

/// Handler implementation that wraps a ViewSet
pub struct ViewSetHandler<V: ViewSet> {
	viewset: Arc<V>,
	action_map: HashMap<Method, String>,
	#[allow(dead_code)]
	name: Option<String>,
	#[allow(dead_code)]
	suffix: Option<String>,
	injection_context: Option<Arc<reinhardt_core::di::InjectionContext>>,

	// Attributes set after as_view() is called
	// These mirror Django REST Framework's behavior
	args: RwLock<Option<Vec<String>>>,
	kwargs: RwLock<Option<HashMap<String, String>>>,
	has_handled_request: RwLock<bool>,
}

impl<V: ViewSet> ViewSetHandler<V> {
	pub fn new(
		viewset: Arc<V>,
		action_map: HashMap<Method, String>,
		name: Option<String>,
		suffix: Option<String>,
	) -> Self {
		Self {
			viewset,
			action_map,
			name,
			suffix,
			injection_context: None,
			args: RwLock::new(None),
			kwargs: RwLock::new(None),
			has_handled_request: RwLock::new(false),
		}
	}

	/// Set the dependency injection context for this handler
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::ViewSetHandler;
	/// use reinhardt_core::di::{InjectionContext, SingletonScope};
	/// use std::sync::Arc;
	///
	/// # fn example() {
	/// let singleton = Arc::new(SingletonScope::new());
	/// let ctx = Arc::new(InjectionContext::new(singleton));
	///
	/// // let handler = ViewSetHandler::new(...)
	/// //     .with_di_context(ctx);
	/// # }
	/// ```
	pub fn with_di_context(mut self, ctx: Arc<reinhardt_core::di::InjectionContext>) -> Self {
		self.injection_context = Some(ctx);
		self
	}

	/// Check if args attribute is set (for testing)
	pub fn has_args(&self) -> bool {
		self.args.read().unwrap().is_some()
	}

	/// Check if kwargs attribute is set (for testing)
	pub fn has_kwargs(&self) -> bool {
		self.kwargs.read().unwrap().is_some()
	}

	/// Check if request attribute is set (for testing)
	pub fn has_request(&self) -> bool {
		*self.has_handled_request.read().unwrap()
	}

	/// Check if action_map is set (for testing)
	pub fn has_action_map(&self) -> bool {
		!self.action_map.is_empty()
	}
}

#[async_trait]
impl<V: ViewSet + 'static> Handler for ViewSetHandler<V> {
	async fn handle(&self, mut request: Request) -> Result<Response> {
		// Set attributes when handling request (DRF behavior)
		*self.has_handled_request.write().unwrap() = true;
		*self.args.write().unwrap() = Some(Vec::new());

		// Extract path parameters from URI
		let kwargs = extract_path_params(&request);
		*self.kwargs.write().unwrap() = Some(kwargs);

		// Process middleware before ViewSet
		if let Some(middleware) = self.viewset.get_middleware()
			&& let Some(response) = middleware.process_request(&mut request).await?
		{
			return Ok(response);
		}

		// Resolve action from HTTP method
		let action_name = self.action_map.get(&request.method).ok_or_else(|| {
			reinhardt_core::exception::Error::Http(format!("Method {} not allowed", request.method))
		})?;

		// Create Action from name
		let action = Action::from_name(action_name);

		// Dispatch to ViewSet (with DI support if available)
		let response = if self.viewset.supports_di() {
			// ViewSet supports DI - use dispatch_with_context
			if let Some(ctx) = &self.injection_context {
				self.viewset
					.dispatch_with_context(request, action, ctx)
					.await?
			} else {
				return Err(reinhardt_core::exception::Error::Internal(
                    "ViewSet requires DI context but none was provided. Use .with_di_context() to configure.".to_string()
                ));
			}
		} else {
			// Standard dispatch without DI
			self.viewset.dispatch(request, action).await?
		};

		// Process middleware after ViewSet
		Ok(response)
	}
}

/// Extract path parameters from request
/// Simple implementation - in production would use router's path matching
fn extract_path_params(request: &Request) -> HashMap<String, String> {
	let mut params = HashMap::new();

	// Simple extraction: if path has pattern like /resource/123/
	// extract "123" as the "id" parameter
	let path = request.uri.path();
	let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

	// If we have at least 2 segments, assume second is an ID
	if segments.len() >= 2 {
		// Check if second segment looks like a numeric ID
		if segments[1].parse::<i64>().is_ok() || !segments[1].is_empty() {
			params.insert("id".to_string(), segments[1].to_string());
		}
	}

	params
}

/// Error type for ModelViewSetHandler
#[derive(Debug)]
pub enum ViewError {
	Serialization(String),
	Permission(String),
	NotFound(String),
	BadRequest(String),
	Internal(String),
	DatabaseError(String),
}

impl std::fmt::Display for ViewError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ViewError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
			ViewError::Permission(msg) => write!(f, "Permission denied: {}", msg),
			ViewError::NotFound(msg) => write!(f, "Not found: {}", msg),
			ViewError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
			ViewError::Internal(msg) => write!(f, "Internal error: {}", msg),
			ViewError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
		}
	}
}

impl std::error::Error for ViewError {}

/// Django REST Framework-style ViewSet handler for models
///
/// Provides automatic CRUD operations with permission checks, filtering,
/// pagination, and serialization for Model types.
///
/// # Examples
///
/// ```no_run
/// # use reinhardt_viewsets::ModelViewSetHandler;
/// # use reinhardt_db::orm::Model;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct User {
/// #     id: Option<i64>,
/// #     username: String,
/// # }
/// #
/// # impl Model for User {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "users" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// #
/// # async fn example() {
/// let handler = ModelViewSetHandler::<User>::new();
/// # }
/// ```
pub struct ModelViewSetHandler<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
	queryset: Option<Vec<T>>,
	serializer_class: Option<Arc<dyn Serializer<Input = T, Output = String> + Send + Sync>>,
	permission_classes: Vec<Arc<dyn Permission>>,
	filter_backends: Vec<Arc<dyn FilterBackend>>,
	pagination_class: Option<reinhardt_core::pagination::PaginatorImpl>,
	pool: Option<Arc<sqlx::AnyPool>>,
	/// Database backend type (default: PostgreSQL)
	db_backend: DbBackend,
	_phantom: PhantomData<T>,
}

impl<T> ModelViewSetHandler<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
	/// Create a new ModelViewSetHandler
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// let handler = ModelViewSetHandler::<User>::new();
	/// ```
	pub fn new() -> Self {
		Self {
			queryset: None,
			serializer_class: None,
			permission_classes: Vec::new(),
			filter_backends: Vec::new(),
			pagination_class: None,
			pool: None,
			db_backend: DbBackend::Postgres, // Default to PostgreSQL
			_phantom: PhantomData,
		}
	}

	/// Set the queryset (in-memory data) for this handler
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// let users = vec![
	///     User { id: Some(1), username: "alice".to_string() },
	///     User { id: Some(2), username: "bob".to_string() },
	/// ];
	/// let handler = ModelViewSetHandler::<User>::new()
	///     .with_queryset(users);
	/// ```
	pub fn with_queryset(mut self, queryset: Vec<T>) -> Self {
		self.queryset = Some(queryset);
		self
	}

	/// Set the serializer class for this handler
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_serializers::ModelSerializer;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use std::sync::Arc;
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// let serializer = Arc::new(ModelSerializer::<User>::new());
	/// let handler = ModelViewSetHandler::<User>::new()
	///     .with_serializer(serializer);
	/// ```
	pub fn with_serializer(
		mut self,
		serializer: Arc<dyn Serializer<Input = T, Output = String> + Send + Sync>,
	) -> Self {
		self.serializer_class = Some(serializer);
		self
	}

	/// Set the database connection pool for this handler
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use sqlx::AnyPool;
	/// # use std::sync::Arc;
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = Arc::new(AnyPool::connect("postgres://localhost/mydb").await?);
	/// let handler = ModelViewSetHandler::<User>::new()
	///     .with_pool(pool);
	/// # Ok(())
	/// # }
	/// ```
	pub fn with_pool(mut self, pool: Arc<sqlx::AnyPool>) -> Self {
		self.pool = Some(pool);
		self
	}

	/// Set the database backend type for this handler
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_db::orm::{Model, query_types::DbBackend};
	/// # use serde::{Serialize, Deserialize};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// let handler = ModelViewSetHandler::<User>::new()
	///     .with_db_backend(DbBackend::Sqlite);
	/// ```
	pub fn with_db_backend(mut self, db_backend: DbBackend) -> Self {
		self.db_backend = db_backend;
		self
	}

	/// Add a permission class to this handler
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_auth::IsAuthenticated;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use std::sync::Arc;
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// let handler = ModelViewSetHandler::<User>::new()
	///     .add_permission(Arc::new(IsAuthenticated));
	/// ```
	pub fn add_permission(mut self, permission: Arc<dyn Permission>) -> Self {
		self.permission_classes.push(permission);
		self
	}

	/// Add a filter backend to this handler
	pub fn add_filter_backend(mut self, backend: Arc<dyn FilterBackend>) -> Self {
		self.filter_backends.push(backend);
		self
	}

	/// Set the pagination class for this handler
	pub fn with_pagination(
		mut self,
		pagination: reinhardt_core::pagination::PaginatorImpl,
	) -> Self {
		self.pagination_class = Some(pagination);
		self
	}

	/// Get the queryset for this handler
	fn get_queryset(&self) -> &[T] {
		self.queryset.as_deref().unwrap_or(&[])
	}

	/// Get the serializer for this handler
	fn get_serializer(&self) -> Arc<dyn Serializer<Input = T, Output = String> + Send + Sync> {
		self.serializer_class
			.clone()
			.unwrap_or_else(|| Arc::new(ModelSerializer::<T>::new()))
	}

	/// Check permissions for the request
	async fn check_permissions(&self, request: &Request) -> std::result::Result<(), ViewError> {
		// Extract authentication information from request extensions
		// The session middleware stores authenticated user_id in extensions
		//
		// Expected usage:
		// 1. Session middleware extracts session from cookie/token
		// 2. Middleware validates session and extracts user_id
		// 3. Middleware stores user_id in request.extensions using a dedicated type
		//
		// Example middleware implementation:
		//   if let Some(user_id) = session.get::<i64>("user_id").ok().flatten() {
		//       request.extensions.insert(AuthenticatedUserId(user_id));
		//   }

		// Try to extract user_id from extensions
		// Support both String and UUID formats
		let user_id_string: Option<String> = request.extensions.get::<String>().or_else(|| {
			request
				.extensions
				.get::<uuid::Uuid>()
				.map(|id| id.to_string())
		});

		// Determine authentication status based on user_id presence
		let is_authenticated = user_id_string.is_some();

		// Load user from database if authenticated and pool is available
		let (is_admin, is_active, user_obj) = if let (Some(user_id_str), Some(_pool)) =
			(user_id_string.as_ref(), self.pool.as_ref())
		{
			// Parse user_id as UUID
			#[cfg(feature = "argon2-hasher")]
			match uuid::Uuid::parse_str(user_id_str) {
				Ok(user_uuid) => {
					// Get database connection
					use reinhardt_db::orm::manager::get_connection;
					match get_connection().await {
						Ok(conn) => {
							// Build SQL query to fetch user from database
							use reinhardt_auth::DefaultUser;
							use reinhardt_db::orm::Model;
							use reinhardt_db::orm::connection::QueryValue;

							let table_name = DefaultUser::table_name();
							let pk_field = DefaultUser::primary_key_field();
							let sql =
								format!("SELECT * FROM {} WHERE {} = $1", table_name, pk_field);

							// Execute query with parameter binding
							let params = vec![QueryValue::String(user_uuid.to_string())];

							match conn.query_optional(&sql, params).await {
								Ok(Some(row)) => {
									// Deserialize user from query result
									match serde_json::from_value::<DefaultUser>(row.data) {
										Ok(user) => {
											use reinhardt_auth::User;
											// Extract admin and active status from loaded user
											let is_admin = user.is_admin();
											let is_active = user.is_active();
											// Box the user object to store in PermissionContext
											let boxed_user: Box<dyn User> = Box::new(user);
											(is_admin, is_active, Some(boxed_user))
										}
										Err(_) => {
											// Deserialization failed, use defaults
											(false, true, None)
										}
									}
								}
								Ok(None) => {
									// User not found, use defaults
									(false, true, None)
								}
								Err(_) => {
									// Database query failed, use defaults
									(false, true, None)
								}
							}
						}
						Err(_) => {
							// Connection failed, use defaults
							(false, true, None)
						}
					}
				}
				Err(_) => {
					// UUID parse failed, use defaults
					(false, true, None)
				}
			}

			// When argon2-hasher feature is disabled, DefaultUser is not available
			// Return default values to indicate user retrieval is not supported
			#[cfg(not(feature = "argon2-hasher"))]
			{
				let _ = user_id_str; // Suppress unused variable warning
				(false, true, None)
			}
		} else {
			// Not authenticated or no pool, use defaults
			(false, true, None)
		};

		let context = PermissionContext {
			request,
			is_authenticated,
			is_admin,
			is_active,
			user: user_obj,
		};

		// Check all registered permission classes
		for permission in &self.permission_classes {
			if !permission.has_permission(&context).await {
				// Permission denied - return specific error
				return Err(ViewError::Permission(format!(
					"Permission denied by {}",
					std::any::type_name_of_val(&**permission)
				)));
			}
		}

		Ok(())
	}

	/// List all objects with optional filtering and pagination
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_core::apps::Request;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use bytes::Bytes;
	/// # use hyper::{Method, Uri, Version, HeaderMap};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// #
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let handler = ModelViewSetHandler::<User>::new();
	/// let request = Request::new(
	///     Method::GET,
	///     "/users/".parse::<Uri>()?,
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	/// );
	/// let response = handler.list(&request).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list(&self, request: &Request) -> std::result::Result<Response, ViewError> {
		self.check_permissions(request).await?;

		let queryset = self.get_queryset();
		let serializer = self.get_serializer();

		// Serialize all objects
		let mut serialized_items = Vec::new();
		for item in queryset {
			let json = serializer
				.serialize(item)
				.map_err(|e| ViewError::Serialization(e.to_string()))?;
			serialized_items.push(json);
		}

		// Create response body
		let response_body = format!("[{}]", serialized_items.join(","));

		Ok(Response::ok().with_body(response_body))
	}

	/// Retrieve a single object by primary key
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_core::apps::Request;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use serde_json::Value;
	/// # use bytes::Bytes;
	/// # use hyper::{Method, Uri, Version, HeaderMap};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// #
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let handler = ModelViewSetHandler::<User>::new();
	/// let request = Request::new(
	///     Method::GET,
	///     "/users/1/".parse::<Uri>()?,
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	/// );
	/// let pk = serde_json::json!(1);
	/// let response = handler.retrieve(&request, pk).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn retrieve(
		&self,
		request: &Request,
		pk: serde_json::Value,
	) -> std::result::Result<Response, ViewError> {
		self.check_permissions(request).await?;

		let queryset = self.get_queryset();
		let serializer = self.get_serializer();

		// Find object by primary key
		let item = queryset
			.iter()
			.find(|item| {
				if let Some(item_pk) = item.primary_key() {
					// Compare primary keys by converting both to strings using Display
					let item_pk_str = item_pk.to_string();
					let pk_str = pk.to_string();

					item_pk_str == pk_str
				} else {
					false
				}
			})
			.ok_or_else(|| ViewError::NotFound(format!("Object with pk={} not found", pk)))?;

		let json = serializer
			.serialize(item)
			.map_err(|e| ViewError::Serialization(e.to_string()))?;

		Ok(Response::ok().with_body(json))
	}

	/// Create a new object
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_core::apps::Request;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use bytes::Bytes;
	/// # use hyper::{Method, Uri, Version, HeaderMap};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// #
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let handler = ModelViewSetHandler::<User>::new();
	/// let request = Request::new(
	///     Method::POST,
	///     "/users/".parse::<Uri>()?,
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::from(r#"{"username":"alice"}"#),
	/// );
	/// let response = handler.create(&request).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create(&self, request: &Request) -> std::result::Result<Response, ViewError> {
		self.check_permissions(request).await?;

		let serializer = self.get_serializer();

		// Parse request body
		let body_str = String::from_utf8(request.body().to_vec())
			.map_err(|e| ViewError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

		// Deserialize into model
		let item = serializer
			.deserialize(&body_str)
			.map_err(|e| ViewError::Serialization(e.to_string()))?;

		// Save to database if pool is available
		if let Some(pool) = &self.pool {
			// Create a new session for this request
			let mut session = reinhardt_db::prelude::Session::new(pool.clone(), self.db_backend)
				.await
				.map_err(|e| {
					ViewError::DatabaseError(format!("Failed to create session: {}", e))
				})?;

			// Begin transaction
			session.begin().await.map_err(|e| {
				ViewError::DatabaseError(format!("Failed to begin transaction: {}", e))
			})?;

			// Add object to session
			session
				.add(item.clone())
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to add object: {}", e)))?;

			// Flush changes to database (generates and executes INSERT)
			session
				.flush()
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to flush: {}", e)))?;

			// Commit transaction
			session
				.commit()
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to commit: {}", e)))?;
		}

		Ok(Response::created().with_body(body_str))
	}

	/// Update an existing object
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_core::apps::Request;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use serde_json::Value;
	/// # use bytes::Bytes;
	/// # use hyper::{Method, Uri, Version, HeaderMap};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// #
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let handler = ModelViewSetHandler::<User>::new();
	/// let request = Request::new(
	///     Method::PUT,
	///     "/users/1/".parse::<Uri>()?,
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::from(r#"{"username":"alice_updated"}"#),
	/// );
	/// let pk = serde_json::json!(1);
	/// let response = handler.update(&request, pk).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update(
		&self,
		request: &Request,
		pk: serde_json::Value,
	) -> std::result::Result<Response, ViewError> {
		self.check_permissions(request).await?;

		let serializer = self.get_serializer();

		// Verify object exists
		let _ = self.retrieve(request, pk).await?;

		// Parse request body
		let body_str = String::from_utf8(request.body().to_vec())
			.map_err(|e| ViewError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

		// Deserialize into model
		let item = serializer
			.deserialize(&body_str)
			.map_err(|e| ViewError::Serialization(e.to_string()))?;

		// Update database if pool is available
		if let Some(pool) = &self.pool {
			// Create a new session for this request
			let mut session = reinhardt_db::prelude::Session::new(pool.clone(), self.db_backend)
				.await
				.map_err(|e| {
					ViewError::DatabaseError(format!("Failed to create session: {}", e))
				})?;

			// Begin transaction
			session.begin().await.map_err(|e| {
				ViewError::DatabaseError(format!("Failed to begin transaction: {}", e))
			})?;

			// Add updated object to session (marks as dirty for UPDATE)
			session
				.add(item.clone())
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to add object: {}", e)))?;

			// Flush changes to database (generates and executes UPDATE)
			session
				.flush()
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to flush: {}", e)))?;

			// Commit transaction
			session
				.commit()
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to commit: {}", e)))?;
		}

		Ok(Response::ok().with_body(body_str))
	}

	/// Delete an object
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_viewsets::ModelViewSetHandler;
	/// # use reinhardt_core::apps::Request;
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # use serde_json::Value;
	/// # use bytes::Bytes;
	/// # use hyper::{Method, Uri, Version, HeaderMap};
	/// #
	/// # #[derive(Debug, Clone, Serialize, Deserialize)]
	/// # struct User {
	/// #     id: Option<i64>,
	/// #     username: String,
	/// # }
	/// #
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// # }
	/// #
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let handler = ModelViewSetHandler::<User>::new();
	/// let request = Request::new(
	///     Method::DELETE,
	///     "/users/1/".parse::<Uri>()?,
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	/// );
	/// let pk = serde_json::json!(1);
	/// let response = handler.destroy(&request, pk).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn destroy(
		&self,
		request: &Request,
		pk: serde_json::Value,
	) -> std::result::Result<Response, ViewError> {
		self.check_permissions(request).await?;

		let serializer = self.get_serializer();

		// Verify object exists and get it for deletion
		let response = self.retrieve(request, pk).await?;

		// Extract the object from response body
		let body_str = String::from_utf8(response.body.to_vec())
			.map_err(|e| ViewError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

		// Deserialize into model
		let item = serializer
			.deserialize(&body_str)
			.map_err(|e| ViewError::Serialization(e.to_string()))?;

		// Delete from database if pool is available
		if let Some(pool) = &self.pool {
			// Create a new session for this request
			let mut session = reinhardt_db::prelude::Session::new(pool.clone(), self.db_backend)
				.await
				.map_err(|e| {
					ViewError::DatabaseError(format!("Failed to create session: {}", e))
				})?;

			// Begin transaction
			session.begin().await.map_err(|e| {
				ViewError::DatabaseError(format!("Failed to begin transaction: {}", e))
			})?;

			// Mark object for deletion
			session.delete(item).await.map_err(|e| {
				ViewError::DatabaseError(format!("Failed to mark object for deletion: {}", e))
			})?;

			// Flush changes to database (generates and executes DELETE)
			session
				.flush()
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to flush: {}", e)))?;

			// Commit transaction
			session
				.commit()
				.await
				.map_err(|e| ViewError::DatabaseError(format!("Failed to commit: {}", e)))?;
		}

		Ok(Response::no_content())
	}
}

impl<T> Default for ModelViewSetHandler<T>
where
	T: Model + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
	use reinhardt_auth::{AllowAny, IsAuthenticated};
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestUser {
		id: Option<i64>,
		username: String,
		email: String,
	}

	impl Model for TestUser {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"users"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	fn create_test_users() -> Vec<TestUser> {
		vec![
			TestUser {
				id: Some(1),
				username: "alice".to_string(),
				email: "alice@example.com".to_string(),
			},
			TestUser {
				id: Some(2),
				username: "bob".to_string(),
				email: "bob@example.com".to_string(),
			},
			TestUser {
				id: Some(3),
				username: "charlie".to_string(),
				email: "charlie@example.com".to_string(),
			},
		]
	}

	#[tokio::test]
	async fn test_model_viewset_handler_new() {
		let handler = ModelViewSetHandler::<TestUser>::new();
		assert!(handler.queryset.is_none());
		assert!(handler.serializer_class.is_none());
		assert_eq!(handler.permission_classes.len(), 0);
	}

	#[tokio::test]
	async fn test_model_viewset_handler_with_queryset() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users.clone());
		assert!(handler.queryset.is_some());
		assert_eq!(handler.get_queryset().len(), 3);
	}

	#[tokio::test]
	async fn test_model_viewset_handler_with_serializer() {
		let serializer = Arc::new(ModelSerializer::<TestUser>::new());
		let handler = ModelViewSetHandler::<TestUser>::new().with_serializer(serializer);
		assert!(handler.serializer_class.is_some());
	}

	#[tokio::test]
	async fn test_model_viewset_handler_add_permission() {
		let handler = ModelViewSetHandler::<TestUser>::new()
			.add_permission(Arc::new(AllowAny))
			.add_permission(Arc::new(IsAuthenticated));
		assert_eq!(handler.permission_classes.len(), 2);
	}

	#[tokio::test]
	async fn test_model_viewset_handler_list() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let request = Request::new(
			Method::GET,
			"/users/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let response = handler.list(&request).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body.contains("alice"));
		assert!(body.contains("bob"));
		assert!(body.contains("charlie"));
	}

	#[tokio::test]
	async fn test_model_viewset_handler_retrieve() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let request = Request::new(
			Method::GET,
			"/users/1/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let pk = serde_json::json!(1);
		let response = handler.retrieve(&request, pk).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body.contains("alice"));
	}

	#[tokio::test]
	async fn test_model_viewset_handler_retrieve_not_found() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let request = Request::new(
			Method::GET,
			"/users/999/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let pk = serde_json::json!(999);
		let result = handler.retrieve(&request, pk).await;
		assert!(result.is_err());
		if let Err(e) = result {
			assert!(matches!(e, ViewError::NotFound(_)));
		}
	}

	#[tokio::test]
	async fn test_model_viewset_handler_create() {
		let handler = ModelViewSetHandler::<TestUser>::new();

		let body = r#"{"id":4,"username":"dave","email":"dave@example.com"}"#;
		let request = Request::new(
			Method::POST,
			"/users/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(body),
		);

		let response = handler.create(&request).await.unwrap();
		assert_eq!(response.status, StatusCode::CREATED);

		let response_body = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(response_body.contains("dave"));
	}

	#[tokio::test]
	async fn test_model_viewset_handler_create_invalid_body() {
		let handler = ModelViewSetHandler::<TestUser>::new();

		let body = r#"{"invalid": "data"}"#;
		let request = Request::new(
			Method::POST,
			"/users/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(body),
		);

		let result = handler.create(&request).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_model_viewset_handler_update() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let body = r#"{"id":1,"username":"alice_updated","email":"alice_new@example.com"}"#;
		let request = Request::new(
			Method::PUT,
			"/users/1/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(body),
		);

		let pk = serde_json::json!(1);
		let response = handler.update(&request, pk).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);

		let response_body = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(response_body.contains("alice_updated"));
	}

	#[tokio::test]
	async fn test_model_viewset_handler_update_not_found() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let body = r#"{"id":999,"username":"nonexistent","email":"none@example.com"}"#;
		let request = Request::new(
			Method::PUT,
			"/users/999/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(body),
		);

		let pk = serde_json::json!(999);
		let result = handler.update(&request, pk).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_model_viewset_handler_destroy() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let request = Request::new(
			Method::DELETE,
			"/users/1/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let pk = serde_json::json!(1);
		let response = handler.destroy(&request, pk).await.unwrap();
		assert_eq!(response.status, StatusCode::NO_CONTENT);
	}

	#[tokio::test]
	async fn test_model_viewset_handler_destroy_not_found() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(users);

		let request = Request::new(
			Method::DELETE,
			"/users/999/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let pk = serde_json::json!(999);
		let result = handler.destroy(&request, pk).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_model_viewset_handler_permission_denied() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new()
			.with_queryset(users)
			.add_permission(Arc::new(IsAuthenticated));

		let request = Request::new(
			Method::GET,
			"/users/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let result = handler.list(&request).await;
		assert!(result.is_err());
		if let Err(e) = result {
			assert!(matches!(e, ViewError::Permission(_)));
		}
	}

	#[tokio::test]
	async fn test_model_viewset_handler_allow_any_permission() {
		let users = create_test_users();
		let handler = ModelViewSetHandler::<TestUser>::new()
			.with_queryset(users)
			.add_permission(Arc::new(AllowAny));

		let request = Request::new(
			Method::GET,
			"/users/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let result = handler.list(&request).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_view_error_display() {
		let error = ViewError::Serialization("test".to_string());
		assert_eq!(error.to_string(), "Serialization error: test");

		let error = ViewError::Permission("denied".to_string());
		assert_eq!(error.to_string(), "Permission denied: denied");

		let error = ViewError::NotFound("missing".to_string());
		assert_eq!(error.to_string(), "Not found: missing");

		let error = ViewError::BadRequest("invalid".to_string());
		assert_eq!(error.to_string(), "Bad request: invalid");

		let error = ViewError::Internal("internal".to_string());
		assert_eq!(error.to_string(), "Internal error: internal");
	}

	#[tokio::test]
	async fn test_model_viewset_handler_default() {
		let handler = ModelViewSetHandler::<TestUser>::default();
		assert!(handler.queryset.is_none());
		assert!(handler.serializer_class.is_none());
		assert_eq!(handler.permission_classes.len(), 0);
	}

	#[tokio::test]
	async fn test_model_viewset_handler_empty_queryset() {
		let handler = ModelViewSetHandler::<TestUser>::new().with_queryset(vec![]);

		let request = Request::new(
			Method::GET,
			"/users/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let response = handler.list(&request).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body, "[]");
	}
}
