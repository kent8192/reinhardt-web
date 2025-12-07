//! Relationship views module
//!
//! Re-exports all relationship view handlers

pub mod block;
pub mod follow;
pub mod list;

// Re-export view functions for convenience
pub use self::block::{block_user, unblock_user};
pub use self::follow::{follow_user, unfollow_user};
pub use self::list::{fetch_blockings, fetch_followers, fetch_followings};
