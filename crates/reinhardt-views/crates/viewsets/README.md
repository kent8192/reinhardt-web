# reinhardt-viewsets

Composable views for API endpoints

## Overview

ViewSets combine the logic for multiple related views into a single class. Provides ModelViewSet for CRUD operations, ReadOnlyModelViewSet, and custom ViewSet classes.

Automatically handles common patterns like list, retrieve, create, update, and delete operations.

## Features

### Implemented ✓

#### Core ViewSet Types

- **ViewSet Trait** - Base trait for all ViewSet implementations with dispatch, middleware support, and action routing
- **GenericViewSet** - Generic ViewSet implementation with composable handler pattern
- **ModelViewSet** - Full CRUD operations (list, retrieve, create, update, destroy) for model-based APIs
- **ReadOnlyModelViewSet** - Read-only operations (list, retrieve) for immutable resources

#### Action System

- **Action Types** - Comprehensive action type system supporting standard CRUD operations and custom actions
  - Standard actions: List, Retrieve, Create, Update, PartialUpdate, Destroy
  - Custom action support with configurable detail/list behavior
- **Action Metadata** - Rich metadata system for actions including:
  - Custom display names and suffixes
  - URL path and name configuration
  - HTTP method filtering
  - Action handler integration
- **Action Registry** - Global and local action registration systems
  - Manual registration API with `register_action()`
  - Macro-based registration with `register_viewset_actions!`
  - Inventory-based automatic collection of actions

#### Mixin System

- **ListMixin** - Provides list() action for querying collections
- **RetrieveMixin** - Provides retrieve() action for fetching single objects
- **CreateMixin** - Provides create() action for object creation
- **UpdateMixin** - Provides update() action for object modification
- **DestroyMixin** - Provides destroy() action for object deletion
- **CrudMixin** - Composite trait combining all CRUD operations

#### Middleware Support

- **ViewSetMiddleware Trait** - Middleware integration for cross-cutting concerns
  - `process_request()` - Pre-processing with early response capability
  - `process_response()` - Post-processing and response modification
- **AuthenticationMiddleware** - Login requirement enforcement
  - Configurable login_required behavior
  - Login URL redirection support
  - Session and header-based authentication detection
- **PermissionMiddleware** - Permission-based access control
  - Per-ViewSet permission requirements
  - Automatic 403 Forbidden responses for unauthorized access
- **CompositeMiddleware** - Middleware composition and chaining
  - Builder pattern for middleware configuration
  - Sequential middleware execution

#### Handler Integration

- **ViewSetHandler** - Converts ViewSets to Handlers for routing integration
  - HTTP method to action mapping
  - Path parameter extraction
  - Request attribute management (args, kwargs)
  - Middleware processing pipeline
- **ViewSetBuilder** - Fluent builder API for Handler creation
  - Action mapping configuration with `with_actions()` and `action()`
  - Custom name/suffix support (mutually exclusive)
  - Validation of action mappings
  - Macro support via `viewset_actions!`

#### Dependency Injection (FastAPI-Style)

- **Three DI Patterns** - Multiple ways to inject dependencies into ViewSets:
  1. **Field-Level Injection** - Use `#[derive(Injectable)]` with `#[inject]` attributes on struct fields
  2. **Method-Level Injection** - Use `#[endpoint]` with `#[inject]` attributes on method parameters
  3. **Dispatch-Level Injection** - Override `dispatch_with_context()` with `#[inject]` parameters
- **DiViewSet** - ViewSet wrapper with full DI support
  - Automatic dependency resolution via `Depends<V>`
  - Integration with reinhardt-di framework
- **ViewSetFactory Trait** - Factory pattern for ViewSet creation with DI
- **Injectable Dependencies** - Example implementations (DatabaseConnection)
- **Cache Control** - Fine-grained control with `#[inject(cache = false)]`
- **Backward Compatibility** - Non-DI ViewSets continue to work without changes

#### Testing Utilities

- **TestViewSet** - Configurable test ViewSet with middleware support
  - Configurable login_required behavior
  - Permission configuration
  - Middleware integration testing
- **SimpleViewSet** - Minimal ViewSet for basic testing scenarios

