//! Serializers for Reinhardt REST framework

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
