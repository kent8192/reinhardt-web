# Router API Reference

Comprehensive API reference for the Reinhardt routing system.

## Table of Contents

- [ServerRouter](#serverrouter)
  - [Performance Characteristics](#performance-characteristics)
  - [Constructors](#constructors)
  - [Router Configuration](#router-configuration)
  - [Route Registration](#route-registration)
  - [Hierarchical Routing](#hierarchical-routing)
  - [URL Reversal](#url-reversal)
  - [Route Information](#route-information)
- [Routing Patterns](#routing-patterns)
- [Best Practices](#best-practices)

---

## ServerRouter

Unified router with hierarchical routing support.

### Performance Characteristics

`` `ServerRouter` `` uses [matchit](https://docs.rs/matchit) Radix Tree for **O(m)** route matching (m = path length):

- Route lookup: **O(m)** - Independent of number of registered routes
- Route compilation: **O(n)** - Done once at startup (n = route count)
- Memory: Efficient through Radix Tree prefix sharing

With 1000+ routes, provides 3-5x better performance than naive O(n×m) linear search.

### Constructors

#### `ServerRouter::new()`

Creates a new router.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new();
assert_eq!(router.prefix(), "");
assert_eq!(router.namespace(), None);
```

### Router Configuration

#### `.with_prefix(prefix)`

Sets the router's prefix.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new()
    .with_prefix("/api/v1");

assert_eq!(router.prefix(), "/api/v1");
```

#### `.with_namespace(namespace)`

Sets the router's namespace.

Used for URL reversal.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new()
    .with_namespace("v1");

assert_eq!(router.namespace(), Some("v1"));
```

#### `.with_di_context(ctx)`

Sets the DI context for the router.

```rust
use reinhardt_urls::routers::ServerRouter;
use reinhardt_di::{InjectionContext, SingletonScope};
use std::sync::Arc;

let singleton_scope = Arc::new(SingletonScope::new());
let di_ctx = Arc::new(InjectionContext::builder(singleton_scope).build());
let router = ServerRouter::new()
    .with_di_context(di_ctx);
```

#### `.with_middleware(mw)`

Adds middleware to the router.

```rust
use reinhardt_urls::routers::ServerRouter;
use reinhardt_middleware::LoggingMiddleware;

let router = ServerRouter::new()
    .with_middleware(LoggingMiddleware::new());
```

### Route Registration

#### `.function(path, method, func)`

Registers a function-based route (FastAPI-style).

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn health_check(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let router = ServerRouter::new()
    .function("/health", Method::GET, health_check);
```

#### `.function_named(path, method, name, func)`

Registers a named function-based route (supports URL reversal).

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn health_check(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let mut router = ServerRouter::new()
    .with_namespace("api")
    .function_named("/health", Method::GET, "health", health_check);

router.register_all_routes();
let url = router.reverse("api:health", &[]).unwrap();
assert_eq!(url, "/health");
```

#### `.route(path, method, func)`

Alias for `` `.function()` ``.

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn health_check(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let router = ServerRouter::new()
    .route("/health", Method::GET, health_check);
```

#### `.route_named(path, method, name, func)`

Alias for `` `.function_named()` ``.

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn health_check(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let mut router = ServerRouter::new()
    .with_namespace("api")
    .route_named("/health", Method::GET, "health", health_check);
```

#### `.handler_with_method(path, method, handler)`

Registers a type implementing the `` `Handler` `` trait.

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;
use reinhardt_http::{Handler, Request, Response};
use async_trait::async_trait;

# #[derive(Clone)]
# struct ArticleHandler;
# #[async_trait]
# impl Handler for ArticleHandler {
#     async fn handle(&self, _request: Request) -> Result<Response, reinhardt_http::Error> {
#         Ok(Response::ok())
#     }
# }
let router = ServerRouter::new()
    .handler_with_method("/articles", Method::GET, ArticleHandler);
```

#### `.handler_with_method_named(path, method, name, handler)`

Registers a named Handler (supports URL reversal).

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# #[derive(Clone)]
# struct ArticleHandler;
# impl reinhardt_http::Handler for ArticleHandler {
#     async fn handle(&self, _req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#         Ok(reinhardt_http::Response::ok())
#     }
# }
let mut router = ServerRouter::new()
    .with_namespace("api")
    .handler_with_method_named("/articles", Method::GET, "list_articles", ArticleHandler);
```

#### `.view(path, view)`

Registers a class-based view (Django-style).

```rust
use reinhardt_urls::routers::ServerRouter;

# struct ArticleListView;
# impl reinhardt_http::Handler for ArticleListView {
#     async fn handle(&self, _req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#         Ok(reinhardt_http::Response::ok())
#     }
# }
let router = ServerRouter::new()
    .view("/articles", ArticleListView);
```

#### `.view_named(path, name, view)`

Registers a named class-based view (supports URL reversal).

```rust
use reinhardt_urls::routers::ServerRouter;

# struct ArticleListView;
# impl reinhardt_http::Handler for ArticleListView {
#     async fn handle(&self, _req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#         Ok(reinhardt_http::Response::ok())
#     }
# }
let mut router = ServerRouter::new()
    .with_namespace("articles")
    .view_named("/articles", "list", ArticleListView);
```

#### `.viewset(prefix, viewset)`

Registers a ViewSet (DRF-style).

Automatically generates CRUD routes:
- `` GET /prefix/`` - list
- `` POST /prefix/`` - create
- `` GET /prefix/{id}/`` - retrieve
- `` PUT /prefix/{id}/`` - update
- `` DELETE /prefix/{id}/`` - destroy

```rust
use reinhardt_urls::routers::ServerRouter;

# struct UserViewSet;
# impl reinhardt_views::viewsets::ViewSet for UserViewSet {
#     fn get_basename(&self) -> &str { "users" }
#     async fn dispatch(&self, _req: reinhardt_http::Request, _action: reinhardt_views::viewsets::Action)
#         -> reinhardt_core::exception::Result<reinhardt_http::Response> {
#         Ok(reinhardt_http::Response::ok())
#     }
# }
let router = ServerRouter::new()
    .viewset("/users", UserViewSet);
```

#### `.endpoint(factory)`

Registers an endpoint using `` `EndpointInfo` `` trait.

Path, HTTP method, and name are automatically extracted from the `` `EndpointInfo` `` implementation.

```rust
use reinhardt_urls::routers::ServerRouter;

# struct ListUsers;
# impl reinhardt_core::endpoint::EndpointInfo for ListUsers {
#     fn path() -> &'static str { "/users" }
#     fn method() -> hyper::Method { hyper::Method::GET }
#     fn name() -> &'static str { "list_users" }
# }
# impl reinhardt_http::Handler for ListUsers {
#     async fn handle(&self, _req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#         Ok(reinhardt_http::Response::ok())
#     }
# }
# fn list_users() -> ListUsers { ListUsers }
let router = ServerRouter::new()
    .endpoint(list_users); // Pass function directly (no () needed)
```

### Hierarchical Routing

#### `.mount(prefix, child_router)`

Mounts a child router at the given prefix.

**Important**: Prefix must end with `` `/` `` (Django URL convention).
Empty string `` ""`` is prohibited. Use `` "/" `` instead.

```rust
use reinhardt_urls::routers::ServerRouter;

let users_router = ServerRouter::new()
    .with_namespace("users");

let router = ServerRouter::new()
    .with_prefix("/api")
    .mount("/users/", users_router); // Trailing / required
```

**Panics** if prefix is empty or doesn't end with `` `/` ``.

#### `.group(routers)`

Adds multiple child routers at once.

```rust
use reinhardt_urls::routers::ServerRouter;

let users = ServerRouter::new().with_prefix("/users");
let posts = ServerRouter::new().with_prefix("/posts");

let router = ServerRouter::new()
    .group(vec![users, posts]);
```

#### `.with_route_middleware(mw)`

Adds middleware to the last registered route.

```rust
use reinhardt_urls::routers::ServerRouter;
use reinhardt_middleware::LoggingMiddleware;
use hyper::Method;

# async fn health(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let router = ServerRouter::new()
    .function("/health", Method::GET, health)
    .with_route_middleware(LoggingMiddleware::new());
```

### URL Reversal

#### `.register_all_routes()`

Registers all routes with the URL reverser.

Enables URL reversal with route namespaces (namespace:name format).

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn dummy_handler(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let mut router = ServerRouter::new()
    .with_namespace("api");

// Register routes
router = router.function_named("/health", Method::GET, "health", dummy_handler);

// Register with URL reverser
router.register_all_routes();
```

#### `.reverse(name, params)`

Generates URL from route name.

Supports hierarchical namespace notation (e.g., `` "v1:users:detail"``).

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn dummy_handler(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let mut router = ServerRouter::new()
    .with_namespace("v1")
    .function_named("/users/{id}", Method::GET, "user_detail", dummy_handler);

router.register_all_routes();

// Reverse with parameters
let url = router.reverse("v1:user_detail", &[("id", "123")]).unwrap();
assert_eq!(url, "/users/123");
```

### Route Information

#### `.get_all_routes()`

Gets all routes from this router and its children.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new()
    .with_prefix("/api/v1")
    .with_namespace("api");

let routes = router.get_all_routes();
// Returns: Vec<(full_path, name, namespace, methods)>
```

#### `.prefix()`

Gets this router's prefix.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new().with_prefix("/api/v1");
assert_eq!(router.prefix(), "/api/v1");
```

#### `.namespace()`

Gets this router's namespace.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new().with_namespace("v1");
assert_eq!(router.namespace(), Some("v1"));
```

#### `.children_count()`

Gets the number of child routers.

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new()
    .mount("/users/", ServerRouter::new())
    .mount("/posts/", ServerRouter::new());

assert_eq!(router.children_count(), 2);
```

---

## Routing Patterns

### Path Parameters

Use matchit format `` `{name}` `` syntax.

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn get_user(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let router = ServerRouter::new()
    .function("/users/{id}", Method::GET, get_user);
```

### Wildcard Routes

Use `` `{*path}` `` syntax for wildcard routes.

```rust
use reinhardt_urls::routers::ServerRouter;
use hyper::Method;

# async fn catch_all(_req: reinhardt_http::Request) -> reinhardt_http::Result<reinhardt_http::Response> {
#     Ok(reinhardt_http::Response::ok())
# }
let router = ServerRouter::new()
    .function("/{*path}", Method::GET, catch_all);
```

### Django-style Trailing Slashes

Reinhardt flexibly handles trailing slashes like Django:

`` `/users`` → tries matching `` `/users/` `
`` `/users/`` → tries matching `` `/users``

---

## Best Practices

### Namespace Hierarchies

Parent-child relationships concatenate namespaces:

```rust
use reinhardt_urls::routers::ServerRouter;

let users = ServerRouter::new().with_namespace("users");
let api = ServerRouter::new()
    .with_namespace("v1")
    .mount("/users/", users);

// Route names are in "v1:users:action" format
```

### Prefix Naming Convention

Always end `` `.mount()` `` prefixes with `` `/` ``:

- ✅ `` "/users/` `
- ✅ `` "/api/v1/` `
- ❌ `` "/users"``
- ❌ `` ""`` (use `` "/" `` instead)

### Middleware Order

Middleware is applied in parent → child order:

```rust
use reinhardt_urls::routers::ServerRouter;

let router = ServerRouter::new()
    .with_middleware(ParentMiddleware) // Applied first
    .mount("/child/", ServerRouter::new()
        .with_middleware(ChildMiddleware)); // Applied second
```

---

## See Also

- [Request API](./request.en.md)
- [Response API](./response.en.md)
- [Middleware Creation](../cookbook/middleware-creation.en.md)
