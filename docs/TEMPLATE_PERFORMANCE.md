# Template Rendering Performance Guide

## Overview

Reinhardt uses Tera for runtime template rendering, providing a flexible and powerful template system with Jinja2-compatible syntax.

## Tera Template Engine

### Performance Characteristics

**Time Complexity**: O(n)
- n = template length and complexity
- Runtime template parsing and rendering

**Characteristics**:
- Runtime template compilation with caching
- Full Jinja2-compatible syntax
- Template inheritance and includes
- Custom filters and functions
- Context-based variable rendering

## Memory Characteristics

### Tera Runtime Rendering

**Memory usage**: Template cache + context data
- Templates cached after first compilation
- Context data serialized for rendering
- Efficient memory usage with lazy loading
- Template reuse across requests

## Use Cases for Tera

### ✅ Ideal Use Cases

**View Templates**
- HTML pages served by the application
- Admin panel templates
- API documentation pages

**Email Templates**
- Transactional emails
- Notification emails
- Marketing emails

**Dynamic Response Templates**
- Error pages (404, 500, etc.)
- Status pages
- OAuth consent pages

**User-Provided Templates**
- User-customizable email templates
- User-defined report templates
- Configurable notification formats

**Dynamic Templates**
- Templates loaded from database
- Templates generated programmatically
- A/B testing variants

**Configuration Templates**
- Config file templates
- Environment-specific templates
- Feature flag-based templates

### Advantages

- Complete flexibility
- No recompilation needed
- Database/file-based templates
- Runtime template generation
- Full Jinja2 syntax compatibility
- Template inheritance and includes
- Custom filters and functions
- Powerful template debugging

## Implementation Examples

### Tera Template Rendering

**Template File**: `templates/user.tpl`

```html
<!DOCTYPE html>
<html>
<head>
    <title>User Profile</title>
</head>
<body>
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    <p>Age: {{ age }}</p>

    {% if age >= 18 %}
        <p>Adult user</p>
    {% else %}
        <p>Minor user</p>
    {% endif %}
</body>
</html>
```

**Rust Code**:

```rust,no_run
# use reinhardt_renderers::TeraRenderer;
# use serde_json::json;
// Create renderer
let renderer = TeraRenderer::new();

// Prepare context
let context = json!({
    "name": "Alice",
    "email": "alice@example.com",
    "age": 25
});

// Render template
let html = renderer.render_template("user.tpl", &context)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Dynamic Template from Database

```rust,no_run
# use tera::{Context, Tera};
# struct Db;
# impl Db {
#     fn get_template(&self, _name: &str) -> Result<String, Box<dyn std::error::Error>> { Ok(String::new()) }
# }
# fn example(db: Db) -> Result<(), Box<dyn std::error::Error>> {
// Load template from database
let template_str = db.get_template("user_email")?;

// Create Tera instance
let mut tera = Tera::default();
tera.add_raw_template("user_email", &template_str)?;

// Prepare context
let mut context = Context::new();
context.insert("user_name", "Alice");
context.insert("activation_link", "https://example.com/activate/token");

// Render
let email_html = tera.render("user_email", &context)?;
# Ok(())
# }
```

## Template Loading Strategies

### File-based Templates

```rust,no_run
# use tera::Tera;
# use serde_json::json;
// Load all templates from directory
let tera = Tera::new("templates/**/*.tpl")?;

// Render specific template
let context = json!({"title": "Welcome"});
let html = tera.render("index.tpl", &context)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Dynamic Templates

```rust,no_run
# use tera::Tera;
# use serde_json::json;
// Add template at runtime
let mut tera = Tera::default();
tera.add_raw_template("dynamic", "Hello {{ name }}!")?;

// Render
let context = json!({"name": "World"});
let result = tera.render("dynamic", &context)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Database Templates

```rust,no_run
# use tera::Tera;
# use serde_json::json;
# struct Db;
# impl Db {
#     fn get_template(&self, _name: &str) -> Result<String, Box<dyn std::error::Error>> { Ok(String::new()) }
# }
# struct User { name: String }
# struct Notification { text: String }
# fn example(db: Db, user: User, notification: Notification) -> Result<(), Box<dyn std::error::Error>> {
// Load from database
let template_content = db.get_template("email_notification")?;

let mut tera = Tera::default();
tera.add_raw_template("email", &template_content)?;

// Render with user data
let context = json!({
    "user_name": user.name,
    "notification_text": notification.text
});
# Ok(())
# }

let email_body = tera.render("email", &context)?;
```

## Best Practices

### Do's ✅

1. **Leverage template caching**
   - Tera caches compiled templates automatically
   - Reuse Tera instances across requests
   - Load templates once at startup when possible

2. **Use structured contexts**
   - Use `serde_json::json!` macro for clean context creation
   - Prepare complex data structures before rendering
   - Keep template logic simple

3. **Profile before optimizing**
   - Measure actual performance impact
   - Consider total request time
   - Balance flexibility vs complexity

4. **Utilize template inheritance**
   - Create base templates for common layouts
   - Use `{% extends %}` and `{% block %}` effectively
   - Reduce code duplication

5. **Validate templates early**
   - Test templates during development
   - Use Tera's built-in error reporting
   - Catch syntax errors before production

### Don'ts ❌

1. **Don't recreate Tera instances**
   - Creating Tera instances is expensive
   - Reuse instances across requests
   - Consider using a global Tera instance

2. **Don't over-complicate templates**
   - Keep business logic in Rust code
   - Use templates for presentation only
   - Avoid complex calculations in templates

3. **Don't ignore template errors**
   - Handle rendering errors gracefully
   - Provide fallback content when appropriate
   - Log template errors for debugging

## Performance Optimization Tips

### Template Loading

1. **Load templates at startup**
   - Use `Tera::new("templates/**/*.tpl")` at initialization
   - Avoid loading templates on every request
   - Consider lazy loading for rarely-used templates

2. **Use template inheritance**
   - Reduce duplication with base templates
   - Share common layouts across pages
   - Improve maintainability

### Context Preparation

1. **Pre-compute values**
   - Calculate complex values in Rust
   - Format data before passing to template
   - Keep template logic simple

2. **Use efficient data structures**
   - Serialize data efficiently
   - Avoid unnecessary cloning
   - Use references where possible

### Caching Strategies

1. **Cache rendered output**
   - Cache frequently-used templates
   - Use content hash as cache key
   - Implement cache invalidation

2. **Cache template compilation**
   - Tera handles this automatically
   - Templates compiled once and reused
   - No manual intervention needed

## Summary

Reinhardt uses Tera for flexible, powerful template rendering with Jinja2-compatible syntax. Tera provides:

- **Runtime flexibility**: Load templates from files, database, or memory
- **Rich feature set**: Template inheritance, includes, filters, functions
- **Good performance**: Template caching and efficient rendering
- **Developer-friendly**: Familiar Jinja2 syntax, excellent error messages
- **Production-ready**: Battle-tested in many Rust applications

**Recommendation**: Use Tera for all template rendering needs in Reinhardt applications. It provides the best balance of flexibility, features, and performance for modern web applications.
