//! # Reinhardt ContentTypes
//!
//! Django-style content types framework for polymorphic relationships.
//!
//! This crate provides a content type system that enables generic relations
//! similar to Django's contenttypes framework.
//!
//! ## Planned Features
//! TODO: Implement automatic ContentType table creation and migration
//! TODO: Add persistence of ContentType instances to database
//! TODO: Implement database-backed content type lookups
//! TODO: Add multi-database support for content types
//! TODO: Implement foreign key constraints for GenericForeignKey fields
//! TODO: Complete ORM integration

pub mod contenttypes;

pub use contenttypes::{
    CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable,
    GenericRelationQuery, ModelType,
};
