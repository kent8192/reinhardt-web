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

#### Dependency Injection
- **DiViewSet** - ViewSet wrapper with full DI support
  - Automatic dependency resolution via `Depends<V>`
  - Integration with reinhardt-di framework
- **ViewSetFactory Trait** - Factory pattern for ViewSet creation with DI
- **Injectable Dependencies** - Example implementations (DatabaseConnection)

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

