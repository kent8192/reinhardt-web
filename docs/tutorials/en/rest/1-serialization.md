# Tutorial 1: Serialization

Learn how to serialize and deserialize data in Reinhardt.

## Setup

First, add Reinhardt to your project's `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "serializers"] }
# Or for minimal setup: reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "serializers"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
```

**Note:** Reinhardt uses feature flags to control which components are included in your build. The `standard` feature includes serializers, ORM, authentication, and other common API development tools. For more granular control, see the [Feature Flags Guide](../../../FEATURE_FLAGS.md).

## Basic Serialization with Serde

Reinhardt primarily uses [serde](https://serde.rs/) for serialization. For simple cases, derive macros are sufficient:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: i64,
    pub title: String,
    pub code: String,
    pub language: String,
}
```

## Custom Validation with Serializer Trait

For custom validation logic, implement the `Serializer` trait:

```rust
use reinhardt::prelude::*;

pub struct SnippetSerializer;

impl Serializer<Snippet> for SnippetSerializer {
    fn validate(&self, instance: &Snippet) -> ValidationResult {
        let mut errors = Vec::new();

        // Validate title length
        if instance.title.is_empty() {
            errors.push(ValidationError::new("title", "Title cannot be empty"));
        }

        // Validate code
        if instance.code.is_empty() {
            errors.push(ValidationError::new("code", "Code cannot be empty"));
        }

        // Validate language
        let valid_languages = ["python", "rust", "javascript"];
        if !valid_languages.contains(&instance.language.as_str()) {
            errors.push(ValidationError::new(
                "language",
                "Language must be python, rust, or javascript"
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

## Field-Level Validation

Use field validators for specific field constraints:

```rust
use reinhardt::prelude::*;

// String field with length constraints
let title_field = CharField::new()
    .min_length(1)
    .max_length(100);

// Integer field with range constraints
let age_field = IntegerField::new()
    .min_value(0)
    .max_value(150);

// Email validation
let email_field = EmailField::new();

// Validate values
title_field.validate(&"My Snippet".to_string()).unwrap();
age_field.validate(&25).unwrap();
email_field.validate(&"user@example.com".to_string()).unwrap();
```

## JSON Serialization

Convert data to/from JSON:

```rust
use reinhardt::prelude::*;

let serializer = JsonSerializer::<Snippet>::new();

// Serialize to JSON
let snippet = Snippet {
    id: 1,
    title: "Hello World".to_string(),
    code: "print('hello')".to_string(),
    language: "python".to_string(),
};

let json_bytes = serializer.serialize(&snippet).unwrap();
let json_str = String::from_utf8(json_bytes).unwrap();
// json_str: {"id":1,"title":"Hello World",...}

// Deserialize from JSON
let json = r#"{"id":2,"title":"Test","code":"fn main(){}","language":"rust"}"#;
let snippet = serializer.deserialize(json.as_bytes()).unwrap();
```

## Model Serializers

For database models, use `ModelSerializer` with custom validation:

```rust
use reinhardt::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub age: i32,
}

let validator = |user: &User| -> ValidationResult {
    let mut errors = Vec::new();

    if user.username.len() < 3 {
        errors.push(ValidationError::new(
            "username",
            "Username must be at least 3 characters"
        ));
    }

    if user.age < 18 {
        errors.push(ValidationError::new(
            "age",
            "User must be at least 18 years old"
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
};

let serializer = ModelSerializer::new(validator);

// Valid data
let valid_user = User {
    id: 1,
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
    age: 25,
};
assert!(serializer.validate(&valid_user).is_ok());

// Invalid data
let invalid_user = User {
    id: 2,
    username: "ab".to_string(),  // Too short
    email: "bob@example.com".to_string(),
    age: 16,  // Too young
};
assert!(serializer.validate(&invalid_user).is_err());
```

## Nested Serializers

Handle nested data structures:

```rust
use reinhardt::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: i64,
    pub title: String,
    pub author: Author,  // Nested object
}

let article = Article {
    id: 1,
    title: "Introduction to Reinhardt".to_string(),
    author: Author {
        id: 1,
        name: "Alice".to_string(),
    },
};

// Serialize with nested data
let serializer = JsonSerializer::<Article>::new();
let json = serializer.serialize(&article).unwrap();
```

## Validation Workflow

Typical validation workflow in an API view with Reinhardt:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;

#[endpoint]
async fn create_snippet(mut request: Request) -> Result<Response> {
    // 1. Parse JSON from request body (automatic deserialization)
    let body_bytes = std::mem::take(&mut request.body);
    let snippet: Snippet = serde_json::from_slice(&body_bytes)?;

    // 2. Validate the data
    let validator = SnippetSerializer;
    if let Err(errors) = validator.validate(&snippet) {
        return Response::bad_request()
            .with_json(&errors);
    }

    // 3. Save to database (using Reinhardt ORM)
    // snippet.save(&conn).await?;

    // 4. Return response with created status
    Response::new(201)
        .with_json(&snippet)
}
```

**Key Points:**
- **Automatic Parsing**: Reinhardt's `#[endpoint]` macro handles Content-Type checking
- **serde_json**: Use `serde_json::from_slice` for JSON deserialization
- **Validation**: Custom validators return `ValidationResult`
- **Response Builder**: Use `.with_json()` for JSON responses

## Summary

In this tutorial, you learned:

1. Using serde for basic serialization/deserialization
2. Implementing custom validation with the `Serializer` trait
3. Field-level validation with built-in field types
4. JSON serialization with `JsonSerializer`
5. Model serializers with custom validation logic
6. Handling nested data structures
7. Complete validation workflow in API views

Next tutorial: [Tutorial 2: Requests and Responses](2-requests-and-responses.md)
