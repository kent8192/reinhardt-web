//! # reinhardt-taggit
//!
//! Simple tagging system for Reinhardt framework, inspired by django-taggit.
//!
//! ## Features
//!
//! - Tag and TaggedItem models
//! - TaggableManager for managing tags on model instances
//! - `#[taggable]` attribute macro for zero-boilerplate tagging
//! - Query extensions for tag-based filtering
//! - Tag normalization and validation
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use reinhardt_taggit::prelude::*;
//!
//! #[model(app_label = "myapp", table_name = "foods")]
//! #[taggable]
//! pub struct Food {
//!     #[field(primary_key = true)]
//!     pub id: i64,
//!
//!     #[field(max_length = 255)]
//!     pub name: String,
//! }
//!
//! # async fn example() -> Result<()> {
//! // Add tags to a food instance
//! let mut food = Food { id: 1, name: "Apple".to_string() };
//! food.tags().add(&["red", "fruit"]).await?;
//!
//! // Query foods by tags
//! let red_foods = Food::objects()
//!     .filter_by_tags(&["red"])
//!     .all()
//!     .await?;
//! # Ok(())
//! # }
//! ```

// Public modules
pub mod error;
pub mod models;
// pub mod config;
// pub mod manager;
// pub mod query;
// pub mod normalizer;
// pub mod validator;

// Re-exports for convenient access
pub use error::{Result, TaggitError};
pub use models::{Tag, TaggedItem};
// pub use models::traits::{Taggable, TagManagerTrait};
// pub use manager::TagManager;
// pub use normalizer::{DefaultNormalizer, Normalizer};
// pub use config::{TagConfig, TagConfigBuilder};

// Re-export proc macros
// pub use reinhardt_taggit_macros::taggable;

/// Prelude module for convenient imports
pub mod prelude {
	pub use crate::error::{Result, TaggitError};
	pub use crate::models::{Tag, TaggedItem};
	// pub use crate::models::traits::{Taggable, TagManagerTrait};
	// pub use crate::manager::TagManager;
	// pub use crate::config::TagConfig;
	// pub use reinhardt_taggit_macros::taggable;
}
