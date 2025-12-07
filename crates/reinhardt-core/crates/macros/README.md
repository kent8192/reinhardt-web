# reinhardt-macros

Procedural macros for the framework

## Overview

Procedural macros for reducing boilerplate code. Includes derive macros for models, serializers, and forms, as well as attribute macros for endpoints and middleware.

Provides compile-time code generation for common patterns.

## Features

### Implemented ✓

#### Function-based API Views

- **`#[api_view]`** - Convert function to API view
  - Supports multiple HTTP methods via `methods` parameter
  - Validates HTTP methods at compile time (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)
  - Defaults to GET if no methods specified
  - Example: `#[api_view(methods = "GET,POST")]`

#### ViewSet Custom Actions

- **`#[action]`** - Define custom ViewSet actions
  - Supports HTTP method specification via `methods` parameter
  - Supports detail/list action via `detail` parameter (required)
  - Optional `url_path` and `url_name` parameters
  - Validates HTTP methods at compile time
  - Example: `#[action(methods = "POST", detail = true)]`

#### HTTP Method Decorators

- **`#[get]`** - GET method decorator with path validation
- **`#[post]`** - POST method decorator with path validation
- **`#[put]`** - PUT method decorator with path validation
- **`#[patch]`** - PATCH method decorator with path validation
- **`#[delete]`** - DELETE method decorator with path validation
- All support compile-time URL pattern validation
- **Dependency Injection**: Use `use_inject = true` option with `#[inject]` parameter attribute
- Path syntax: Simple `{id}` or typed `{<uuid:id>}`, `{<int:id>}`, `{<str:name>}`, `{<slug:title>}`, `{<path:route>}`
- Example: `#[get("/users/{id}")]`
- Example with DI: `#[get("/users/{<uuid:id>}", use_inject = true)]`

#### Permission System

- **`#[permission_required]`** - Permission decorator
  - Validates permission strings at compile time
  - Supports Django-style permission format: `"app.permission"`
  - Uses nom parser for validation
  - Example: `#[permission_required("users.view_user")]`

#### Dependency Injection (FastAPI-style)

##### Injectable Macro (Factory/Provider Pattern)

- **`#[injectable]`** - Transform functions or structs into `Injectable` trait implementations

  **Function Usage**: Factory functions for creating dependencies
  - All parameters must be marked with `#[inject]`
  - Supports both sync and async functions
  - Cache control: `#[inject(cache = false)]`
  - Scope control: `#[inject(scope = Singleton)]` or `#[inject(scope = Request)]`
  - Example:
    ```rust
    #[injectable]
    fn create_user_service(
        #[inject] db: Arc<Database>,
        #[inject] cache: Arc<Cache>,
    ) -> UserService {
        UserService { db, cache }
    }
    ```

  **Struct Usage**: Auto-generate `Injectable` implementation
  - All fields must have either `#[inject]` or `#[no_inject]` attribute
  - Field attributes:
    - `#[inject]` - Inject from DI container (cached by default)
    - `#[inject(cache = false)]` - Inject without caching
    - `#[inject(scope = Singleton)]` - Use singleton scope
    - `#[no_inject(default = Default)]` - Initialize with `Default::default()`
    - `#[no_inject(default = value)]` - Initialize with specific value
    - `#[no_inject]` - Initialize with `None` (field must be `Option<T>`)
  - Struct must implement `Clone` (required by `Injectable` trait)
  - Example:
    ```rust
    #[injectable]
    #[derive(Clone)]
    struct UserViewSet {
        #[inject]
        db: Database,
        #[inject]
        cache: RedisCache,
        #[no_inject(default = Default)]
        config: Config,
    }
    ```

##### HTTP Method Macros with Dependency Injection

- **`#[get("/path", use_inject = true)]`** - GET with DI enabled
- **`#[post("/path", use_inject = true)]`** - POST with DI enabled
- **`#[put("/path", use_inject = true)]`** - PUT with DI enabled
- **`#[patch("/path", use_inject = true)]`** - PATCH with DI enabled
- **`#[delete("/path", use_inject = true)]`** - DELETE with DI enabled
  - FastAPI-style parameter attributes with `#[inject]`
  - Automatic resolution from `InjectionContext`
  - `use_inject = true` is **required** when using `#[inject]` parameters
  - Example:
    ```rust
    #[get("/users/{<uuid:id>}", use_inject = true)]
    async fn get_user(
        Path(id): Path<Uuid>,
        #[inject] db: Arc<DatabaseConnection>,  // Injected from context
    ) -> ViewResult<Response> {
        // ...
    }

    #[post("/users", use_inject = true)]
    async fn create_user(
        Json(data): Json<CreateUserRequest>,
        #[inject] db: Arc<DatabaseConnection>,
    ) -> ViewResult<Response> {
        // ...
    }
    ```

**Pattern Comparison:**
- `#[injectable]` - Creates an `Injectable` implementation for the return type (Factory/Provider pattern)
- `#[<http_method>(..., use_inject = true)]` - Injects dependencies into function parameters

#### Configuration Macros

- **`installed_apps!`** - Define installed applications
  - Compile-time validation of application paths
  - Type-safe enum generation for all installed apps
  - Validates `reinhardt.contrib.*` modules exist
  - Generates `Display` and `FromStr` implementations
  - Example: `installed_apps! { auth: "reinhardt.contrib.auth", }`

#### URL Pattern Validation

- **`path!`** - Validate URL patterns at compile time
  - Uses nom parser for pattern validation
  - Supports simple parameters: `{id}`
  - Supports Django-style typed parameters: `{<int:id>}`
  - Validates parameter names and type specifiers
  - Supported types: `int`, `str`, `uuid`, `slug`, `path`
  - Detailed error messages with position information
  - Example: `path!("users/{<int:user_id>}/posts/{post_id}/")`

#### Signal System

- **`#[receiver]`** - Connect receiver function to signal
  - Django-style `@receiver` decorator functionality
  - Supports signal and sender parameters
  - Marker macro for signal registration
  - Example: `#[receiver(signal = post_save::<User>())]`

#### Type-safe Query Fields

- **`#[derive(QueryFields)]`** - Generate field accessor methods
  - Automatic field accessor generation for models
  - Compile-time validated field lookups
  - Type-specific lookup methods based on field type
  - String fields: `lower()`, `upper()`, `trim()`, `contains()`
  - Numeric fields: `abs()`, `ceil()`, `floor()`, `round()`
  - DateTime fields: `year()`, `month()`, `day()`, `hour()`
  - All fields: `eq()`, `ne()`, `gt()`, `gte()`, `lt()`, `lte()`
  - Example: `QuerySet::<User>::new().filter(User::email().lower().contains("example.com"))`