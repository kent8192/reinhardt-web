# reinhardt-views

View classes and response handling for the Reinhardt framework, inspired by Django's class-based views and Django REST Framework.

## Overview

This crate provides the view layer for building RESTful APIs in Reinhardt. It includes base view traits, concrete view implementations for common CRUD operations, and utilities for OpenAPI schema generation and browsable API rendering.

## Features

### Implemented ✓

#### Core View Infrastructure

- **View Trait** - Base trait for all views with async dispatch support
  - Automatic OPTIONS method handling
  - Customizable allowed methods
  - Async request/response processing

#### Class-Based Views

- **ListView** - Display paginated lists of objects
  - Configurable pagination with DRF-style metadata
  - Multi-field ordering (ascending/descending)
  - Empty result set handling
  - Custom context object naming
  - Full serializer support
  - HEAD method support
- **DetailView** - Display single objects
  - Primary key (pk) lookup
  - Slug-based lookup
  - QuerySet integration
  - Custom context object naming
  - Full serializer support
  - HEAD method support

#### View Mixins

- **MultipleObjectMixin** - Common functionality for list views
  - Object retrieval
  - Ordering configuration
  - Pagination settings
  - Context data building
- **SingleObjectMixin** - Common functionality for detail views
  - Object retrieval with pk/slug
  - URL parameter configuration
  - Context data building

#### Generic API Views (Stubs)

- **ListAPIView** - List endpoint
- **CreateAPIView** - Create endpoint
- **UpdateAPIView** - Update endpoint
- **DestroyAPIView** - Delete endpoint
- **ListCreateAPIView** - Combined list/create endpoint
- **RetrieveUpdateAPIView** - Combined retrieve/update endpoint
- **RetrieveDestroyAPIView** - Combined retrieve/delete endpoint
- **RetrieveUpdateDestroyAPIView** - Combined retrieve/update/delete endpoint

#### OpenAPI Schema Generation

- **OpenAPISpec** - OpenAPI 3.0 specification structure
- **Schema** - JSON Schema definitions
- **PathItem** - API path definitions with HTTP methods
- **Operation** - HTTP operation metadata
- **Parameter** - Request parameter definitions (query, header, path, cookie)
- **Response** - Response schema definitions
- **Components** - Reusable schema components
- **SchemaGenerator** - Schema generation utilities
- **EndpointInfo** - Endpoint metadata for documentation

#### Browsable API

- Basic rendering infrastructure (minimal stub)

#### Admin Integration

- Re-exports admin views from `reinhardt-contrib`
- Admin change view support
- Integration with Django-style admin interface

#### ViewSets (from `reinhardt-viewsets`)

- **ModelViewSet** - Complete CRUD operations for models
  - List, Create, Retrieve, Update, Partial Update, Destroy actions
  - Automatic HTTP method mapping
  - Custom action support (`ActionType::Custom`)
- **ViewSet Builder** - Fluent API for ViewSet configuration
- **Action System** - Type-safe action definitions
  - Built-in actions (List, Retrieve, Create, Update, PartialUpdate, Destroy)
  - Custom action support
  - Detail/non-detail action distinction
- **Handler System** - Request routing and dispatch
- **Dependency Injection** - Field and method injection for ViewSets
- **Middleware Support** - ViewSet-specific middleware
- **Registry** - ViewSet registration and discovery

### Planned

#### Enhanced Browsable API

- HTML rendering for API exploration
- Interactive API documentation
- Form generation for POST/PUT/PATCH methods
- Syntax highlighting for responses

#### Schema Generation Enhancements

- Automatic schema inference from models
- Type-safe schema generation from Rust types
- Request/response example generation
- Schema validation utilities

#### Advanced ViewSet Features

- Nested resources handling
- Batch operations support
- Optimistic locking support

#### Template Support

- Template-based rendering
- Context processors
- Template inheritance
- Custom template loaders

## Usage

### ListView Example

```rust
use reinhardt_views::{ListView, View};
use reinhardt_serializers::JsonSerializer;
use reinhardt_orm::Model;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
    id: Option<i64>,
    title: String,
    content: String,
}

impl Model for Article {
    type PrimaryKey = i64;
    fn table_name() -> &'static str { "articles" }
    fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
}

// Create a paginated list view
let articles = vec![
    Article { id: Some(1), title: "First".into(), content: "...".into() },
    Article { id: Some(2), title: "Second".into(), content: "...".into() },
];

let view = ListView::<Article, JsonSerializer<Article>>::new()
    .with_objects(articles)
    .with_paginate_by(10)
    .with_ordering(vec!["-id".into()])
    .with_context_object_name("articles");
```

### DetailView Example

```rust
use reinhardt_views::{DetailView, View};
use reinhardt_serializers::JsonSerializer;

let article = Article {
    id: Some(1),
    title: "Hello".into(),
    content: "World".into()
};

let view = DetailView::<Article, JsonSerializer<Article>>::new()
    .with_object(article)
    .with_pk_url_kwarg("article_id")
    .with_context_object_name("article");
```

### OpenAPI Schema Generation

```rust
use reinhardt_views::{OpenAPISpec, Info, PathItem, Operation};

let spec = OpenAPISpec::new(Info::new(
    "My API".into(),
    "1.0.0".into()
));
```

## Dependencies

- `reinhardt-apps` - Request/Response types
- `reinhardt-orm` - ORM integration
- `reinhardt-serializers` - Serialization support
- `reinhardt-exception` - Error handling
- `reinhardt-contrib` - Admin views
- `async-trait` - Async trait support
- `serde` - Serialization framework
- `serde_json` - JSON serialization

## Testing

The crate includes comprehensive unit tests covering:

- Basic view functionality
- ListView pagination and ordering
- DetailView object retrieval
- Error handling
- ViewSet patterns
- API view behavior
- Admin change views
