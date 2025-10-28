//! Serializers for Reinhardt REST framework

pub mod arena;
pub mod cache_invalidation;
pub mod content_negotiation;
pub mod fields;
pub mod hyperlinked;
pub mod introspection;
pub mod meta;
pub mod method_field;
pub mod model_serializer;
pub mod nested;
pub mod nested_config;
pub mod nested_orm_integration;
pub mod parsers;
pub mod performance;
pub mod pool_manager;
pub mod queryset_integration;
pub mod recursive;
pub mod relation_fields_orm;
pub mod relations;
pub mod serializer;
pub mod validator;
pub mod validator_config;
pub mod validators;

pub use arena::{FieldValue, SerializationArena, SerializedValue};
pub use cache_invalidation::{CacheInvalidator, InvalidationStrategy};
pub use content_negotiation::ContentNegotiator;
pub use fields::{
    BooleanField, CharField, ChoiceField, DateField, DateTimeField, EmailField, FieldError,
    FloatField, IntegerField, URLField,
};
pub use hyperlinked::{HyperlinkedModelSerializer, UrlReverser};
pub use introspection::{FieldInfo, FieldIntrospector, TypeMapper};
pub use meta::{DefaultMeta, MetaConfig, SerializerMeta};
pub use method_field::{
    MethodFieldError, MethodFieldProvider, MethodFieldRegistry, SerializerMethodField,
};
pub use model_serializer::ModelSerializer;
pub use nested::{ListSerializer, NestedSerializer, WritableNestedSerializer};
pub use nested_config::{NestedFieldConfig, NestedSerializerConfig};
pub use nested_orm_integration::{
    ManyToManyManager, NestedSaveContext, NestedSerializerSave, TransactionHelper,
};
pub use performance::{
    BatchValidator, IntrospectionCache, N1Detector, PerformanceMetrics, PerformanceStats,
    QueryCache,
};
#[cfg(feature = "django-compat")]
pub use pool_manager::default_pool_config;
pub use pool_manager::ConnectionPoolManager;
pub use queryset_integration::{CacheAwareSaveContext, SaveContext, SerializerSaveMixin};
pub use recursive::{RecursiveError, RecursiveResult, SerializationContext};
pub use relation_fields_orm::{
    OptimizableRelationField, PrimaryKeyRelatedFieldORM, QueryOptimizer, SlugRelatedFieldORM,
};
pub use relations::{
    HyperlinkedRelatedField, IdentityField, ManyRelatedField, PrimaryKeyRelatedField,
    RelationField, SlugRelatedField, StringRelatedField,
};
pub use serializer::{Deserializer, JsonSerializer, Serializer, SerializerError, ValidatorError};
pub use validator::{
    validate_fields, FieldLevelValidation, FieldValidator, ObjectLevelValidation, ObjectValidator,
    ValidationError, ValidationResult,
};
pub use validator_config::ValidatorConfig;
pub use validators::{DatabaseValidatorError, UniqueTogetherValidator, UniqueValidator};
