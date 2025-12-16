# reinhardt-panel

Django-style admin panel for Reinhardt framework.

## Overview

`reinhardt-panel` provides a comprehensive admin interface for managing database models through a web-based CRUD interface. It offers automatic model discovery, advanced filtering, bulk operations, and import/export capabilities.

## Features

- **Automatic CRUD Operations**: List, detail, create, update, delete
- **Advanced Filtering**: Complex queries with multiple operators
- **Pagination & Search**: Efficient data browsing
- **Bulk Operations**: Bulk delete, bulk edit
- **Import/Export**: CSV, JSON, XML formats
- **Inline Editing**: Edit related models inline
- **Drag-and-Drop Reordering**: Visual list reordering
- **Custom Views**: Add custom admin views
- **Permission Integration**: Fine-grained access control
- **Responsive UI**: Mobile-friendly interface

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-panel = { version = "0.1", features = ["full"] }
```

### Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | Core admin functionality |
| `templates` | Template rendering (Jinja2-style) |
| `file-uploads` | File upload handling |
| `full` | All features enabled |

## Quick Start

### Basic Setup

```rust
use reinhardt_panel::{AdminRouter, AdminSite};
use reinhardt_db::DatabaseManager;
use std::sync::Arc;

// Create admin site
let site = Arc::new(AdminSite::new("My Admin"));

// Create database manager
let db = Arc::new(DatabaseManager::new(pool));

// Create router
let router = AdminRouter::new(site.clone(), db.clone())
    .with_favicon("path/to/favicon.ico")
    .build();

// Register model routes
router.register_model_routes::<User>("/admin/user/")?;
router.register_model_routes::<Post>("/admin/post/")?;
```

### Model Registration

```rust
use reinhardt_db::Model;
use reinhardt_panel::ModelAdmin;

#[derive(Model)]
#[table_name = "users"]
struct User {
    id: i32,
    username: String,
    email: String,
    is_active: bool,
}

// Custom ModelAdmin configuration
impl ModelAdmin for User {
    fn list_display() -> Vec<&'static str> {
        vec!["id", "username", "email", "is_active"]
    }

    fn search_fields() -> Vec<&'static str> {
        vec!["username", "email"]
    }

    fn list_filter() -> Vec<&'static str> {
        vec!["is_active"]
    }
}
```

## Database Layer

### Filtering System

The admin panel provides a powerful filtering system that translates to SeaQuery expressions:

```rust
use reinhardt_panel::database::{Filter, FilterOperator, FilterCondition, FilterValue};

// Single filter
let filter = Filter {
    field: "username".to_string(),
    operator: FilterOperator::Contains,
    value: FilterValue::String("john".to_string()),
};

// Multiple filters with AND condition
let condition = FilterCondition::And(vec![
    FilterCondition::Single(Filter {
        field: "is_active".to_string(),
        operator: FilterOperator::Eq,
        value: FilterValue::Bool(true),
    }),
    FilterCondition::Single(Filter {
        field: "age".to_string(),
        operator: FilterOperator::Gte,
        value: FilterValue::Int(18),
    }),
]);
```

### Filter Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `Eq` | Equals | `age = 25` |
| `Ne` | Not equals | `status != "inactive"` |
| `Gt` | Greater than | `price > 100` |
| `Gte` | Greater than or equal | `age >= 18` |
| `Lt` | Less than | `stock < 10` |
| `Lte` | Less than or equal | `discount <= 50` |
| `Contains` | Contains substring | `name LIKE "%john%"` |
| `StartsWith` | Starts with | `email LIKE "admin%"` |
| `EndsWith` | Ends with | `domain LIKE "%.com"` |
| `In` | In list | `status IN ("active", "pending")` |
| `NotIn` | Not in list | `role NOT IN ("guest")` |
| `Between` | Between two values | `age BETWEEN 18 AND 65` |
| `Regex` | Regex match | `phone REGEXP "^\\+1"` |

### SeaQuery Integration

Filters are automatically converted to `sea_query::SimpleExpr`:

```rust
use sea_query::{Query, Expr};
use reinhardt_panel::database::FilterExt;

let filter = Filter {
    field: "email".to_string(),
    operator: FilterOperator::EndsWith,
    value: FilterValue::String("@example.com".to_string()),
};

// Converts to: WHERE email LIKE '%@example.com'
let expr = filter.to_sea_query_expr()?;
```

## Authentication & Authorization

### Permission Checking

```rust
use reinhardt_panel::auth::{AdminPermission, PermissionChecker};

// Check if user has permission
let has_permission = site.check_permission(
    &user,
    AdminPermission::View("user"),
)?;

// Model-level permissions
impl ModelAdmin for User {
    fn has_view_permission(&self, user: &User) -> bool {
        user.is_staff
    }

