# Part 7: Admin Customization

In this tutorial, we'll explore the Reinhardt admin interface and learn how to customize it for managing poll data.

## Activating the Admin

The Reinhardt admin is a powerful, automatically-generated interface for managing your application's data. Let's enable it for our polls application.

Add the admin dependency if not already present:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["admin"] }
```

## Creating an Admin User

First, create a superuser account that can access the admin interface using the `createsuperuser` command:

```bash
cargo run --bin manage createsuperuser
```

You'll be prompted to enter:
- Username
- Email address
- Password (entered twice for confirmation)

Example session:

```
$ cargo run --bin manage createsuperuser
Username: admin
Email address: admin@example.com
Password:
Password (again):
Superuser created successfully.
```

**Note**: The `createsuperuser` command handles password hashing automatically using secure algorithms (Argon2 by default).

## Registering Models with the Admin

To make our Poll models editable in the admin, we need to register them. Create `src/admin.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt::admin::{AdminSite, ModelAdmin};
use crate::models::{Question, Choice};

pub struct QuestionAdmin;

impl ModelAdmin for QuestionAdmin {
    type Model = Question;

    fn list_display(&self) -> Vec<&str> {
        vec!["question_text", "pub_date", "was_published_recently"]
    }

    fn list_filter(&self) -> Vec<&str> {
        vec!["pub_date"]
    }

    fn search_fields(&self) -> Vec<&str> {
        vec!["question_text"]
    }
}

pub struct ChoiceAdmin;

impl ModelAdmin for ChoiceAdmin {
    type Model = Choice;

    fn list_display(&self) -> Vec<&str> {
        vec!["choice_text", "question", "votes"]
    }

    fn list_filter(&self) -> Vec<&str> {
        vec!["question"]
    }
}

pub fn register_admin(site: &mut AdminSite) {
    site.register::<Question, QuestionAdmin>();
    site.register::<Choice, ChoiceAdmin>();
}
```

Update `src/main.rs` to include the admin:

```rust
mod admin;

use reinhardt::prelude::*;
use reinhardt::admin::AdminSite;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing setup ...

    // Setup admin
    let mut admin_site = AdminSite::new("Polls Administration");
    admin::register_admin(&mut admin_site);

    // Add admin routes
    router.mount("/admin/", admin_site.urls());

    // ... rest of setup ...
}
```

Now visit `http://127.0.0.1:8000/admin/` and log in with your superuser credentials.

## Customizing the Admin Form

Let's customize how the Question admin form looks. Update `QuestionAdmin`:

```rust
impl ModelAdmin for QuestionAdmin {
    type Model = Question;

    fn fieldsets(&self) -> Vec<(&str, Vec<&str>)> {
        vec![
            ("Question Information", vec!["question_text"]),
            ("Date Information", vec!["pub_date"]),
        ]
    }

    fn list_display(&self) -> Vec<&str> {
        vec!["question_text", "pub_date", "was_published_recently"]
    }

    fn list_filter(&self) -> Vec<&str> {
        vec!["pub_date"]
    }

    fn search_fields(&self) -> Vec<&str> {
        vec!["question_text"]
    }

    fn readonly_fields(&self) -> Vec<&str> {
        vec!["was_published_recently"]
    }
}
```

The `fieldsets` option allows you to organize fields into groups with descriptive headers.

## Adding Related Objects

We can edit Choices directly on the Question admin page using inline editing. Update `QuestionAdmin`:

```rust
use reinhardt::prelude::*;
use reinhardt::admin::{AdminSite, ModelAdmin, TabularInline, StackedInline};

pub struct ChoiceInline;

impl TabularInline for ChoiceInline {
    type Model = Choice;
    type ParentModel = Question;

    fn fields(&self) -> Vec<&str> {
        vec!["choice_text", "votes"]
    }

    fn extra(&self) -> usize {
        3  // Show 3 extra empty forms
    }
}

impl ModelAdmin for QuestionAdmin {
    type Model = Question;

    fn inlines(&self) -> Vec<Box<dyn InlineAdmin>> {
        vec![Box::new(ChoiceInline)]
    }

    // ... rest of configuration ...
}
```

Now when editing a Question, you can add and edit Choices on the same page.

