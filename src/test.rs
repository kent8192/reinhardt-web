//! Testing utilities module.
//!
//! This module provides testing utilities and test client.
//!
//! # Examples
//!
//! ```rust,ignore
//! use reinhardt::test::{TestClient, TestCase};
//! ```

#[cfg(feature = "test")]
pub use reinhardt_test::*;

/// Pages component testing utilities.
#[cfg(all(native, feature = "test", feature = "pages"))]
pub mod pages {
	pub use reinhardt_pages::testing::component::*;
}
