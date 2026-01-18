//! Association proxies for Reinhardt
//!
//! This crate provides SQLAlchemy-style association proxies for simplifying
//! access to related objects through associations.
//!
//! # Features
//!
//! ## Relationship Types
//!
//! - **ForeignKey**: One-to-many and many-to-one relationships
//! - **OneToOne**: Unique one-to-one relationships
//! - **OneToMany**: Reverse side of ForeignKey relationships
//! - **ManyToMany**: Many-to-many relationships through junction tables
//! - **PolymorphicAssociation**: Polymorphic one-to-many relationships
//! - **PolymorphicManyToMany**: Polymorphic many-to-many relationships
//!
//! ## Automatic Reverse Relationship Accessors
//!
//! All relationship types support automatic generation of reverse accessor names
//! when `related_name` is not explicitly provided:
//!
//! ```
//! use reinhardt_db::associations::{ForeignKey, ReverseRelationship};
//!
//! #[derive(Clone)]
//! struct User {
//!     id: i64,
//! }
//!
//! let fk: ForeignKey<User, i64> = ForeignKey::new("author_id");
//! // Automatically generates "post_set" as the reverse accessor
//! assert_eq!(fk.get_or_generate_reverse_name("Post"), "post_set");
//! ```
//!
//! ## Polymorphic Associations
//!
//! Support for polymorphic associations allows a model to belong to multiple
//! different model types through a single association:
//!
//! ```
//! use reinhardt_db::associations::PolymorphicAssociation;
//!
//! #[derive(Clone)]
//! struct Comment {
//!     id: i64,
//!     commentable_id: i64,
//!     commentable_type: String,
//! }
//!
//! // A comment can belong to either a Post or a Video
//! let rel: PolymorphicAssociation<i64> = PolymorphicAssociation::new("commentable");
//! ```
//!
//! ## Loading Strategies
//!
//! Multiple loading strategies are available to optimize database queries:
//!
//! - **Lazy**: Load related objects only when accessed
//! - **Eager**: Load related objects immediately with the parent
//! - **SelectIn**: Use SELECT IN strategy for multiple objects
//! - **Joined**: Use SQL JOIN for single query loading
//! - **Subquery**: Use subquery for complex filtering

pub mod collection;
pub mod foreign_key;
pub mod loading;
pub mod many_to_many;
pub mod many_to_many_manager;
pub mod markers;
pub mod one_to_many;
pub mod one_to_one;
pub mod polymorphic;
pub mod proxy;
pub mod reverse;

pub use collection::AssociationCollection;
pub use foreign_key::{CascadeAction, ForeignKey};
pub use loading::{EagerLoader, JoinedLoader, LazyLoader, LoadingStrategy, SelectInLoader};
pub use many_to_many::ManyToMany;
pub use many_to_many_manager::ManyToManyManager;
pub use markers::{
	ForeignKeyField, ManyToManyField, OneToManyField, OneToOneField, PolymorphicManyToManyField,
};
pub use one_to_many::OneToMany;
pub use one_to_one::OneToOne;
pub use polymorphic::{PolymorphicAssociation, PolymorphicManyToMany};
pub use proxy::AssociationProxy;
pub use reverse::{
	ReverseRelationship, generate_reverse_accessor, generate_reverse_accessor_singular,
	to_snake_case,
};

/// Re-export commonly used types
pub mod prelude {
	pub use crate::collection::*;
	pub use crate::foreign_key::*;
	pub use crate::loading::*;
	pub use crate::many_to_many::*;
	pub use crate::many_to_many_manager::*;
	pub use crate::markers::*;
	pub use crate::one_to_many::*;
	pub use crate::one_to_one::*;
	pub use crate::polymorphic::*;
	pub use crate::proxy::*;
	pub use crate::reverse::*;
}
