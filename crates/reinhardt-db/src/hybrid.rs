//! Hybrid properties for Reinhardt
//!
//! This crate provides SQLAlchemy-style hybrid properties - properties that
//! work both at the instance level and at the SQL expression level.

pub mod comparator;
pub mod expression;
pub mod override_property;
pub mod property;

pub use comparator::{Comparator, UpperCaseComparator};
pub use expression::{Expression, SqlExpression};
pub use override_property::{HybridPropertyOverride, OverridableProperty};
pub use property::{HybridMethod, HybridProperty};

/// Re-export commonly used types
pub mod prelude {
	pub use super::comparator::*;
	pub use super::expression::*;
	pub use super::override_property::*;
	pub use super::property::*;
}

/// Macro for defining hybrid properties
#[macro_export]
macro_rules! hybrid_property {
	($name:ident, $getter:expr) => {
		pub fn $name(&self) -> impl Fn() -> String {
			$getter
		}
	};
}
