//! Compatibility re-exports for the scoped route-loader store.

pub use super::loader::{
	LoaderStore, LoaderStoreError, LoaderStoreScope, active_loader_store, enter_loader_store,
	with_loader_store,
};