### Planned

#### Advanced Features

- **Pagination Integration** - Automatic pagination support for list actions
- **Filtering System** - Query parameter-based filtering for collections
- **Ordering Support** - Sortable collections with multiple field support
- **Bulk Operations** - Batch create/update/delete operations
- **Nested ViewSets** - Parent-child resource relationships
- **ViewSet Schema Generation** - OpenAPI schema generation from ViewSet definitions
- **Caching Support** - Response caching for read-only operations
- **Rate Limiting** - Per-ViewSet or per-action rate limiting
- **WebSocket ViewSets** - Real-time action support via WebSockets

## Usage Examples

### Pattern 1: Field-Level Dependency Injection

Inject dependencies when the ViewSet is instantiated:

```rust
use reinhardt_macros::Injectable;
use reinhardt_di::{Injectable, InjectionContext};

#[derive(Clone, Injectable)]
struct UserViewSet {
    #[inject]
    db: Database,
    #[inject]
    cache: RedisCache,
    name: String,  // Uses Default::default()
}

impl ViewSet for UserViewSet {
    fn get_basename(&self) -> &str {
        "users"
    }

    async fn dispatch(&self, request: Request, action: Action) -> Result<Response> {
        // Use self.db and self.cache
        let users = self.db.query_all().await?;
        Ok(Response::json(&users)?)
    }
}

// Usage
let ctx = InjectionContext::new(singleton);
let viewset = UserViewSet::inject(&ctx).await?;
```

### Pattern 2: Method-Level Dependency Injection

Inject dependencies into individual action methods:

```rust
use reinhardt_macros::endpoint;

impl ProductViewSet {
    #[endpoint]
    async fn list(&self, request: Request, #[inject] db: Database) -> Result<Response> {
        let products = db.query_all().await?;
        Ok(Response::json(&products)?)
    }

    #[endpoint]
    #[action(detail = true, methods = ["POST"])]
    async fn activate(
        &self,
        request: Request,
        #[inject] email: EmailService
    ) -> Result<Response> {
        email.send_activation().await?;
        Ok(Response::ok())
    }

    #[endpoint]
    async fn create(
        &self,
        request: Request,
        #[inject] db: Database,
        #[inject] logger: Logger,
    ) -> Result<Response> {
        logger.info("Creating product");
        let product = db.create(request.body()).await?;
        Ok(Response::created().json(&product)?)
    }
}
```

### Pattern 3: Dispatch-Level Dependency Injection

Inject dependencies at the dispatch level for centralized control:

```rust
use reinhardt_macros::endpoint;

impl ViewSet for OrderViewSet {
    fn supports_di(&self) -> bool {
        true
    }

    #[endpoint]
    async fn dispatch_with_context(
        &self,
        request: Request,
        action: Action,
        #[inject] db: Database,
        #[inject] logger: Logger,
    ) -> Result<Response> {
        logger.log(&format!("Dispatching action: {:?}", action));

        match action.action_type {
            ActionType::List => self.handle_list(request, db).await,
            ActionType::Retrieve => self.handle_retrieve(request, db).await,
            _ => Err(Error::NotFound("Action not found".to_string()))
        }
    }
}
```

### Cache Control

Disable caching for specific dependencies:

```rust
#[derive(Clone, Injectable)]
struct MyService {
    #[inject]
    db: Database,              // Cached (default)
    #[inject(cache = false)]
    fresh_data: FreshData,     // Not cached
}

// Or in methods:
#[endpoint]
async fn handler(
    &self,
    request: Request,
    #[inject] cached: Database,
    #[inject(cache = false)] fresh: Database,
) -> Result<Response> {
    // ...
}
```

### Configuring ViewSetHandler with DI

```rust
use reinhardt_di::{InjectionContext, SingletonScope};
use std::sync::Arc;

// Create DI context
let singleton = Arc::new(SingletonScope::new());
let ctx = Arc::new(InjectionContext::new(singleton));

// Create ViewSet handler with DI
let handler = ViewSetHandler::new(
    Arc::new(viewset),
    action_map,
    None,
    None,
).with_di_context(ctx);

// Now the handler will automatically inject dependencies
```