## Customizing the Change List

The change list is the page that displays all objects of a given type. Let's make it more useful:

```rust
impl ModelAdmin for QuestionAdmin {
    type Model = Question;

    fn list_display(&self) -> Vec<&str> {
        // Columns to display in the list
        vec!["question_text", "pub_date", "was_published_recently"]
    }

    fn list_filter(&self) -> Vec<&str> {
        // Add filters in the sidebar
        vec!["pub_date"]
    }

    fn search_fields(&self) -> Vec<&str> {
        // Enable search on these fields
        vec!["question_text"]
    }

    fn list_per_page(&self) -> usize {
        // Number of items per page
        25
    }

    fn date_hierarchy(&self) -> Option<&str> {
        // Add drill-down navigation by date
        Some("pub_date")
    }
}
```

### Customizing Display Methods

Let's improve how `was_published_recently` is displayed:

Update `src/models.rs`:

```rust
impl Question {
    /// Check if this question was published recently (within the last day)
    pub fn was_published_recently(&self) -> bool {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);
        self.pub_date >= one_day_ago && self.pub_date <= now
    }

    /// Admin display for was_published_recently
    pub fn was_published_recently_display(&self) -> &str {
        if self.was_published_recently() {
            "Yes"
        } else {
            "No"
        }
    }
}
```

## Customizing Admin Templates

You can override admin templates to change the look and feel. Create custom templates:

Create `templates/admin/base_site.html`:

```html
{% extends "admin/base.html" %} {% block title %}{{ title }} | Polls Admin{%
endblock %} {% block branding %}
<h1 id="site-name">
  <a href="{% url 'admin:index' %}"> Polls Administration </a>
</h1>
{% endblock %} {% block nav-global %}{% endblock %}
```

This customizes the admin site header.

## Customizing the Index Page

Create `templates/admin/index.html` to customize the admin home page:

```html
{% extends "admin/index.html" %} {% block content %}
<h2>Welcome to the Polls Administration</h2>
<p>Use the sidebar to manage polls and choices.</p>

{{ block.super }} {% endblock %}
```

## Admin Actions

Admin actions allow you to perform bulk operations on selected items. Add a custom action:

```rust
impl ModelAdmin for QuestionAdmin {
    type Model = Question;

    fn actions(&self) -> Vec<Box<dyn AdminAction<Self::Model>>> {
        vec![
            Box::new(MakePublishedAction),
        ]
    }
}

pub struct MakePublishedAction;

impl AdminAction<Question> for MakePublishedAction {
    fn name(&self) -> &str {
        "make_published"
    }

    fn short_description(&self) -> &str {
        "Mark selected questions as published"
    }

    async fn perform(
        &self,
        conn: &DatabaseConnection,
        queryset: Vec<Question>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let count = queryset.len();

        for mut question in queryset {
            question.pub_date = Utc::now();
            question.save(conn).await?;
        }

        Ok(format!("Successfully updated {} question(s).", count))
    }
}
```

## Summary

In this tutorial, you learned:

- How to activate the Reinhardt admin interface
- How to create and use admin user accounts
- How to register models with the admin
- How to customize admin forms with fieldsets
- How to use inline editing for related objects
- How to customize the change list display
- How to add filters and search functionality
- How to customize admin templates
- How to create custom admin actions

The Reinhardt admin is a powerful tool for managing your application's data. With customization, it can be tailored to fit your exact needs.

## Congratulations!

You've completed the Reinhardt Basis Tutorial! You now know how to:

- Create a Reinhardt project and apps
- Define models and work with databases
- Create views and URL configurations
- Use templates to render HTML
- Process forms and handle user input
- Write tests for your application
- Serve static files (CSS, images)
- Use and customize the admin interface

## Where to Go From Here

Now that you've learned the basics, you might want to explore:

- **[REST API Tutorial](../rest/quickstart.md)** - Learn to build REST APIs with Reinhardt
- **[Feature Flags Guide](../../../FEATURE_FLAGS.md)** - Optimize your build configuration
- **Individual Crate Documentation** - Deep dive into specific components
- **Production Deployment** - Learn about deploying Reinhardt applications

Happy coding with Reinhardt!