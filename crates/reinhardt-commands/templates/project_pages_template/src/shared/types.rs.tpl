//! Shared types used by both client and server
//!
//! These types are serializable and can be sent between the WASM client
//! and the Rust server via server functions.

use serde::{Deserialize, Serialize};

// Example shared type:
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct DataItem {
//     pub id: u64,
//     pub name: String,
// }
