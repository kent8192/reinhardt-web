//! # Reinhardt ContentTypes
//!
//! Django-style content types framework for polymorphic relationships.
//!
//! This crate provides a content type system that enables generic relations
//! similar to Django's contenttypes framework.

pub mod contenttypes;

pub use contenttypes::{
    ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable, GenericRelationQuery,
    ModelType, CONTENT_TYPE_REGISTRY,
};
