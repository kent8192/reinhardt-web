# reinhardt-admin

Django-style admin panel functionality for Reinhardt framework.

## Overview

This crate provides two main components:

- **Panel**: Web-based admin interface for managing database models
- **CLI**: Command-line tool for project management (available as
  `reinhardt-admin-cli`)

## Features

### Admin Panel (`reinhardt-panel`)

- ✅ **Model Management Interface**: Web-based CRUD operations for database
  models
- ✅ **Automatic Admin Discovery**: Auto-generate admin interfaces from model
  definitions
- ✅ **Customizable Admin Actions**: Bulk operations and custom actions
- ✅ **Search and Filtering**: Advanced search capabilities with multiple filter
  types
- ✅ **Permissions Integration**: Role-based access control for admin operations
- ✅ **Change Logging**: Audit trail for all admin actions
- ✅ **Inline Editing**: Edit related models inline
- ✅ **Drag-and-Drop Reordering**: Reorder model instances with transaction-safe
  operations
- ✅ **Responsive Design**: Mobile-friendly admin interface with customizable
  templates

### Command-Line Interface (`reinhardt-admin-cli`)

For project management commands (`startproject`, `startapp`), please use
[`reinhardt-admin-cli`](../reinhardt-admin-cli).

## Quick Start

### Using the Admin Panel

```rust
use reinhardt_panel::{AdminSite, ModelAdmin};

#[tokio::main]
async fn main() {
    let mut admin = AdminSite::new("My Admin");

    // Register your models
    admin.register::<User>(UserAdmin::default()).await;

    // Start admin server
    admin.serve("127.0.0.1:8001").await.unwrap();
}
```

### Customizing the Admin

```rust
use reinhardt_panel::ModelAdmin;

struct UserAdmin {
    list_display: Vec<String>,
    list_filter: Vec<String>,
    search_fields: Vec<String>,
}

impl Default for UserAdmin {
    fn default() -> Self {
        Self {
            list_display: vec!["username".to_string(), "email".to_string(), "is_active".to_string()],
            list_filter: vec!["is_active".to_string()],
            search_fields: vec!["username".to_string(), "email".to_string()],
        }
    }
}
```

## Advanced Features

### Drag-and-Drop Reordering

Enable drag-and-drop reordering for your models with transaction-safe
operations:

```rust
use reinhardt_panel::{DragDropConfig, ReorderableModel, ReorderHandler};
use async_trait::async_trait;

// 1. Configure drag-and-drop
let config = DragDropConfig {
	order_field: "display_order".to_string(),
	enabled: true,
	custom_js: None,  // Or provide custom JavaScript for client-side handling
};

// 2. Implement ReorderableModel for your model
#[async_trait]
impl ReorderableModel for MenuItem {
	async fn get_order(&self) -> i32 {
		self.display_order
	}

	async fn set_order(&mut self, new_order: i32) {
		self.display_order = new_order;
	}

	fn get_id(&self) -> String {
		self.id.to_string()
	}
}

// 3. Create a reorder handler
let handler = ReorderHandler::new(
	config,
	connection.clone(),
	"menu_items",  // table name
	"id",          // primary key field
);

// 4. Process reorder requests
let reorder_items = vec![
	("item_1".to_string(), 0),
	("item_2".to_string(), 1),
	("item_3".to_string(), 2),
];

match handler.process_reorder(reorder_items).await {
	result if result.is_success() => {
		println!("Reordered {} items successfully", result.items_updated);
	}
	result => {
		eprintln!("Reorder failed: {}", result.message);
	}
}
```

**Key Features:**

- **Validation**: Ensures order values are sequential, non-negative, and unique
- **Transaction Safety**: All updates are executed within a database transaction
- **Error Handling**: Detailed error messages for validation and database
  failures
- **Bulk Updates**: Efficient handling of multiple items in a single transaction

**Implementation Details:**

The `ReorderHandler` validates reorder operations by:

1. Checking for negative order values
2. Detecting duplicate order values
3. Ensuring order values are sequential (0, 1, 2, ...)

All database updates are performed using SeaQuery v1.0.0-rc within a
transaction, ensuring atomicity.

For a complete implementation example, see
[`crates/panel/src/custom_views.rs`](crates/panel/src/custom_views.rs).

## Feature Flags

- `panel` (default): Web admin panel
- `cli`: Command-line interface
- `all`: All admin functionality

## Documentation

- [API Documentation](https://docs.rs/reinhardt-panel) (coming soon)
- [Panel Module Documentation](crates/panel/src/lib.rs)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
