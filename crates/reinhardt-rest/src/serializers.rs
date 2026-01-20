//! # Reinhardt Serializers
//!
//! Django REST Framework-inspired serializers for data validation and transformation.
//!
//! ## Overview
//!
//! This crate provides serialization, deserialization, and validation capabilities
//! for REST APIs, with ORM integration for model-based serializers.
//!
//! ## Features
//!
//! - **[`Serializer`]**: Base serializer for data validation and transformation
//! - **[`ModelSerializer`]**: Auto-generated serializers from model definitions
//! - **[`NestedSerializer`]**: Handle nested relationships
//! - **[`HyperlinkedModelSerializer`]**: Serializers with hyperlinked relationships
//! - **Field Types**: CharField, IntegerField, DateTimeField, etc.
//! - **Validators**: UniqueValidator, custom validation functions
//! - **Performance**: Query caching, N+1 detection, batch validation
//! - **Content Negotiation**: JSON, XML, and custom parsers
//!
//! ## Quick Start
//!
//! ### Basic Serializer
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{Serializer, CharField, IntegerField, EmailField};
//!
//! struct UserSerializer {
//!     id: IntegerField,
//!     username: CharField,
//!     email: EmailField,
//! }
//!
//! impl Serializer for UserSerializer {
//!     type Output = User;
//!
//!     fn validate(&self, data: &Value) -> ValidationResult<Self::Output> {
//!         // Validation logic
//!     }
//! }
//! ```
//!
//! ### Model Serializer
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::ModelSerializer;
//!
//! // Automatically generates serializer fields from User model
//! let serializer = ModelSerializer::<User>::new()
//!     .fields(&["id", "username", "email", "created_at"])
//!     .read_only(&["id", "created_at"])
//!     .build();
//!
//! // Serialize a user
//! let json = serializer.serialize(&user)?;
//!
//! // Deserialize and validate
//! let user: User = serializer.deserialize(&json_data)?;
//! ```
//!
//! ## Relation Fields
//!
//! Handle model relationships:
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{
//!     PrimaryKeyRelatedField, SlugRelatedField,
//!     HyperlinkedRelatedField, StringRelatedField
//! };
//!
//! // Primary key representation
//! let author = PrimaryKeyRelatedField::<Author>::new();
//! // Output: {"author": 1}
//!
//! // Slug field representation
//! let category = SlugRelatedField::<Category>::new("slug");
//! // Output: {"category": "technology"}
//!
//! // Hyperlink representation
//! let author = HyperlinkedRelatedField::<Author>::new("author-detail");
//! // Output: {"author": "http://example.com/api/authors/1/"}
//!
//! // String representation (uses __str__)
//! let tags = StringRelatedField::<Tag>::many();
//! // Output: {"tags": ["Python", "Rust", "Web"]}
//! ```
//!
//! ## Nested Serializers
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{NestedSerializer, WritableNestedSerializer, ListSerializer};
//!
//! // Read-only nested serializer
//! let author = NestedSerializer::<AuthorSerializer>::new();
//! // Output: {"author": {"id": 1, "name": "Alice"}}
//!
//! // Writable nested serializer (supports create/update)
//! let profile = WritableNestedSerializer::<ProfileSerializer>::new();
//!
//! // List of nested objects
//! let comments = ListSerializer::<CommentSerializer>::new();
//! // Output: {"comments": [{"id": 1, "text": "..."}, ...]}
//! ```
//!
//! ## Validation
//!
//! ### Field-Level Validation
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{FieldValidator, ValidationError};
//!
//! fn validate_username(value: &str) -> Result<(), ValidationError> {
//!     if value.len() < 3 {
//!         return Err(ValidationError::new("Username must be at least 3 characters"));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Object-Level Validation
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{ObjectValidator, ValidationError};
//!
//! fn validate_password_match(data: &Value) -> Result<(), ValidationError> {
//!     let password = data.get("password");
//!     let confirm = data.get("password_confirm");
//!     if password != confirm {
//!         return Err(ValidationError::new("Passwords do not match"));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Database Validators
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{UniqueValidator, UniqueTogetherValidator};
//!
//! // Ensure email is unique
//! let email_validator = UniqueValidator::<User>::new("email");
//!
//! // Ensure (user_id, slug) is unique together
//! let slug_validator = UniqueTogetherValidator::<Post>::new(&["user_id", "slug"]);
//! ```
//!
//! ## Performance Optimization
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::{QueryCache, N1Detector, BatchValidator};
//!
//! // Cache repeated queries
//! let cache = QueryCache::new();
//! let serializer = serializer.with_cache(&cache);
//!
//! // Detect N+1 query issues
//! let detector = N1Detector::new();
//! let result = detector.analyze(&serializer, &queryset)?;
//! if let Some(warning) = result.warning() {
//!     eprintln!("N+1 detected: {}", warning);
//! }
//!
//! // Batch validation for multiple objects
//! let validator = BatchValidator::new();
//! let results = validator.validate_many(&objects)?;
//! ```
//!
//! ## Content Negotiation
//!
//! ```rust,ignore
//! use reinhardt_rest::serializers::ContentNegotiator;
//!
//! let negotiator = ContentNegotiator::new()
//!     .add_parser("application/json", JsonParser::new())
//!     .add_parser("application/xml", XmlParser::new());
//!
//! let parser = negotiator.select(&request)?;
//! let data = parser.parse(&request.body)?;
//! ```

// Re-export base layer types from reinhardt-core
pub use reinhardt_core::serializers::{
	arena::{FieldValue, SerializationArena, SerializedValue},
	fields::{
		BooleanField, CharField, ChoiceField, DateField, DateTimeField, EmailField, FieldError,
		FloatField, IntegerField, URLField,
	},
	recursive::{RecursiveError, RecursiveResult, SerializationContext},
	serializer::{Deserializer, JsonSerializer, Serializer, SerializerError, ValidatorError},
	validator::{
		FieldLevelValidation, FieldValidator, ObjectLevelValidation, ObjectValidator,
		ValidationError, ValidationResult, validate_fields,
	},
};

// REST-specific modules (ORM-integrated features)
pub mod cache_invalidation;
pub mod content_negotiation;
pub mod hyperlinked;
pub mod introspection;
pub mod meta;
pub mod method_field;
pub mod model_serializer;
pub mod nested;
pub mod nested_config;
pub mod nested_orm;
pub mod parsers;
pub mod performance;
pub mod pool_manager;
pub mod queryset_integration;
pub mod relation_fields_orm;
pub mod relations;
pub mod validator_config;
pub mod validators;

// Re-export REST-specific types
pub use cache_invalidation::{CacheInvalidator, InvalidationStrategy};
pub use content_negotiation::ContentNegotiator;
pub use hyperlinked::{HyperlinkedModelSerializer, UrlReverser};
pub use introspection::{FieldInfo, FieldIntrospector, TypeMapper};
pub use meta::{DefaultMeta, MetaConfig, SerializerMeta};
pub use method_field::{
	MethodFieldError, MethodFieldProvider, MethodFieldRegistry, SerializerMethodField,
};
pub use model_serializer::ModelSerializer;
pub use nested::{ListSerializer, NestedSerializer, WritableNestedSerializer};
pub use nested_config::{NestedFieldConfig, NestedSerializerConfig};
pub use nested_orm::{
	ManyToManyManager, NestedSaveContext, NestedSerializerSave, TransactionHelper,
};
pub use performance::{
	BatchValidator, IntrospectionCache, N1Detector, PerformanceMetrics, PerformanceStats,
	QueryCache,
};
pub use pool_manager::{ConnectionPoolManager, default_pool_config};
pub use queryset_integration::{CacheAwareSaveContext, SaveContext, SerializerSaveMixin};
pub use relation_fields_orm::{
	OptimizableRelationField, PrimaryKeyRelatedFieldORM, QueryOptimizer, SlugRelatedFieldORM,
};
pub use relations::{
	HyperlinkedRelatedField, IdentityField, ManyRelatedField, PrimaryKeyRelatedField,
	RelationField, SlugRelatedField, StringRelatedField,
};
pub use validator_config::ValidatorConfig;
pub use validators::{DatabaseValidatorError, UniqueTogetherValidator, UniqueValidator};