    fn has_change_permission(&self, user: &User) -> bool {
        user.is_superuser
    }

    fn has_delete_permission(&self, user: &User) -> bool {
        user.is_superuser
    }
}
```

## Handlers

### AdminHandlers

The `AdminHandlers` struct provides HTTP request handlers for all CRUD operations:

```rust
use reinhardt_panel::handlers::AdminHandlers;

let handlers = AdminHandlers::new(site.clone(), db.clone())
    .with_favicon("static/favicon.ico");

// Available handler methods:
// - dashboard() -> DashboardResponse
// - list(model_name, params) -> ListResponse
// - detail(model_name, id) -> DetailResponse
// - create(model_name, data) -> MutationResponse
// - update(model_name, id, data) -> MutationResponse
// - delete(model_name, id) -> MutationResponse
// - bulk_delete(model_name, ids) -> BulkDeleteResponse
// - export(model_name, format) -> Binary
// - import(model_name, file) -> ImportResponse
```

### Endpoints

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/admin/` | `dashboard()` | Admin dashboard |
| GET | `/admin/favicon.ico` | `favicon()` | Favicon |
| GET | `/admin/<model>/` | `list()` | Model list view |
| GET | `/admin/<model>/<id>/` | `detail()` | Detail view |
| POST | `/admin/<model>/` | `create()` | Create new instance |
| PUT | `/admin/<model>/<id>/` | `update()` | Update instance |
| DELETE | `/admin/<model>/<id>/` | `delete()` | Delete instance |
| DELETE | `/admin/<model>/bulk/` | `bulk_delete()` | Bulk delete |
| GET | `/admin/<model>/export/` | `export()` | Export data |
| POST | `/admin/<model>/import/` | `import()` | Import data |

## Routing

### AdminRouter

The `AdminRouter` struct manages route registration:

```rust
use reinhardt_panel::router::AdminRouter;

let router = AdminRouter::new(site, db)
    .with_favicon("static/favicon.ico")
    .build();

// Register routes for a model
router.register_model_routes::<User>("/admin/user/")?;

// Routes are automatically created:
// GET    /admin/user/
// GET    /admin/user/:id/
// POST   /admin/user/
// PUT    /admin/user/:id/
// DELETE /admin/user/:id/
// DELETE /admin/user/bulk/
// GET    /admin/user/export/
// POST   /admin/user/import/
```

## Custom Views

### Adding Custom Admin Views

```rust
use reinhardt_panel::custom_views::{CustomView, CustomViewResponse};

struct UserStatsView;

impl CustomView for UserStatsView {
    fn name(&self) -> &str {
        "user-stats"
    }

    fn url_path(&self) -> &str {
        "/admin/stats/users/"
    }

    async fn handle(&self, ctx: &ViewContext) -> CustomViewResponse {
        // Custom logic here
        CustomViewResponse::Json(json!({
            "total_users": 1000,
            "active_users": 850,
        }))
    }
}

// Register custom view
site.register_custom_view(Box::new(UserStatsView))?;
```

## Import/Export

### Export Formats

```rust
use reinhardt_panel::handlers::ExportFormat;

// Export as CSV
let csv_data = handlers.export("user", ExportFormat::Csv).await?;

// Export as JSON
let json_data = handlers.export("user", ExportFormat::Json).await?;

// Export as XML
let xml_data = handlers.export("user", ExportFormat::Xml).await?;
```

### Import Data

```rust
use reinhardt_panel::handlers::ImportResponse;

// Import from CSV file
let response = handlers.import("user", csv_file_data).await?;

println!("Imported: {}", response.imported);
println!("Updated: {}", response.updated);
println!("Failed: {}", response.failed);
```

## Bulk Operations

### Bulk Delete

```rust
use reinhardt_panel::handlers::BulkDeleteRequest;

let request = BulkDeleteRequest {
    ids: vec![1, 2, 3, 4, 5],
};

let response = handlers.bulk_delete("user", request).await?;
println!("Deleted {} records", response.deleted);
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     AdminSite                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   models     │  │   custom     │  │ permissions  │  │
│  │   registry   │  │    views     │  │              │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
   │   Handlers  │ │   Router    │ │  Database   │
   │             │ │             │ │   Layer     │
   └─────────────┘ └─────────────┘ └─────────────┘
          │               │               │
          └───────────────┴───────────────┘
                          ▼
                   ┌─────────────┐
                   │  SeaQuery   │
                   │ Integration │
                   └─────────────┘
```

## Related Crates

- `reinhardt-admin`: Parent crate with CLI integration
- `reinhardt-db`: Database and ORM integration
- `reinhardt-auth`: Authentication and permissions
- `reinhardt-urls`: Routing system
- `reinhardt-template`: Template rendering

## License

Licensed under the MIT license.
