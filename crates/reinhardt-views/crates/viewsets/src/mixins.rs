use async_trait::async_trait;
use reinhardt_apps::{Request, Response, Result};

/// Mixin traits for ViewSet functionality
/// These use composition instead of multiple inheritance

/// List mixin - provides list() action
#[async_trait]
pub trait ListMixin: Send + Sync {
    async fn list(&self, request: Request) -> Result<Response>;
}

/// Retrieve mixin - provides retrieve() action
#[async_trait]
pub trait RetrieveMixin: Send + Sync {
    async fn retrieve(&self, request: Request, id: String) -> Result<Response>;
}

/// Create mixin - provides create() action
#[async_trait]
pub trait CreateMixin: Send + Sync {
    async fn create(&self, request: Request) -> Result<Response>;
}

/// Update mixin - provides update() action
#[async_trait]
pub trait UpdateMixin: Send + Sync {
    async fn update(&self, request: Request, id: String) -> Result<Response>;
}

/// Destroy mixin - provides destroy() action
#[async_trait]
pub trait DestroyMixin: Send + Sync {
    async fn destroy(&self, request: Request, id: String) -> Result<Response>;
}

/// Composite trait for all CRUD operations
/// This demonstrates trait composition in Rust
#[async_trait]
pub trait CrudMixin: ListMixin + RetrieveMixin + CreateMixin + UpdateMixin + DestroyMixin {}

// Blanket implementation for any type that implements all mixins
impl<T> CrudMixin for T where T: ListMixin + RetrieveMixin + CreateMixin + UpdateMixin + DestroyMixin
{}
