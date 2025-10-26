# Quickstart

Create a simple API for administrators to view and edit users and groups in the system.

## Project Setup

First, install the global tool:

```bash
cargo install reinhardt-admin
```

Create a new Reinhardt project called tutorial:

```bash
# Create RESTful API project
reinhardt-admin startproject tutorial --template-type restful
cd tutorial
```

This generates a complete project structure:

```
tutorial/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs
    ├── config.rs
    ├── apps.rs
    ├── config/
    │   ├── settings.rs
    │   ├── settings/
    │   │   ├── base.rs
    │   │   ├── local.rs
    │   │   ├── staging.rs
    │   │   └── production.rs
    │   ├── urls.rs
    │   └── apps.rs
    └── bin/
        ├── runserver.rs
        └── manage.rs
```

The generated `Cargo.toml` already includes all necessary dependencies for REST API development.

## Models

For this quickstart, we'll use Reinhardt's built-in `User` and `Group` models provided by the auth feature.

## Serializers

Define serializers for data representation. Add to `src/main.rs`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSerializer {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupSerializer {
    pub id: i64,
    pub name: String,
}
```

This example uses simple data structures. In real applications, you can implement the `Serializer` trait to add validation and data transformation logic.

## ViewSets

Use ViewSets to implement CRUD operations. Add to `src/main.rs`:

```rust
use reinhardt::prelude::*;

// UserViewSet - full CRUD operations
let user_viewset = ModelViewSet::<User, UserSerializer>::new("user");

// GroupViewSet - read-only
let group_viewset = ReadOnlyModelViewSet::<Group, GroupSerializer>::new("group");
```

`ModelViewSet` provides all standard CRUD operations (list, retrieve, create, update, delete). `ReadOnlyModelViewSet` provides only list and retrieve operations.

## Routing

First, create a users app:

```bash
cargo run --bin manage startapp users --template-type restful
```

### Define Models and Serializers

Edit `users/models.rs`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}
```

Edit `users/serializers.rs`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSerializer {
    pub id: i64,
    pub username: String,
    pub email: String,
}
```

### Create ViewSet

Edit `users/views.rs`:

```rust
use reinhardt::viewsets::ModelViewSet;
use crate::models::User;
use crate::serializers::UserSerializer;

pub struct UserViewSet;

impl UserViewSet {
    pub fn new() -> ModelViewSet<User, UserSerializer> {
        ModelViewSet::new("user")
    }
}
```

### Configure URLs

Edit `users/urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use crate::views::UserViewSet;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    // Register ViewSet - CRUD endpoints are auto-generated
    router.register_viewset("users", UserViewSet::new());

    router
}
```

### Register with Project

Edit `src/config/urls.rs`:

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder()
        .build();

    // Include users app router
    router.include_router("/api/", users::urls::url_patterns(), Some("users".to_string()));

    Arc::new(router)
}
```

Edit `src/config/apps.rs`:

```rust
use reinhardt_macros::installed_apps;

installed_apps! {
    users: "users",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

This automatically generates the following URL patterns:

- `GET /api/users/` - List users
- `POST /api/users/` - Create new user
- `GET /api/users/{id}/` - Retrieve specific user
- `PUT /api/users/{id}/` - Update user
- `PATCH /api/users/{id}/` - Partial update user
- `DELETE /api/users/{id}/` - Delete user

## Testing the API

First, start the development server:

```bash
cargo run --bin runserver
```

Test the API using curl or httpie:

```bash
# Get list of users
curl http://127.0.0.1:8000/api/users/

# Create new user
curl -X POST http://127.0.0.1:8000/api/users/ \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com"}'

# Get specific user
curl http://127.0.0.1:8000/api/users/1/

# Update user
curl -X PUT http://127.0.0.1:8000/api/users/1/ \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"newemail@example.com"}'

# Delete user
curl -X DELETE http://127.0.0.1:8000/api/users/1/
```

## Summary

In this quickstart, you learned:

1. Setting up a Reinhardt project
2. Defining serializers
3. Creating CRUD APIs with ViewSets
4. Automatic URL generation with routers

For more detailed information, see the [tutorials](1-serialization.md).
