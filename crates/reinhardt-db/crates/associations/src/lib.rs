//! Association proxies for Reinhardt
//!
//! This crate provides SQLAlchemy-style association proxies for simplifying
//! access to related objects through associations.
//!
//! ## Planned Features
//! TODO: Add automatic reverse relationship accessor generation
//! TODO: Add support for polymorphic associations

pub mod collection;
pub mod foreign_key;
pub mod loading;
pub mod many_to_many;
pub mod one_to_many;
pub mod one_to_one;
pub mod proxy;

pub use collection::AssociationCollection;
pub use foreign_key::{CascadeAction, ForeignKey};
pub use loading::{EagerLoader, JoinedLoader, LazyLoader, LoadingStrategy, SelectInLoader};
pub use many_to_many::ManyToMany;
pub use one_to_many::OneToMany;
pub use one_to_one::OneToOne;
pub use proxy::AssociationProxy;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::collection::*;
    pub use crate::foreign_key::*;
    pub use crate::loading::*;
    pub use crate::many_to_many::*;
    pub use crate::one_to_many::*;
    pub use crate::one_to_one::*;
    pub use crate::proxy::*;
}
