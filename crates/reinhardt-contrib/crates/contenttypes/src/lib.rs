//! # Reinhardt ContentTypes
//!
//! Django-style content types framework for polymorphic relationships.
//!
//! This crate provides a content type system that enables generic relations
//! similar to Django's contenttypes framework.
//!
//! ## Features
//!
//! - **Multi-database support**: Manage content types across multiple databases
//! - **ORM integration**: Seamless integration with reinhardt-orm
//! - **Generic relations**: Type-safe polymorphic relationships
//! - **Database persistence**: Store content types in database with caching

pub mod contenttypes;
pub mod generic_fk;
pub mod persistence;

#[cfg(feature = "database")]
pub mod multi_db;

#[cfg(feature = "database")]
pub mod orm_integration;

pub use contenttypes::{
	CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable,
	GenericRelationQuery, ModelType,
};

pub use generic_fk::GenericForeignKeyField;

#[cfg(feature = "database")]
pub use generic_fk::constraints;

#[cfg(feature = "database")]
pub use persistence::{
	ContentTypeModel, ContentTypePersistence, ContentTypePersistenceBackend, PersistenceError,
};

#[cfg(not(feature = "database"))]
pub use persistence::PersistenceError;

#[cfg(feature = "database")]
pub use multi_db::MultiDbContentTypeManager;

#[cfg(feature = "database")]
pub use orm_integration::{ContentTypeQuery, ContentTypeTransaction};
