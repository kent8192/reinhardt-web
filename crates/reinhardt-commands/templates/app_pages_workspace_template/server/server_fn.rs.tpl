//! {{ app_name }} - Server functions
//!
//! Server functions that can be called from the WASM client.

// Example server function:
//
// use reinhardt::pages::server_fn;
// use std::sync::Arc;
//
// #[server_fn(use_inject = true)]
// pub async fn get_items(
//     #[inject] db: Arc<DatabaseConnection>,
// ) -> Result<Vec<DataItem>, String> {
//     // Implementation
//     todo!()
// }
